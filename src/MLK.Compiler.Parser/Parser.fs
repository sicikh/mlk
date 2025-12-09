[<AutoOpen>]
module MLK.Compiler.Parser.Impl

open MLK.Compiler.Text
open MLK.Compiler.Syntax
open MLK.Compiler.Parser.IParsec

let pIdent = pTokenS SyntaxKind.Ident
let pLBracket = pTokenS SyntaxKind.LBracket
let pRBracket = pTokenS SyntaxKind.RBracket
let pLParen = pTokenS SyntaxKind.LParen
let pRParen = pTokenS SyntaxKind.RParen

let expectedExpression = ParseDiagnostic.mkSingleNode "expression"

let recoverExpr<'a> : 'a -> 'a parser -> 'a parser = recoverInto SyntaxKind.ErrExpr expectedExpression

let pExpr : uparser = mkRec <| fun pExpr ->
    let pFunc = node SyntaxKind.FunExpr <| pIdent ^>> pLParen ^>> recoverExpr () pExpr ^>> opt pRParen
    let pList = node SyntaxKind.ListExpr <| pLBracket ^>> recoverExpr [] (many pExpr) ^>> opt pRBracket

    pFunc <|> pList

let parseRoot (sourceText : string) : ParseEvent list * ParseDiagnostic list =
    let tokens = Lexer.tokenize sourceText
    let tokenSource = TokenSource.FromTokens sourceText tokens
    match runParser pExpr tokenSource with
    | Success ((), _, events, state) -> List.rev events, state.Diagnostics
    | Failure _ ->
        failwith "Parsing failed unexpectedly"
