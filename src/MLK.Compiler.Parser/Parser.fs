[<AutoOpen>]
module MLK.Compiler.Parser.Impl

open Stdx
open MLK.Compiler.Text
open MLK.Compiler.Syntax
open MLK.Compiler.Parser.IParsec

let pIdent = pTokenS SyntaxKind.Ident
let pIntLiteral = pTokenS SyntaxKind.IntLiteral
let pLBracket = pTokenS SyntaxKind.LBracket
let pRBracket = pTokenS SyntaxKind.RBracket
let pLParen = pTokenS SyntaxKind.LParen
let pRParen = pTokenS SyntaxKind.RParen
let pLetKw = pTokenS SyntaxKind.LetKw
let pInKw = pTokenS SyntaxKind.InKw
let pAndKw = pTokenS SyntaxKind.AndKw

let pEq =
    pChooseToken (fun t ->
        match t.Text with
        | "=" -> Some ()
        | _ -> None
    )

let pName = node SyntaxKind.Name pIdent
let pLiteral = node SyntaxKind.Literal <| pIntLiteral

let expectedExpression = ParseDiagnostic.mkSingleNode "expression"

let recoverExpr<'a> : 'a -> 'a parser -> 'a parser =
    recoverInto SyntaxKind.ErrExpr expectedExpression

// could make via `mkRec`, but parsers as values is better here
// (except to that we need to manually calculate significant tokens)
let pExprRun : (int -> RunParser<CompletedMarker>) ref =
    ref (fun _minbp _state _ctx -> failwith "uninitialized")

let pExpr' (minbp : int) : CompletedMarker parser =
    Parser (
        (fun state ctx -> pExprRun.Value minbp state ctx),
        Set [ SyntaxKind.Ident ; SyntaxKind.Op ; SyntaxKind.IntLiteral ],
        false
    )

let pExpr = pExpr' 0

let pPat = node SyntaxKind.NamedPat ^<| pName

let pBinding = node SyntaxKind.Binding ^<| pPat ^>> pEq ^>> pExpr
let pBindingList = node SyntaxKind.BindingList ^<| sepBy1 pBinding pAndKw

let pLetDecl = node SyntaxKind.LetDecl ^<| pLetKw ^>> pBindingList

let pLetExpr =
    node SyntaxKind.LetExpr
    ^<| localTokenMode (konst Gt)
    ^<| pLetDecl
        ^>> ((localIndentation Any (pInKw ^>>. pExpr))
             <|> (localIndentation Eq (localTokenMode (konst Ge) pExpr)))

let pParenExpr =
    node SyntaxKind.ParenExpr
    ^<| between pLParen pRParen
    ^<| localIndentation Any pExpr

let pTerm = pName <|> pLiteral <|> pParenExpr <|> pLetExpr

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
        | ";", _ -> Some (0, 0, SyntaxKind.SeqExpr)
        | ("+" | "-"), _ -> Some (1, 2, SyntaxKind.BinExpr)
        | ("*" | "/"), _ -> Some (3, 4, SyntaxKind.BinExpr)
        | _, SyntaxKind.Op -> Some (5, 6, SyntaxKind.BinExpr)
        | _ -> None
    )
    <|> (localIndentation Gt
         // ^<| absoluteIndentation
         ^<| testIndent (9, 10, SyntaxKind.AppExpr))
    <|> (localIndentation Eq
         // ^<| absoluteIndentation
         ^<| testIndent ((0, 0, SyntaxKind.SeqExpr)))

pExprRun.Value <- fun minbp -> (pPratt' pTerm pPrefixOp pInfixOp minbp).Run

let parseRoot (sourceText : string) : ParseEvent list * Trivia list * ParseDiagnostic list =
    let tokens = Lexer.tokenize sourceText
    let tokenSource = TokenSource.FromTokens sourceText tokens

    match runParser pExpr tokenSource with
    | Success (_, state) -> state.Finish ()
    | Failure _ -> failwith "Parsing failed unexpectedly"
