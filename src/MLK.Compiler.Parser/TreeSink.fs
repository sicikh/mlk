namespace MLK.Compiler.Parser

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
        member this.Token(kind: SyntaxKind) (endOffset: TextSize) : unit =
            sink <- TokenEvent(kind, endOffset) :: sink

        member this.EmitErrors(_diagnostics) = ()
        member this.FinishNode() =
            sink <- FinishEvent :: sink
        member this.StartNode(kind) =
            sink <- StartEvent(kind, None) :: sink

    member this.Finish() : ParseEvent list =
        List.rev sink
