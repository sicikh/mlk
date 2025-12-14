namespace MLK.Compiler.Ungrammar

open System
open Stdx
open System.Text
open MLK.Compiler.Text
open System.Collections.Generic

type Node = | Node of uint32
type Token = | Token of uint32

type Rule =
    | RLabeled of label : string * rule : Rule
    | RNode of Node
    | RToken of Token
    | RSeq of Rule list
    | RAlt of Rule list
    | ROpt of Rule
    | RRep of Rule

[<Sealed>]
type TokenData(name : string) =
    member this.Name = name

[<Sealed>]
type NodeData(name : string, rule : Rule) =
    let mutable rule = rule

    member this.Name = name

    member this.Rule
        with get () = rule
        and set v = rule <- v

module TokenData =
    let (|TokenData|) (tokenData : TokenData) : string = tokenData.Name

module NodeData =
    let (|NodeData|) (nodeData : NodeData) : string * Rule = nodeData.Name, nodeData.Rule

type Grammar =
    {
        mutable TokensData : TokenData list
        mutable NodesData : NodeData list
    }

    member this.Tokens : Token seq =
        this.TokensData |> Seq.mapi (fun i _ -> uint32 i |> Token)

    member this.Nodes : Node seq = this.NodesData |> Seq.mapi (fun i _ -> uint32 i |> Node)

    member this.Token (Token index) : TokenData = this.TokensData[int index]
    member this.Node (Node index) : NodeData = this.NodesData[int index]

type TokenKind =
    | TNode of string
    | TToken of string
    | TEq
    | TStar
    | TPipe
    | TQMark
    | TColon
    | TLParen
    | TRParen

type SyntaxError =
    {
        Message : string
        Location : LineCol option
    }

    member this.WithLocation (loc : LineCol) = { this with Location = Some loc }

    override this.ToString () =
        match this.Location with
        | Some loc -> $"{loc}: {this.Message}"
        | None -> $"{this.Message}"

type SyntaxException(err : SyntaxError) =
    inherit Exception(err.ToString ())
    member this.Error = err

module SyntaxError =
    let create message : SyntaxError = { Message = message ; Location = None }

    let createLoc loc message : SyntaxError =
        {
            Message = message
            Location = Some loc
        }

    let withLocation loc (err : SyntaxError) : SyntaxError = err.WithLocation loc

    let raise (err : SyntaxError) : 'T = raise <| SyntaxException err
    let createRaise message : 'T = create message |> raise
    let createLocRaise loc message : 'T = createLoc loc message |> raise

