namespace MLK.Compiler.Parser.TokenSource

open System
open MLK.Compiler.Text
open MLK.Compiler.Fusca
open MLK.Compiler.Syntax

type Token =
    {
        Kind : SyntaxKind
        Text : string
        Range : TextRange
        StartLineCol : LineCol
    }

    static member Eof =
        {
            Kind = SyntaxKind.Eof
            Text = ""
            Range = TextRange.create (TextSize.ofUint UInt32.MaxValue) (TextSize.ofUint UInt32.MaxValue)
            StartLineCol = LineCol.zero
        }

type Trivia =
    {
        Kind : TriviaPieceKind
        Range : TextRange
        /// Trivia after and including the newline is considered the leading trivia of the next token
        Trailing : bool
    }

/// Token source where any trivia has been removed from tokens and stored separately
[<NoEquality ; NoComparison>]
type TokenSource =
    private
    | TokenSource of input : string * tokens : Token list * trivias : Trivia list

    static member FromTokens (input : string) (tokens : Token list) : TokenSource =
        let clearTokensFromTrivia (_tokens : Token list) : Token list * Trivia list = failwith "todo"

        let tokens, trivias = clearTokensFromTrivia tokens

        TokenSource (input, tokens, trivias)

    member this.Input : string =
        match this with
        | TokenSource (input, _, _) -> input

    member this.Tokens : Token list =
        match this with
        | TokenSource (_, tokens, _) -> tokens

    member this.Trivias : Trivia list =
        match this with
        | TokenSource (_, _, trivias) -> trivias

    member this.Head : Token =
        match this with
        | TokenSource (_, tokens, _) ->
            match tokens with
            | [] -> Token.Eof
            | head :: _ -> head

    member this.Tail : TokenSource =
        match this with
        | TokenSource (input, tokens, trivias) ->
            match tokens with
            | [] -> this
            | _ :: tail -> TokenSource (input, tail, trivias)

    member this.Uncons : Token * TokenSource =
        match this with
        | TokenSource (input, tokens, trivias) ->
            match tokens with
            | [] -> (Token.Eof, this)
            | head :: tail -> (head, TokenSource (input, tail, trivias))

// workaround for FsLex that does not let change the name of the result type of lexer func
type token = Token
