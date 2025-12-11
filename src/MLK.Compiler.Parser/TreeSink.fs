namespace MLK.Compiler.Parser

open MLK.Compiler.Fusca
open MLK.Compiler.Text
open MLK.Compiler.Syntax

[<Interface>]
type ITreeSink =
    abstract member Token : kind : SyntaxKind -> endOffset : TextSize -> unit
    abstract member StartNode : kind : SyntaxKind -> unit
    abstract member FinishNode : unit -> unit

    abstract member EmitErrors : diagnostics : ParseDiagnostic list -> unit

type ParseEvent =
    | StartEvent of kind : SyntaxKind * forwardParent : int option
    | FinishEvent
    | TokenEvent of kind : SyntaxKind * endOffset : TextSize

module ParseEvent =
    let processEvents (sink : ITreeSink) (errors : ParseDiagnostic list) (events : ParseEvent list) : unit =
        sink.EmitErrors errors
        let events = events |> List.toArray

        let startChain idx kind fp =
            let mutable currentIdx = idx
            let mutable currentKind = kind
            let mutable currentFp = fp
            let mutable chain = []

            let mutable stop = false

            while not stop do
                chain <- currentKind :: chain
                events[currentIdx] <- StartEvent (SyntaxKind.Tombstone, None)

                match currentFp with
                | Some offset ->
                    let parentIdx = currentIdx + offset

                    match events[parentIdx] with
                    | StartEvent (parentKind, parentFp) ->
                        currentIdx <- parentIdx
                        currentKind <- parentKind
                        currentFp <- parentFp
                    | _ -> failwith "forwardParent points at non-start event"
                | None ->
                    chain |> List.iter sink.StartNode
                    stop <- true

        for i = 0 to events.Length - 1 do
            match events[i] with
            | StartEvent (kind, fp) ->
                if kind <> SyntaxKind.Tombstone then
                    startChain i kind fp
            | FinishEvent -> sink.FinishNode ()
            | TokenEvent (kind, endOffset) -> sink.Token kind endOffset

type DebugTreeSink() =
    let mutable sink : ParseEvent list = []

    interface ITreeSink with
        member this.Token (kind : SyntaxKind) (endOffset : TextSize) : unit =
            sink <- TokenEvent (kind, endOffset) :: sink

        member this.EmitErrors _diagnostics = ()
        member this.FinishNode () = sink <- FinishEvent :: sink
        member this.StartNode kind = sink <- StartEvent (kind, None) :: sink

    member this.Finish () : ParseEvent list = List.rev sink

type GenericTree =
    | Token of
        kind : SyntaxKind *
        text : string *
        range : TextRange *
        leadingTrivia : TriviaPiece list *
        trailingTrivia : TriviaPiece list
    | Node of kind : SyntaxKind * children : GenericTree list * range : TextRange

    member this.DebugPrint (source : string option) : string =
        let escapeNewlines (s : string) : string =
            s.Replace("\n", "\\n").Replace("\r", "\\r")

        let printTrivia (startOffset : TextSize) (trivia : TriviaPiece) : string =
            match source with
            | Some src ->
                let text = src.Slice(TextRange.at startOffset trivia.Length) |> escapeNewlines
                $"%A{trivia.Kind}(\"{text}\")"
            | None -> $"%A{trivia.Kind}({trivia.Length})"

        let rec toStringIndent (tree : GenericTree) (indent : int) : string =
            let indentStr = String.replicate indent "  "

            match tree with
            | Token (kind, text, range, leadingTrivia, trailingTrivia) ->
                let printTriviaList (startOffset : TextSize) (trivias : TriviaPiece list) (isLeading : bool) : string =
                    let mutable offset = startOffset
                    let triviaStrs =
                        trivias
                        |> List.map (fun t ->
                            let offset' = if isLeading then offset - t.Length else offset
                            let s = printTrivia offset' t
                            offset <- if isLeading then offset' else offset' + t.Length
                            s)

                    String.concat " ; " triviaStrs

                let leadingStr = printTriviaList range.Start leadingTrivia true
                let trailingStr = printTriviaList range.End trailingTrivia false

                $"{indentStr}%A{kind}@{range} \"{text}\" [{leadingStr}] [{trailingStr}]"
            | Node (kind, children, range) ->
                let newline =
                    if children.Length > 0 then "\n" else ""
                let childStrs =
                    children
                    |> List.map (fun c -> toStringIndent c (indent + 1))
                    |> String.concat "\n"

                $"{indentStr}%A{kind}@{range}{newline}{childStrs}"

        toStringIndent this 0

    override this.ToString(): string = this.DebugPrint(None)

