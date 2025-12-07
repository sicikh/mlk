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
    | RStar of Rule

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

type Result<'T> = Result<'T, SyntaxError>

module SyntaxError =
    let create message : SyntaxError = { Message = message ; Location = None }
    let bail message : Result<'T> = create message |> Error
    let withLocation loc (err : SyntaxError) : SyntaxError = err.WithLocation loc

    let bailWithLocation loc message : Result<'T> =
        create message |> withLocation loc |> Error


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

    let advance (input : string) : Result<TokenKind * string> =
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

        let emit t = Ok (t, rest)

        match nextChar().Value with
        | '=' -> emit TEq
        | '*' -> emit TStar
        | '?' -> emit TQMark
        | '(' -> emit TLParen
        | ')' -> emit TRParen
        | '|' -> emit TPipe
        | ':' -> emit TColon
        | '\'' ->
            let mutable res = Ok <| StringBuilder ()

            let pushBuf (c : char) =
                Result.iter (fun (buf : StringBuilder) -> buf.Append c |> ignore) res

            let mutable closed = false

            let bail msg =
                res <- SyntaxError.bail msg
                closed <- true

            while not closed do
                match nextChar () with
                | None -> bail "unclosed token literal"
                | Some '\\' ->
                    match nextChar () with
                    | Some (Escapable as c) -> pushBuf c
                    | _ -> bail "invalid escape in token literal"
                | Some '\'' -> closed <- true
                | Some c -> pushBuf c

            res |> Result.map (fun buf -> TToken <| buf.ToString (), rest)

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

            Ok (TNode <| buf.ToString (), rest)

        | '\r' -> SyntaxError.bail "unexpected `\\r`, only Unix-style line endings allowed"
        | c -> SyntaxError.bail $"unexpected character: `{c}`"

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

    let tokenize (input : string) : Result<Token list> =
        let mutable input = input
        let mutable location = LineCol.zero
        let mutable res = Ok []

        while not <| String.IsNullOrEmpty input && res.IsOk do
            let oldInput = input
            input <- skipWhitespace input
            input <- skipComment input

            if oldInput.Length = input.Length then
                match advance input with
                | Ok (kind, rest) ->
                    input <- rest
                    let token = { Kind = kind ; Location = location }
                    res <- Result.map (fun lst -> token :: lst) res
                | Error e -> res <- Error <| e.WithLocation location

            let consumed = oldInput.Length - input.Length
            location <- location |> advanceLocation (oldInput[..consumed])

        res |> Result.map List.rev

type Parser(tokens : Lexer.Token list) =
    let mutable tokens = tokens
    let nodeTable = Dictionary<string, Node> ()
    let tokenTable = Dictionary<string, Token> ()
    let grammar = { TokensData = [] ; NodesData = [] }

    static let DUMMY_RULE = RNode (Node 0u)

    let peekN (n : int) : Lexer.Token option = List.tryItem n tokens
    let peek () : Lexer.Token option = peekN 0

    let bump () : Result<Lexer.Token> =
        match tokens with
        | [] -> SyntaxError.bail "unexpected end of input"
        | t :: rest ->
            tokens <- rest
            Ok t

    let expect (what : string) (kind : TokenKind) : Result<unit> =
        match bump () with
        | Error e -> Error e
        | Ok t when t.Kind = kind -> Ok ()
        | Ok t -> SyntaxError.bailWithLocation t.Location $"unexpected token, expected {what}"

    let isEof () : bool = tokens.IsEmpty

    let finish () : Result<Grammar> =
        grammar.NodesData
        |> List.tryPick (
            function
            | nd when nd.Rule = DUMMY_RULE -> Some nd.Name
            | _ -> None
        )
        |> function
            | Some name -> SyntaxError.bail $"undefined node: {name}"
            | None -> Ok grammar

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

    let rec node () : Result<unit> =
        match bump () with
        | Ok ({ Kind = TNode it } as token) ->
            let node = internNode it

            match expect "=" TEq with
            | Error e -> Error e
            | Ok () ->
                match grammar.Node node with
                | nd when nd.Rule <> DUMMY_RULE ->
                    SyntaxError.bailWithLocation token.Location $"duplicate rule: {nd.Name}"
                | _ ->
                    match rule () with
                    | Ok rule ->
                        let (Node node) = node
                        grammar.NodesData[int node].Rule <- rule
                        Ok ()
                    | Error e -> Error e
        | Error e -> Error e
        | Ok t -> SyntaxError.bailWithLocation t.Location "expected node identifier"

    and rule () : Result<Rule> = failwith "todo"
    and seqRule () : Result<Rule> = failwith "todo"
    and atomRule () : Result<Rule> = failwith "todo"
    and optAtomRule () : Result<Rule> = failwith "todo"

    member this.Run () : Result<Grammar> =
        let rec loop () : Result<Grammar> =
            if isEof () then
                finish ()
            else
                match node () with
                | Ok () -> loop ()
                | Error e -> Error e

        loop ()

type Grammar with
    static member Parse (input : string) : Result<Grammar> =
        match Lexer.tokenize input with
        | Error e -> Error e
        | Ok tokens -> Parser(tokens).Run ()
