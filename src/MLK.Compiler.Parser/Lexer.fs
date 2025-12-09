module MLK.Compiler.Parser.Lexer

open FSharp.Text.Lexing
open MLK.Compiler.Syntax
open MLK.Compiler.Parser.TokenSource
open MLK.Compiler.Parser.Lex

let tokenize (source : string) : Token list =
    let lexbuf = LexBuffer<char>.FromString source
    let mutable tokens = []
    let mutable token = read lexbuf

    while token.Kind <> SyntaxKind.Eof do
        tokens <- token :: tokens
        token <- read lexbuf

    List.rev tokens
