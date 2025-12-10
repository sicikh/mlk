[<AutoOpen>]
module MLK.Compiler.Parser.Impl

open MLK.Compiler.Text
open MLK.Compiler.Syntax
open MLK.Compiler.Parser.IParsec

let pIdent = pTokenS SyntaxKind.Ident
let pIntLiteral = pTokenS SyntaxKind.IntLiteral
let pLBracket = pTokenS SyntaxKind.LBracket
let pRBracket = pTokenS SyntaxKind.RBracket
let pLParen = pTokenS SyntaxKind.LParen
let pRParen = pTokenS SyntaxKind.RParen

let expectedExpression = ParseDiagnostic.mkSingleNode "expression"

let recoverExpr<'a> : 'a -> 'a parser -> 'a parser =
    recoverInto SyntaxKind.ErrExpr expectedExpression

// could make via `mkRec`, but parsers as values is better here
// (except to that we need to manually calculate significant tokens)
let pExpr =
    Parser (
        (fun _ _ -> failwith "uninitialized"),
        Set [ SyntaxKind.Ident ; SyntaxKind.Op ; SyntaxKind.IntLiteral ],
        false
    )

let pLiteral = node SyntaxKind.Literal <| pIntLiteral
let pParenExpr = node SyntaxKind.ParenExpr <| between pLParen pRParen pExpr

let pTerm = pLiteral <|> pParenExpr

let pPrefixOp =
    pChooseToken (fun t ->
        match t.Text, t.Kind with
        | "+", _ -> Some (5, SyntaxKind.UnaryExpr)
        | "-", _ -> Some (5, SyntaxKind.UnaryExpr)
        | _ -> None
    )

let pInfixOp =
    pChooseToken (fun t ->
        match t.Text, t.Kind with
        | ("+" | "-"), _ -> Some (1, 2, SyntaxKind.BinExpr)
        | ("*" | "/"), _ -> Some (3, 4, SyntaxKind.BinExpr)
        | _ -> None
    )

pExpr.Run <- (pPratt' pTerm pPrefixOp pInfixOp 0).Run

let parseRoot (sourceText : string) : ParseEvent list * Trivia list * ParseDiagnostic list =
    let tokens = Lexer.tokenize sourceText
    let tokenSource = TokenSource.FromTokens sourceText tokens

    match runParser pExpr tokenSource with
    | Success (_, state) -> state.Finish ()
    | Failure _ -> failwith "Parsing failed unexpectedly"
