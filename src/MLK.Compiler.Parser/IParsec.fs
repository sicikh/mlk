namespace MLK.Compiler.Parser.IParsec

open MLK.Compiler.Text
open MLK.Compiler.Syntax
open MLK.Compiler.Parser
open Stdx

type SignificantTokens = Set<SyntaxKind>

type Marker =
    {
        Index : int
        mutable ForwardParent : int option
    }

type CompletedMarker = { StartIndex : int }

type Indent =
    | Indent of int

    static member Inf = Indent System.Int32.MaxValue
    static member Zero = Indent 0

type IndentRel =
    | Eq
    | Any
    | Const of indent : Indent
    | Ge
    | Gt

type ParseState =
    {
        Source : TokenSource
        Diagnostics : ParseDiagnostic list
        Events : ResizeArray<ParseEvent>
        EventCount : int
        MinIndent : Indent
        MaxIndent : Indent
        AbsMode : bool
        IndentRel : IndentRel
    }

    static member WithSource (source : TokenSource) =
        {
            Source = source
            Diagnostics = []
            Events = ResizeArray ()
            EventCount = 0
            MinIndent = Indent.Zero
            MaxIndent = Indent.Inf
            AbsMode = true
            IndentRel = Ge
        }

    member this.AddDiag (diag : ParseDiagnostic) : ParseState =
        { this with
            Diagnostics = diag :: this.Diagnostics
        }

    member this.PushToken (kind : SyntaxKind) (offset : TextSize) : ParseState =
        let idx = this.EventCount

        if idx = this.Events.Count then
            this.Events.Add (TokenEvent (kind, offset))
        else
            this.Events[idx] <- TokenEvent (kind, offset)

        { this with EventCount = idx + 1 }

    member this.PushEvent (event : ParseEvent) : ParseState =
        let idx = this.EventCount

        if idx = this.Events.Count then
            this.Events.Add event
        else
            this.Events[idx] <- event

        { this with EventCount = idx + 1 }

    member this.PushEvents (events : ParseEvent list) : ParseState =
        List.fold _.PushEvent this (List.rev events)

    member this.SameCurrent (other : ParseState) : bool =
        this.Source.HasSameCurrentToken other.Source

    member this.Uncons : Token * ParseState =
        let head, tail = this.Source.Uncons
        let state' = { this with Source = tail }
        head, state'

    member this.WithSource (source : TokenSource) : ParseState = { this with Source = source }

    member this.StartNode () =
        let idx = this.EventCount

        let start = StartEvent (SyntaxKind.Tombstone, None)

        if idx = this.Events.Count then
            this.Events.Add start
        else
            this.Events[idx] <- start

        { this with EventCount = idx + 1 }, { Index = idx ; ForwardParent = None }

    member this.FinishNode (marker : Marker) (kind : SyntaxKind) : ParseState * CompletedMarker =
        this.Events[marker.Index] <- StartEvent (kind, marker.ForwardParent)
        this.PushEvent FinishEvent, { StartIndex = marker.Index }

    member this.ChangeKind (marker : Marker) (kind : SyntaxKind) : ParseState =
        this.Events[marker.Index] <-
            match this.Events[marker.Index] with
            | StartEvent (_, fp) -> StartEvent (kind, fp)
            | _ -> failwith "expected start event"

        this

    member this.Precede (completed : CompletedMarker) : ParseState * Marker =
        let s1, marker = this.StartNode ()

        match s1.Events[completed.StartIndex] with
        | StartEvent (kind, _) ->
            let offset = marker.Index - completed.StartIndex
            assert (offset > 0)
            s1.Events[completed.StartIndex] <- StartEvent (kind, Some offset)
            s1, marker
        | _ -> failwith "expected start event"

    member this.Finish () : ParseEvent list * Trivia list * ParseDiagnostic list =
        this.Events |> Seq.take this.EventCount |> Seq.toList, this.Source.Trivias, List.rev this.Diagnostics

type ParseCtx =
    {
        SignificantTokens : SignificantTokens
    }

    static member Zero = { SignificantTokens = Set.empty }

    member this.UnionTokens (sigs : SignificantTokens) : ParseCtx =
        { this with
            SignificantTokens = Set.union this.SignificantTokens sigs
        }

    member this.WithTokens (sigs : SignificantTokens) : ParseCtx = { this with SignificantTokens = sigs }

