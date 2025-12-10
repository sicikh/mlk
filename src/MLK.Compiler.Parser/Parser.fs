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

let recoverExpr<'a> : 'a -> 'a parser -> 'a parser =
    recoverInto SyntaxKind.ErrExpr expectedExpression

// let pExpr0 : uparser =
//     mkRec
//     <| fun pExpr ->
//         let pFunc =
//             node SyntaxKind.FunExpr
//             <| pIdent ^>> pLParen ^>> recoverExpr () pExpr ^>> opt pRParen
//
//         let pList =
//             node SyntaxKind.ListExpr
//             <| pLBracket ^>> recoverExpr () (manyU pExpr) ^>> opt pRBracket
//
//         pFunc <|> pList

let pTerm = node SyntaxKind.Literal <| pTokenS SyntaxKind.IntLiteral

let pPrefixOp =
    pChooseToken (fun t ->
        match t.Text, t.Kind with
        | "+", SyntaxKind.Ident -> Some (5, SyntaxKind.UnaryExpr)
        | "-", SyntaxKind.Ident -> Some (5, SyntaxKind.UnaryExpr)
        | _ -> None
    )

let pInfixOp =
    pChooseToken (fun t ->
        match t.Text, t.Kind with
        | "+", SyntaxKind.Ident -> Some (1, 2, SyntaxKind.BinExpr)
        | "*", SyntaxKind.Ident -> Some (3, 4, SyntaxKind.BinExpr)
        | _ -> None
    )

let pArithExpr = pPratt' pTerm pPrefixOp pInfixOp 0


let parseRoot (sourceText : string) : ParseEvent list * Trivia list * ParseDiagnostic list =
    let tokens = Lexer.tokenize sourceText
    let tokenSource = TokenSource.FromTokens sourceText tokens

    match runParser pArithExpr tokenSource with
    | Success (_, state) -> state.Finish ()
    | Failure _ -> failwith "Parsing failed unexpectedly"