module Lexer =
    type Token =
        {
            Kind : TokenKind
            Location : LineCol
        }

    let advanceLocation (text : string) (loc : LineCol) : LineCol =
        match text.IndexOf '\n' with
        | -1 ->
            { loc with
                Column = loc.Column + uint32 text.Length
            }
        | idx ->
            let lineIncr = text |> String.filter ((=) '\n') |> String.length |> uint32

            let newColumn = text.Substring(idx + 1).Length |> uint32

            {
                Line = loc.Line + lineIncr
                Column = newColumn
            }

    let (|Escapable|_|) (c : char) : bool =
        match c with
        | '\\'
        | '\'' -> true
        | _ -> false

    let (|Whitespace|_|) (c : char) : bool =
        match c with
        | ' '
        | '\t'
        | '\n' -> true
        | _ -> false

    let identLetters =
        [ 'a' .. 'z' ] @ [ 'A' .. 'Z' ] @ [ '0' .. '9' ] @ [ '_' ] |> Set.ofList

    let (|Ident|_|) (c : char) : bool = Set.contains c identLetters

    let advance (input : string) : TokenKind * string =
        let mutable rest = input

        let nextChar () =
            if rest.Length = 0 then
                None
            else
                let c = rest[0]
                rest <- rest[1..]
                Some c

        let peekChar () : char option =
            if rest.Length = 0 then None else Some rest[0]

        let emit t = (t, rest)

        match nextChar().Value with
        | '=' -> emit TEq
        | '*' -> emit TStar
        | '?' -> emit TQMark
        | '(' -> emit TLParen
        | ')' -> emit TRParen
        | '|' -> emit TPipe
        | ':' -> emit TColon
        | '\'' ->
            let mutable buf = StringBuilder ()

            let pushBuf (c : char) = buf.Append c |> ignore

            let mutable closed = false

            while not closed do
                match nextChar () with
                | None -> SyntaxError.createRaise "unclosed token literal"
                | Some '\\' ->
                    match nextChar () with
                    | Some (Escapable as c) -> pushBuf c
                    | _ -> SyntaxError.createRaise "invalid escape in token literal"
                | Some '\'' -> closed <- true
                | Some c -> pushBuf c

            TToken <| buf.ToString (), rest

        | Ident as c ->
            let buf = StringBuilder ()
            buf.Append c |> ignore
            let mutable isDone = false

            while not isDone do
                match peekChar () with
                | Some (Ident as c) ->
                    nextChar () |> ignore
                    buf.Append c |> ignore
                | _ -> isDone <- true

            TNode <| buf.ToString (), rest

        | '\r' -> SyntaxError.createRaise "unexpected `\\r`, only Unix-style line endings allowed"
        | c -> SyntaxError.createRaise $"unexpected character: `{c}`"

    let skipWhitespace (input : string) : string = input.TrimStart ()

    let skipComment (input : string) : string =
        if input.StartsWith "//" then
            let idx =
                match input.IndexOf '\n' with
                | -1 -> input.Length
                | i -> i + 1

            input[idx..]
        else
            input

    let tokenize (input : string) : Token list =
        let mutable input = input
        let mutable location = LineCol.zero
        let mutable tokens = []

        while not <| String.IsNullOrEmpty input do
            let oldInput = input
            input <- skipWhitespace input
            input <- skipComment input

            if oldInput.Length = input.Length then
                try
                    let kind, rest = advance input
                    input <- rest
                    let token = { Kind = kind ; Location = location }
                    tokens <- token :: tokens
                with :? SyntaxException as ex ->
                    ex.Error |> SyntaxError.withLocation location |> SyntaxError.raise

            let consumed = oldInput.Length - input.Length
            location <- location |> advanceLocation (oldInput[..consumed])

        List.rev tokens

