namespace MLK.Compiler.Parser

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
            StartLineCol = LineCol.create UInt32.MaxValue UInt32.MaxValue
        }

    override this.ToString (): string =
        $"{this.Kind}@{this.Range}({this.StartLineCol}) \"{this.Text}\""

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
        let clearTokensFromTrivia (tokens : Token list) : Token list * Trivia list =
            // trivia after and including the newline is considered the leading trivia of the next token
            let tokenToTrivia (token : Token) : Trivia option =
                let syntaxKindToTriviaPieceKind (kind : SyntaxKind) : TriviaPieceKind option =
                    match kind with
                    | SyntaxKind.Whitespace -> Some TriviaPieceKind.Whitespace
                    | SyntaxKind.Newline -> Some TriviaPieceKind.Newline
                    // | SyntaxKind.LineComment -> TriviaPieceKind.LineComment
                    // | SyntaxKind.BlockComment -> TriviaPieceKind.BlockComment
                    | _ -> None

                match syntaxKindToTriviaPieceKind token.Kind with
                | Some triviaKind ->
                    Some
                        {
                            Kind = triviaKind
                            Range = token.Range
                            // TODO: determine properly, it's easy
                            Trailing = false
                        }
                | None -> None

            // TODO: make in one pass
            let trivias = tokens |> List.choose tokenToTrivia
            let tokens = tokens |> List.filter (tokenToTrivia >> Option.isNone)

            tokens, trivias

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

    member this.IsEof : bool = this.Head.Kind = SyntaxKind.Eof

    member this.Position : TextSize = this.Head.Range.Start

    member this.HasSameCurrentToken (other : TokenSource) : bool =
        this.Head = other.Head

// workaround for FsLex that does not let change the name of the result type of lexer func
type token = Token