type ErrorReason =
    {
        Reason : string option
        Expected : SignificantTokens
        Position : TextSize
    }

type PResult<'a> =
    | Success of res : 'a * state : ParseState
    | Failure of con : bool

type RunParser<'a> = ParseState -> ParseCtx -> PResult<'a>

type Parser<'a>(f : RunParser<'a>, significantTokens : SignificantTokens, [<Struct>] ?isOpt : bool) =
    let mutable f = f
    let mutable significantTokens = significantTokens
    let isOpt = defaultValueArg isOpt false

    member this.Run
        with get () = f
        and set v = f <- v

    member this.SignificantTokens
        with get () = significantTokens
        and set v = significantTokens <- v

    member this.IsOpt = isOpt

    new() = Parser<'a> ((fun _ _ -> failwith "called dummy parser"), Set.empty)

type 'a parser = Parser<'a>

[<AutoOpen>]
module Combinators =
    let runParser (p : 'a parser) (inp : TokenSource) : PResult<'a> =
        p.Run (ParseState.WithSource inp) ParseCtx.Zero

    let preturn (x : 'a) : 'a parser =
        let f state _ = Success (x, state)

        Parser (f, Set.empty, true)

    let unionCtx (p : 'a parser) (ctx : ParseCtx) : ParseCtx = ctx.UnionTokens p.SignificantTokens

    let pushEvent (event : ParseEvent) (state : ParseState) : ParseState = state.PushEvent event

    let pipe2 (p : 'a parser) (q : 'b parser) (f : 'a -> 'b -> 'c) : 'c parser =
        let f state ctx =
            // TODO: how to union ctx?
            match p.Run state (unionCtx q ctx) with
            | Success (r1, state1) ->
                match q.Run state1 ctx with
                | Success (r2, state2) -> Success (f r1 r2, state2)
                | Failure con -> Failure (con || not (state.SameCurrent state1))
            | Failure con -> Failure con

        let significantTokens =
            if p.IsOpt then
                Set.union p.SignificantTokens q.SignificantTokens
            else
                p.SignificantTokens

        Parser (f, significantTokens, p.IsOpt && q.IsOpt)

    let pipe3 (p1 : 'a parser) (p2 : 'b parser) (p3 : 'c parser) (f : 'a -> 'b -> 'c -> 'd) : 'd parser =
        // TODO: optimize later
        pipe2 p1 (pipe2 p2 p3 (fun x y -> (x, y))) (fun x (y, z) -> f x y z)

    let inline (^.>>.) (p : 'a parser) (q : 'b parser) : ('a * 'b) parser = pipe2 p q (fun x y -> (x, y))

    let inline (^.>>) (p : 'a parser) (q : 'b parser) : 'a parser = pipe2 p q (fun x _ -> x)

    let inline (^>>.) (p : 'a parser) (q : 'b parser) : 'b parser = pipe2 p q (fun _ y -> y)

    let inline (^>>) (p : 'a parser) (q : 'b parser) : unit parser = pipe2 p q (fun _ _ -> ())

    let (<|>) (p : 'a parser) (q : 'a parser) : 'a parser =
        let f state ctx =
            match p.Run state ctx with
            | Success _ as r -> r
            | Failure true as r -> r
            | Failure false -> q.Run state ctx

        let significantTokens = Set.union p.SignificantTokens q.SignificantTokens

        Parser (f, significantTokens, p.IsOpt || q.IsOpt)

    let map (f : 'a -> 'b) (p : 'a parser) : 'b parser =
        let f' state ctx =
            match p.Run state ctx with
            | Success (r, state1) -> Success (f r, state1)
            | Failure con -> Failure con

        Parser (f', p.SignificantTokens, p.IsOpt)

    let inline (|>>) (p : 'a parser) (f : 'a -> 'b) : 'b parser = map f p

    // TODO: add parameter to get significant tokens as isOpt from f's result?
    let bind (f : 'a -> 'b parser) (p : 'a parser) : 'b parser =
        let f' state ctx =
            match p.Run state ctx with
            | Success (r, state1) ->
                let q = f r

                match q.Run state1 ctx with
                | Success _ as r -> r
                | Failure con -> Failure (con || not (state.SameCurrent state1))
            | Failure con -> Failure con

        Parser (f', p.SignificantTokens, p.IsOpt)

    let inline (>>=) (p : 'a parser) (f : 'a -> 'b parser) : 'b parser = bind f p

    let opt (p : 'a parser) : 'a option parser =
        let f state ctx =
            match p.Run state ctx with
            | Success (r, state1) -> Success (Some r, state1)
            | Failure false -> Success (None, state)
            | Failure true -> Failure true

        Parser (f, p.SignificantTokens, true)

    let many (p : 'a parser) : 'a list parser =
        let rec aux acc state ctx =
            match p.Run state (unionCtx p ctx) with
            | Success (r, state1) -> aux (r :: acc) state1 ctx
            | Failure false -> Success (List.rev acc, state)
            | Failure true -> Failure true

        Parser (aux [], p.SignificantTokens, true)

    let manyU (p : unit parser) : unit parser =
        let rec aux state ctx =
            match p.Run state (unionCtx p ctx) with
            | Success (_, state1) -> aux state1 ctx
            | Failure false -> Success ((), state)
            | Failure true -> Failure true

        Parser (aux, p.SignificantTokens, true)

    let many1 (p : 'a parser) : 'a list parser = pipe2 p (many p) (fun x xs -> x :: xs)

    let many1U (p : unit parser) : unit parser = pipe2 p (manyU p) (fun () () -> ())

    let sepBy1 (p : 'a parser) (sep : 'b parser) : 'a list parser =
        let sepThenP = sep ^>>. p
        pipe2 p (many sepThenP) (fun x xs -> x :: xs)

    let sepBy (p : 'a parser) (sep : 'b parser) : 'a list parser = sepBy1 p sep <|> preturn []

    let sepBy1Trailing (p : 'a parser) (sep : 'b parser) : 'a list parser =
        let sepThenP = sep ^>>. p
        pipe3 p (many sepThenP) (opt sep) (fun x xs _ -> x :: xs)

    let sepByTrailing (p : 'a parser) (sep : 'b parser) : 'a list parser = sepBy1Trailing p sep <|> preturn []

    let between (pOpen : 'a parser) (pClose : 'b parser) (p : 'c parser) : 'c parser =
        pipe3 pOpen p pClose (fun _ x _ -> x)

    let mkRec (mkParser : 'a parser -> 'a parser) : 'a parser =
        let p = Parser ()
        p.SignificantTokens <- (mkParser <| Parser ()).SignificantTokens
        let p' = mkParser p
        p.Run <- p'.Run
        // TODO? do we need this? should we make third pass?
        p.SignificantTokens <- p'.SignificantTokens
        p

    let node (kind : SyntaxKind) (p : 'a parser) : CompletedMarker parser =
        let f (state : ParseState) ctx =
            let s1, marker = state.StartNode ()

            match p.Run s1 ctx with
            | Success (_, state1) ->
                let s2, completed = state1.FinishNode marker kind
                Success (completed, s2)
            | Failure con -> Failure con

        Parser (f, p.SignificantTokens, p.IsOpt)

    let skipInsignificantInto
        (errorKind : SyntaxKind)
        (diagBuilder : TokenSource -> TextRange -> ParseDiagnostic)
        : unit parser
        =
        let f (state : ParseState) (ctx : ParseCtx) =
            let rec loop (inp : TokenSource) events =
                match inp.Head.Kind with
                | SyntaxKind.Eof -> inp, events
                | tk when Set.contains tk ctx.SignificantTokens -> inp, events
                | tk ->
                    let event = TokenEvent (tk, inp.Head.Range.End)
                    loop inp.Tail (event :: events)

            let inp1, errEvents = loop state.Source []

            let state1 =
                let diagRange =
                    match errEvents with
                    | TokenEvent (endOffset = endOffset) :: _ -> TextRange.create state.Source.Position endOffset
                    | [] -> state.Source.Head.Range
                    | _ -> failwith "unreachable"

                let diag = diagBuilder state.Source diagRange

                state
                    .WithSource(inp1)
                    .AddDiag(diag)
                    // .PushEvent(StartEvent (errorKind, None))
                    .PushEvents(errEvents)
                    // .PushEvent
                    // FinishEvent

            Success ((), state1)

        Parser (f, Set.empty, true)

    let recoverInto
        (errorNode : SyntaxKind)
        (diagBuilder : TokenSource -> TextRange -> ParseDiagnostic)
        (p : CompletedMarker parser)
        : CompletedMarker parser
        =
        p <|> (node errorNode <| skipInsignificantInto errorNode diagBuilder)

    let tryOrDiag (_diagBuilder : TokenSource -> TextRange -> ParseDiagnostic) (p : 'a parser) : 'a option parser =
        // TODO: add diag to state
        opt p

    let checkIndentation (state : ParseState) (token : Token) : ParseState option =
        let (Indent lo) = state.MinIndent
        let (Indent hi) = state.MaxIndent
        let i = int (token.StartLineCol.Column + 1u)
        let rel = if state.AbsMode then Eq else state.IndentRel

        let updateState lo hi =
            { state with
                MinIndent = Indent lo
                MaxIndent = Indent hi
                AbsMode = false
            }

        match rel with
        | Any -> Some <| updateState lo hi
        | Const (Indent c) when c = i -> Some <| updateState lo hi
        | Eq when lo <= i && i <= hi -> Some <| updateState i i
        | Gt when lo < i -> Some <| updateState lo (min (i - 1) hi)
        | Ge when lo <= i -> Some <| updateState lo (min i hi)
        | _ -> None

    let testIndent (value : 'a) : 'a parser =
        let f (state : ParseState) _ctx =
            match state.Source.Head with
            | t ->
                match checkIndentation state t with
                | None -> Failure false
                // TODO: to thread or not to thread? that is the question
                | Some _state1 -> Success (value, state)

        Parser (f, Set.empty, true)

    let private pTokenS' isSignificant kind : Token parser =
        let f (state : ParseState) _ctx =
            match state.Uncons with
            | t, state1 when t.Kind = kind ->
                match checkIndentation state1 t with
                | None -> Failure false
                | Some state1 ->
                    let tokenEndOffset = t.Range.End
                    Success (t, state1.PushEvent (TokenEvent (kind, tokenEndOffset)))
            | _ -> Failure false

        Parser (f, if isSignificant then Set.singleton kind else Set.empty)

    let pToken kind = pTokenS' false kind
    let pTokenS kind = pTokenS' true kind

    let pChooseToken (f : Token -> 'a option) : 'a parser =
        let f' (state : ParseState) _ctx =
            match state.Uncons with
            | t, state1 ->
                match f t with
                | Some r ->
                    match checkIndentation state1 t with
                    | None -> Failure false
                    | Some state1 ->
                        let tokenEndOffset = t.Range.End
                        Success (r, state1.PushEvent (TokenEvent (t.Kind, tokenEndOffset)))
                | None -> Failure false

        Parser (f', Set.empty)

    let pCheckToken (f : Token -> bool) : Token parser =
        let f' (state : ParseState) _ctx =
            match state.Uncons with
            | t, state1 when f t ->
                match checkIndentation state1 t with
                | None -> Failure false
                | Some _ ->
                    Success (t, state)
            | _ -> Failure false

        Parser (f', Set.empty)

    let pPratt'
        (pTerm : CompletedMarker parser)
        (pPrefixOp : (int * SyntaxKind) parser)
        (pInfixOp : (int * int * SyntaxKind) parser)
        (minbp : int)
        : Parser<CompletedMarker>
        =
        let sigs =
            Set.unionMany
                [
                    pTerm.SignificantTokens
                    pPrefixOp.SignificantTokens
                    pInfixOp.SignificantTokens
                ]
        let u (ctx : ParseCtx) = ctx.UnionTokens sigs

        let rec pExpr minbp state ctx =
            match parseHead state ctx with
            | Failure con -> Failure con
            | Success (head, state1) -> parseTail head minbp state1 ctx

        and parseHead (state : ParseState) ctx =
            let s1, marker = state.StartNode ()

            match pPrefixOp.Run s1 (u ctx) with
            | Success ((rbp, kind), state1) ->
                match pExpr rbp state1 (u ctx) with
                | Success (_completed, state2) ->
                    let s3 = state2.ChangeKind marker kind
                    let s4 = s3.PushEvent FinishEvent
                    let completedMarker = { StartIndex = marker.Index }
                    // let state3, marker = state2.Precede completed
                    // let state4, completedNode = state3.FinishNode marker kind
                    Success (completedMarker, s4)
                | Failure con -> Failure con
            | Failure false -> pTerm.Run state ctx
            | Failure true -> Failure true

        and parseTail left minbp state ctx =
            match pInfixOp.Run state (u ctx) with
            | Success ((lbp, rbp, kind), stateOp) when lbp >= minbp ->
                match pExpr rbp stateOp (u ctx) with
                | Success (_right, stateRhs) ->
                    let stateParent, wrap = stateRhs.Precede left
                    let stateDone, combined = stateParent.FinishNode wrap kind
                    parseTail combined minbp stateDone ctx
                | Failure con -> Failure con
            | _ -> Success (left, state)

        Parser (pExpr minbp, pTerm.SignificantTokens, false)

    let localTokenMode (fRel : IndentRel -> IndentRel) (p : 'a parser) : 'a parser =
        let f state ctx =
            match
                p.Run
                    { state with
                        IndentRel = fRel state.IndentRel
                    }
                    ctx
            with
            | Success (r, state1) ->
                Success (
                    r,
                    { state1 with
                        IndentRel = state.IndentRel
                    }
                )
            | Failure con -> Failure con

        Parser (f, p.SignificantTokens, p.IsOpt)

    let localIndentation (rel : IndentRel) (p : 'a parser) : 'a parser =
        let aux fLo fHi fHi' state ctx =
            match
                p.Run
                    { state with
                        MinIndent = fLo state.MinIndent
                        MaxIndent = fHi state.MaxIndent
                    }
                    ctx
            with
            | Success (r, state1) ->
                Success (
                    r,
                    { state1 with
                        MinIndent = state.MinIndent
                        MaxIndent = fHi' state.MaxIndent state1.MaxIndent
                    }
                )
            | Failure con -> Failure con

        let f =
            match rel with
            // TODO: ??? why on equality nothing happens?
            | Eq -> p.Run
            | Any -> aux (konst (Indent 0)) (konst Indent.Inf) konst
            | Const indent -> aux (konst indent) (konst indent) konst
            | Ge -> aux id (konst Indent.Inf) (flip konst)
            | Gt ->
                let f hi hi' =
                    if hi' = Indent.Inf || hi < hi' then
                        hi
                    elif hi' > Indent.Zero then
                        let (Indent n) = hi'
                        Indent (n - 1)
                    else
                        failwith "assertion failed: hi' > 0"

                aux (fun (Indent n) -> Indent (n + 1)) (konst Indent.Inf) f

        Parser (f, p.SignificantTokens, p.IsOpt)

    let absoluteIndentation (p : 'a parser) : 'a parser =
        let f state ctx =
            match p.Run { state with AbsMode = true } ctx with
            | Success (r, state1) ->
                Success (
                    r,
                    { state1 with
                        AbsMode = state.AbsMode && state1.AbsMode
                    }
                )
            | Failure con -> Failure con

        Parser (f, p.SignificantTokens, p.IsOpt)

    let ignoreAbsoluteIndentation (p : 'a parser) : 'a parser =
        let f state ctx =
            match p.Run { state with AbsMode = false } ctx with
            | Success (r, state1) -> Success (r, { state1 with AbsMode = state.AbsMode })
            | Failure con -> Failure con

        Parser (f, p.SignificantTokens, p.IsOpt)

    let localAbsoluteIndentation (p : 'a parser) : 'a parser =
        let f state ctx =
            match p.Run { state with AbsMode = true } ctx with
            | Success (r, state1) -> Success (r, { state1 with AbsMode = state.AbsMode })
            | Failure con -> Failure con

        Parser (f, p.SignificantTokens, p.IsOpt)

    let withSignificant (sigs : SignificantTokens) (p : 'a parser) : 'a parser =
        Parser (p.Run, Set.union p.SignificantTokens sigs, p.IsOpt)

    let mutable private traceMsgIndent = 0

    let trace (name : string) (p : 'a parser) : 'a parser =
        let f (state : ParseState) (ctx : ParseCtx) =
            let indentStr = String.replicate traceMsgIndent "  "
            printfn $"{indentStr}Entering parser {name}. Current token: {state.Source.Head}"
            traceMsgIndent <- traceMsgIndent + 1

            match p.Run state ctx with
            | Success (_, state1) as r ->
                printfn $"{indentStr}Exiting parser {name}. Next token: {state1.Source.Head}"
                traceMsgIndent <- traceMsgIndent - 1
                r
            | Failure con ->
                printfn $"{indentStr}Failing parser {name}, consumed = {con}. Current token: {state.Source.Head}"
                traceMsgIndent <- traceMsgIndent - 1
                Failure con

        Parser (f, p.SignificantTokens, p.IsOpt)