type TreeFactory() =
    let mutable position : TextSize = TextSize.zero
    let mutable stack : (SyntaxKind * GenericTree list * TextSize) list = []

    member this.StartNode (kind : SyntaxKind) = stack <- (kind, [], position) :: stack

    member this.TokenWithTrivia
        (kind : SyntaxKind)
        (text : string)
        (range : TextRange)
        (leadingTrivia : TriviaPiece list)
        (trailingTrivia : TriviaPiece list)
        =
        let token = Token (kind, text, range, leadingTrivia, trailingTrivia)

        position <- range.End

        match stack with
        | (parentKind, children, startPos) :: rest -> stack <- (parentKind, children @ [ token ], startPos) :: rest
        | [] ->
            // No parent node, ignore
            ()

    member this.FinishNode () =
        match stack with
        | (kind, children, startPos) :: rest ->
            let endPos = position
            let node = Node (kind, children, TextRange.create startPos endPos)
            position <- endPos

            stack <-
                match rest with
                | (parentKind, parentChildren, parentStartPos) :: restTail ->
                    (parentKind, parentChildren @ [ node ], parentStartPos) :: restTail
                | [] -> [ (SyntaxKind.Tombstone, [ node ], TextSize.zero) ]
        | [] ->
            // No node to finish, ignore
            ()

    member this.GetTree () : GenericTree option =
        match stack with
        | [] -> None
        | [ (_, [ root ], _) ] -> Some root
        | _ -> None

type LosslessTreeSink(source : string, trivias : Trivia list) =
    let builder = TreeFactory ()
    let mutable textPos = TextSize.zero
    let mutable triviaPos = 0
    let triviaPieces : ResizeArray<TriviaPiece> = ResizeArray ()

    member this.Finish () : GenericTree option = builder.GetTree ()

    member this.EatTrivia (trailing : bool) (tokenEnd : TextSize) =
        let isDone (trivia : Trivia) =
            trailing <> trivia.Trailing
            || textPos <> trivia.Range.Start
            || (not trailing && trivia.Range.End > tokenEnd)

        for trivia in trivias |> Seq.skip triviaPos |> Seq.takeWhile (not << isDone) do
            let triviaLen = TextRange.length trivia.Range
            let piece = TriviaPiece.create trivia.Kind triviaLen
            triviaPieces.Add piece
            textPos <- textPos + triviaLen
            triviaPos <- triviaPos + 1


    member this.DoToken (kind : SyntaxKind) (tokenEnd : TextSize) =
        // let tokenStart = textPos

        this.EatTrivia false tokenEnd

        let tokenStart = textPos

        let trailingStart = triviaPieces.Count

        textPos <- tokenEnd

        this.EatTrivia true tokenEnd

        let tokenRange = TextRange.create tokenStart tokenEnd

        let text = source.Slice tokenRange
        let leading = triviaPieces |> Seq.take trailingStart |> Seq.toList
        let trailing = triviaPieces |> Seq.skip trailingStart |> Seq.toList

        builder.TokenWithTrivia kind text tokenRange leading trailing
        triviaPieces.Clear ()

    interface ITreeSink with
        member this.Token (kind : SyntaxKind) (endOffset : TextSize) : unit =
            this.DoToken kind endOffset

        member this.EmitErrors _diagnostics = ()

        member this.FinishNode () = builder.FinishNode ()

        member this.StartNode kind = builder.StartNode kind
