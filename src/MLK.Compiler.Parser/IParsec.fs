namespace MLK.Compiler.Parser.IParsec

open MLK.Compiler.Text
open MLK.Compiler.Syntax
open MLK.Compiler.Parser
open MLK.Compiler.Parser.TokenSource

type SignificantTokens = Set<SyntaxKind>

type ParseState =
    {
        Diagnostics : ParseDiagnostic list
    }

    member this.AddDiag (diag : ParseDiagnostic) : ParseState =
        { this with
            Diagnostics = diag :: this.Diagnostics
        }

type ErrorReason =
    {
        Reason : string option
        Expected : SignificantTokens
        Position : TextSize
    }

type PResult<'a> =
    | Success of res : 'a * inp : TokenSource * events : ParseEvent list * state : ParseState
    | Failure of err : ErrorReason * con : bool * state : ParseState

type RunParser<'a> = TokenSource -> SignificantTokens -> ParseEvent list -> ParseState -> PResult<'a>

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

    new() = Parser<'a> ((fun _ _ _ -> failwith "called dummy parser"), Set.empty)

type 'a parser = Parser<'a>

type UParser = Parser<unit>
type uparser = UParser

[<AutoOpen>]
module Combinators =
    let runParser (p : 'a parser) (inp : TokenSource) : PResult<'a> =
        let initialState = { Diagnostics = [] }

        p.Run inp Set.empty [] initialState

    let preturn (x : 'a) : 'a parser =
        let f (inp : TokenSource) (_ : SignificantTokens) (events : ParseEvent list) (state : ParseState) =
            Success (x, inp, events, state)

        Parser (f, Set.empty, true)

    let pipe2 (p : 'a parser) (q : 'b parser) (f : 'a -> 'b -> 'c) : 'c parser =
        let f (inp : TokenSource) (sigs : SignificantTokens) (events : ParseEvent list) (state : ParseState) =
            match p.Run inp (Set.union sigs q.SignificantTokens) events state with
            | Success (r1, inp', events', state') ->
                match q.Run inp' sigs events' state' with
                | Success (r2, inp'', events'', state'') -> Success (f r1 r2, inp'', events'', state'')
                | Failure (err, con, state'') -> Failure (err, con || not (inp.HasSameCurrentToken inp'), state'')
            | Failure (err, con, state') -> Failure (err, con, state')

        let significantTokens =
            if p.IsOpt then
                Set.union p.SignificantTokens q.SignificantTokens
            else
                p.SignificantTokens

        Parser (f, significantTokens, p.IsOpt && q.IsOpt)

    let pipe3 (p1 : 'a parser) (p2 : 'b parser) (p3 : 'c parser) (f : 'a -> 'b -> 'c -> 'd) : 'd parser =
        let f (inp : TokenSource) (sigs : SignificantTokens) (events : ParseEvent list) (state : ParseState) =
            match p1.Run inp (Set.union sigs (Set.union p2.SignificantTokens p3.SignificantTokens)) events state with
            | Success (r1, inp', events', state') ->
                match p2.Run inp' (Set.union sigs p3.SignificantTokens) events' state' with
                | Success (r2, inp'', events'', state'') ->
                    match p3.Run inp'' sigs events'' state'' with
                    | Success (r3, inp''', events''', state''') -> Success (f r1 r2 r3, inp''', events''', state''')
                    | Failure (err, con, state''') ->
                        Failure (err, con || not (inp.HasSameCurrentToken inp''), state''')
                | Failure (err, con, state'') -> Failure (err, con || not (inp.HasSameCurrentToken inp'), state'')
            | Failure (err, con, state') -> Failure (err, con, state')

        let significantTokens =
            if p1.IsOpt then
                let p2SignificantTokens =
                    if p2.IsOpt then
                        Set.union p2.SignificantTokens p3.SignificantTokens
                    else
                        p2.SignificantTokens

                Set.union p1.SignificantTokens p2SignificantTokens
            else
                p1.SignificantTokens

        Parser (f, significantTokens, p1.IsOpt && p2.IsOpt && p3.IsOpt)

    let inline (^.>>.) (p : 'a parser) (q : 'b parser) : ('a * 'b) parser = pipe2 p q (fun x y -> (x, y))

    let inline (^.>>) (p : 'a parser) (q : 'b parser) : 'a parser = pipe2 p q (fun x _ -> x)

    let inline (^>>.) (p : 'a parser) (q : 'b parser) : 'b parser = pipe2 p q (fun _ y -> y)

    let inline (^>>) (p : 'a parser) (q : 'b parser) : uparser = pipe2 p q (fun _ _ -> ())

    let (<|>) (p : 'a parser) (q : 'a parser) : 'a parser =
        let f (inp : TokenSource) (sigs : SignificantTokens) (events : ParseEvent list) (state : ParseState) =
            match p.Run inp sigs events state with
            | Success _ as r -> r
            | Failure (con = true) as r -> r
            | Failure (con = false) -> q.Run inp sigs events state

        let significantTokens = Set.union p.SignificantTokens q.SignificantTokens

        Parser (f, significantTokens, p.IsOpt || q.IsOpt)

    let map (f : 'a -> 'b) (p : 'a parser) : 'b parser =
        let f' (inp : TokenSource) (sigs : SignificantTokens) (events : ParseEvent list) (state : ParseState) =
            match p.Run inp sigs events state with
            | Success (r, inp', events', state') -> Success (f r, inp', events', state')
            | Failure (err, con, state') -> Failure (err, con, state')

        Parser (f', p.SignificantTokens, p.IsOpt)

    let inline (|>>) (p : 'a parser) (f : 'a -> 'b) : 'b parser = map f p

    // TODO: add parameter to get significant tokens as isOpt from f's result?
    let bind (f : 'a -> 'b parser) (p : 'a parser) : 'b parser =
        let f' (inp : TokenSource) (sigs : SignificantTokens) (events : ParseEvent list) (state : ParseState) =
            match p.Run inp sigs events state with
            | Success (r, inp', events', state') ->
                let q = f r

                match q.Run inp' sigs events' state' with
                | Success _ as r -> r
                | Failure (err, con, state'') -> Failure (err, con || not (inp.HasSameCurrentToken inp'), state'')
            | Failure (err, con, state') -> Failure (err, con, state')

        Parser (f', p.SignificantTokens, p.IsOpt)

    let inline (>>=) (p : 'a parser) (f : 'a -> 'b parser) : 'b parser = bind f p

    let opt (p : 'a parser) : 'a option parser =
        let f (inp : TokenSource) (sigs : SignificantTokens) (events : ParseEvent list) (state : ParseState) =
            match p.Run inp sigs events state with
            | Success (r, inp', events', state') -> Success (Some r, inp', events', state')
            | Failure (con = false) -> Success (None, inp, events, state)
            | Failure (err, con, state) -> Failure (err, con, state)

        Parser (f, p.SignificantTokens, true)

    let many (p : 'a parser) : 'a list parser =
        let rec pfold acc inp sigs events state =
            match p.Run inp (Set.union sigs p.SignificantTokens) events state with
            | Success (r, inp', events', state') -> pfold (r :: acc) inp' sigs events' state'
            | Failure (con = false) -> Success (List.rev acc, inp, events, state)
            | Failure (err, con, state) -> Failure (err, con, state)

        let f = pfold []

        Parser (f, p.SignificantTokens, true)

    let many1 (p : 'a parser) : 'a list parser = pipe2 p (many p) (fun x xs -> x :: xs)

    let sepBy1 (p : 'a parser) (sep : 'b parser) : 'a list parser =
        let sepThenP = sep ^>>. p
        pipe2 p (many sepThenP) (fun x xs -> x :: xs)

    let sepBy (p : 'a parser) (sep : 'b parser) : 'a list parser = sepBy1 p sep <|> preturn []

    let sepBy1Trailing (p : 'a parser) (sep : 'b parser) : 'a list parser =
        let sepThenP = sep ^>>. p
        pipe3 p (many sepThenP) (opt sep) (fun x xs _ -> x :: xs)

    let sepByTrailing (p : 'a parser) (sep : 'b parser) : 'a list parser = sepBy1Trailing p sep <|> preturn []

    let between (popen : 'a parser) (pclose : 'b parser) (p : 'c parser) : 'c parser =
        pipe3 popen p pclose (fun _ x _ -> x)

    let mkRec (mkParser : 'a parser -> 'a parser) : 'a parser =
        let p = Parser ()
        p.SignificantTokens <- (mkParser <| Parser ()).SignificantTokens
        let p' = mkParser p
        p.Run <- p'.Run
        // TODO? do we need this? should we make third pass?
        p.SignificantTokens <- p'.SignificantTokens
        p

    let node (kind : SyntaxKind) (p : 'a parser) : 'a parser =
        let f (inp : TokenSource) (sigs : SignificantTokens) (events : ParseEvent list) (state : ParseState) =
            match p.Run inp sigs (StartEvent kind :: events) state with
            | Success (r, inp', events', state') -> Success (r, inp', FinishEvent :: events', state')
            | Failure (err, con, state') -> Failure (err, con, state')

        Parser (f, p.SignificantTokens, p.IsOpt)

    let token (kind : SyntaxKind) (p : 'a parser) : 'a parser =
        let f (inp : TokenSource) (sigs : SignificantTokens) (events : ParseEvent list) (state : ParseState) =
            let tokenEndOffset = inp.Head.Range.End

            match p.Run inp sigs events state with
            | Success (r, inp', events', state') ->
                Success (r, inp', TokenEvent (kind, tokenEndOffset) :: events', state')
            | Failure (err, con, state') -> Failure (err, con, state')

        Parser (f, p.SignificantTokens, p.IsOpt)

    let private pTokenS' isSignificant kind : Token parser =
        let f (inp : TokenSource) (sigs : SignificantTokens) (events : ParseEvent list) (state : ParseState) =
            if inp.Head.Kind = kind then
                let tokenEndOffset = inp.Head.Range.End
                let events' =
                    if isSignificant then
                        TokenEvent (kind, tokenEndOffset) :: events
                    else
                        events

                Success (inp.Head, inp.Tail, events', state)
            else
                let expected =
                    if isSignificant then
                        Set.add kind sigs
                    else
                        sigs

                let err =
                    {
                        Reason = None
                        Expected = expected
                        Position = inp.Head.Range.Start
                    }

                Failure (err, false, state)

        Parser (f, if isSignificant then Set.singleton kind else Set.empty)

    let pToken kind = token kind (pTokenS' false kind)
    let pTokenS kind = token kind (pTokenS' true kind)

    let skipInsignificant : uparser =
        let f (inp : TokenSource) (sigs : SignificantTokens) (events : ParseEvent list) (state : ParseState) =
            let rec loop (inp : TokenSource) =
                match inp.Head.Kind with
                | SyntaxKind.Eof -> inp
                | _ when Set.contains inp.Head.Kind sigs -> inp
                | _ ->
                    // TODO: emit error token
                    loop inp.Tail

            Success ((), loop inp, events, state)

        Parser (f, Set.empty, true)

    let expect p =
        pipe2 skipInsignificant p (fun _ x -> x)

    // TODO: rewrite to use events, not directly build AST nodes
    let pPratt'
        (pTerm : 'a parser)
        (pPrefixOp : (int * ('a -> 'a)) parser)
        (pInfixOp : (int * ('a -> 'a -> 'a)) parser)
        (pPostfixOp : (int * ('a -> 'a)) parser)
        (minbp : int)
        : 'a parser
        =
        failwith "todo"

    let pPratt
        (pTerm : 'a parser)
        (pPrefixOp : (int * ('a -> 'a)) parser)
        (pInfixOp : (int * ('a -> 'a -> 'a)) parser)
        (pPostfixOp : (int * ('a -> 'a)) parser)
        : 'a parser
        =
        pPratt' pTerm pPrefixOp pInfixOp pPostfixOp 0


    let mutable private traceMsgIndent = 0

    let trace (name : string) (p : 'a parser) : 'a parser =
        let f (inp : TokenSource) (sigs : SignificantTokens) (events : ParseEvent list) (state : ParseState) =
            let indentStr = String.replicate traceMsgIndent "  "
            printfn "%sEntering parser \"%s\". Current token: %A" indentStr name inp.Head
            traceMsgIndent <- traceMsgIndent + 1

            match p.Run inp sigs events state with
            | Success (r, inp', events', state') ->
                printfn "%sExiting parser \"%s\". Next token: %A" indentStr name inp'.Head
                traceMsgIndent <- traceMsgIndent - 1
                Success (r, inp', events', state')
            | Failure (err, con, state') ->
                printfn "%sFailing parser: \"%s\". Current token: %A" indentStr name inp.Head
                traceMsgIndent <- traceMsgIndent - 1
                Failure (err, con, state')

        Parser (f, p.SignificantTokens, p.IsOpt)