type Parser(tokens : Lexer.Token list) =
    let mutable tokens = tokens
    let nodeTable = Dictionary<string, Node> ()
    let tokenTable = Dictionary<string, Token> ()
    let grammar = { TokensData = [] ; NodesData = [] }

    static let DUMMY_RULE = RNode (Node ~~~0u)

    let peekN (n : int) : Lexer.Token option = List.tryItem n tokens
    let peek () : Lexer.Token option = peekN 0

    let bump () : Lexer.Token =
        match tokens with
        | [] -> SyntaxError.create "unexpected end of input" |> SyntaxError.raise
        | t :: rest ->
            tokens <- rest
            t

    let expect (what : string) (kind : TokenKind) =
        let token = bump ()

        if token.Kind <> kind then
            SyntaxError.createLocRaise token.Location $"unexpected token, expected {what}"

    let isEof () : bool = tokens.IsEmpty

    let finish () : Grammar =
        grammar.NodesData
        |> List.tryPick (
            function
            | nd when nd.Rule = DUMMY_RULE -> Some nd.Name
            | _ -> None
        )
        |> function
            | Some name -> SyntaxError.createRaise $"undefined node: {name}"
            | None -> grammar

    let internNode (name : string) : Node =
        nodeTable
        |> Dictionary.getOrAdd
            name
            (fun () ->
                let n = Node (uint32 nodeTable.Count)
                grammar.NodesData <- grammar.NodesData @ [ NodeData (name, DUMMY_RULE) ]
                n
            )

    let internToken (name : string) : Token =
        tokenTable
        |> Dictionary.getOrAdd
            name
            (fun () ->
                let t = Token (uint32 tokenTable.Count)
                grammar.TokensData <- grammar.TokensData @ [ TokenData name ]
                t
            )

    let rec node () =
        let token = bump ()

        let node =
            match token.Kind with
            | TNode it -> internNode it
            | _ -> SyntaxError.createLocRaise token.Location "expected node identifier"

        expect "='" TEq

        if (grammar.Node node).Rule <> DUMMY_RULE then
            SyntaxError.createLocRaise token.Location $"duplicate rule: {(grammar.Node node).Name}"

        let rule = rule ()
        let (Node nodeIdx) = node
        grammar.NodesData[int nodeIdx].Rule <- rule

    and rule () : Rule =
        match peek () with
        | Some token when token.Kind = TPipe ->
            SyntaxError.createLocRaise
                token.Location
                "The first element in a sequence of productions or alternatives must not have a leading pipe (`|`)"
        | _ -> ()

        let lhs = seqRule ()
        let mutable alts = [ lhs ]

        let hasAlts () =
            match peek () with
            | Some token when token.Kind = TPipe -> true
            | _ -> false

        while hasAlts () do
            bump () |> ignore // consume the pipe

            let rule = seqRule ()
            alts <- rule :: alts

        match alts with
        | [ single ] -> single
        | _ -> RAlt <| List.rev alts

    and seqRule () : Rule =
        let lhs = atomRule ()

        let mutable seqs = [ lhs ]

        let rec loop () =
            match optAtomRule () with
            | Some r ->
                seqs <- r :: seqs
                loop ()
            | None -> ()

        loop ()

        match seqs with
        | [ single ] -> single
        | _ -> RSeq <| List.rev seqs

    and atomRule () : Rule =
        match optAtomRule () with
        | Some r -> r
        | None ->
            let token = bump ()
            SyntaxError.createLocRaise token.Location "unexpected token"

    and optAtomRule () : Rule option =
        match peek () with
        | None -> None
        | Some token ->
            let res =
                match token.Kind with
                | TNode name ->
                    match peekN 1 with
                    | Some lookahead when lookahead.Kind = TEq -> None
                    | Some lookahead when lookahead.Kind = TColon ->
                        bump () |> ignore // consume node
                        bump () |> ignore // consume colon
                        let rule = atomRule ()
                        Some <| RLabeled (name, rule)
                    | _ ->
                        bump () |> ignore
                        let node = internNode name
                        Some <| RNode node
                | TToken name ->
                    bump () |> ignore
                    let token = internToken name
                    Some <| RToken token
                | TLParen ->
                    bump () |> ignore // consume '('
                    let rule = rule ()
                    expect "')'" TRParen
                    Some rule
                | _ -> None

            match res, peek () with
            | None, _ -> None
            | Some r, None -> Some r
            | Some r, Some lookahead ->
                match lookahead.Kind with
                | TQMark ->
                    bump () |> ignore
                    Some <| ROpt r
                | TStar ->
                    bump () |> ignore
                    Some <| RRep r
                | _ -> Some r

    member this.Run () : Grammar =
        let rec loop () : Grammar =
            if isEof () then
                finish ()
            else
                node ()
                loop ()

        loop ()

type Grammar with
    static member Parse (input : string) : Grammar =
        let tokens = Lexer.tokenize input
        let grammar = Parser(tokens).Run ()
        grammar

type Rule with
    member this.PrettyPrint (grammar : Grammar) : string =
        let rec ppRule (rule : Rule) : string =
            match rule with
            | RLabeled (label, rule) -> $"{label}:{ppRule rule}"
            | RNode node -> (grammar.Node node).Name
            | RToken token -> $"'{(grammar.Token token).Name}'"
            | RSeq rules ->
                rules
                |> List.map ppRule
                |> String.concat " "
            | RAlt rules ->
                rules
                |> List.map ppRule
                |> String.concat " | "
            | ROpt rule -> $"{ppRule rule}?"
            | RRep rule -> $"{ppRule rule}*"

        ppRule this
