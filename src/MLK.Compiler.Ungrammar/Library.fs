namespace MLK.Compiler.Ungrammar

open System
open System.Text
open MLK.Compiler.Text

/// A node, like `A = 'b' | 'c'`.
type Node = | Node of uint32
/// A token, denoted with single quotes, like `'struct'` or `'+'`.
type Token = | Token of uint32

/// A production rule.
type Rule =
    /// A labeled rule, like `a:B` (`a` is the label, `B` is the rule).
    | RLabeled of label : string * rule : Rule
    /// A node, like `A`.
    | RNode of Node
    /// A token, like `'struct'`.
    | RToken of Token
    /// A sequence of rules, like `A 'b' C`.
    | RSeq of Rule list
    /// An alternative between many rules, like `'+' | '-' | '*' | '/'`.
    | RAlt of Rule list
    /// An optional rule, like `A?`.
    | ROpt of Rule
    /// A repeated rule, like `A*`.
    | RStar of Rule

type TokenData = | TokenData of name : string
type NodeData = | NodeData of name : string * rule : Rule

type Grammar =
    private
        {
            TokensData : TokenData list
            NodesData : NodeData list
        }

    member this.Tokens : Token seq =
        this.TokensData |> Seq.mapi (fun i _ -> uint32 i |> Token)

    member this.Nodes : Node seq = this.NodesData |> Seq.mapi (fun i _ -> uint32 i |> Node)

    member this.Token (Token index) : TokenData = this.TokensData[int index]
    member this.Node (Node index) : NodeData = this.NodesData[int index]

type internal TokenKind =
    | TNode of string
    | TToken of string
    | TEq
    | TStar
    | TPipe
    | TQMark
    | TColon
    | TLParen
    | TRParen

type Error =
    {
        Message : string
        Location : LineCol option
    }

    member this.WithLocation (loc : LineCol) = { this with Location = Some loc }

    override this.ToString () =
        match this.Location with
        | Some loc -> $"{loc}: {this.Message}"
        | None -> $"{this.Message}"

type Result<'T> = Result<'T, Error>

module Error =
    let create message : Error = { Message = message ; Location = None }
    let bail message : Result<'T> = create message |> Error


module private Lexer =
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
                res <- Error.bail msg
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

        | '\r' -> Error.bail "unexpected `\\r`, only Unix-style line endings allowed"
        | c -> Error.bail $"unexpected character: `{c}`"

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

module private Parser =
    open Lexer
