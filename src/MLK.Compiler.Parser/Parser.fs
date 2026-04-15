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
let pFunKw = pTokenS SyntaxKind.FunKw
let pArrow = pTokenS SyntaxKind.Arrow
let pSemi = pTokenS SyntaxKind.Semicolon
let pIf = pTokenS SyntaxKind.IfKw
let pThen = pTokenS SyntaxKind.ThenKw
let pElse = pTokenS SyntaxKind.ElseKw

let pEq =
    pChooseToken (fun t ->
        match t.Text with
        | "=" -> Some ()
        | _ -> None
    )

let pName = node SyntaxKind.Name pIdent
let pLiteral = node SyntaxKind.Literal <| pIntLiteral

let expectedExpression = ParseDiagnostic.mkSingleNode "expression"
let expectedClosingParen = ParseDiagnostic.mkSingleNode "closing parenthesis ')'"
let expectedClosingBracket = ParseDiagnostic.mkSingleNode "closing bracket ']'"

let recoverExpr = recoverInto SyntaxKind.ErrExpr expectedExpression

// could make via `mkRec`, but parsers as values is better here
// (except to that we need to manually calculate significant tokens)
let pExprRun : (int -> RunParser<CompletedMarker>) ref =
    ref (fun _minbp _state _ctx -> failwith "uninitialized")

let pExpr' (minbp : int) : CompletedMarker parser =
    Parser (
        (fun state ctx -> pExprRun.Value minbp state ctx),
        Set
            [
                SyntaxKind.Ident
                SyntaxKind.Op
                SyntaxKind.IntLiteral
                SyntaxKind.LetKw
                SyntaxKind.LParen
            ],
        false
    )

let pExpr = pExpr' 0

// let pPat = node SyntaxKind.NamedPat ^<| pName
// let pBinding = node SyntaxKind.Binding ^<| pPat ^>> pEq ^>> pExpr
// let pBindingList = node SyntaxKind.BindingList ^<| sepBy1 pBinding pAndKw

let pLetDecl = node SyntaxKind.LetDecl ^<| pLetKw ^>> pName ^>> pEq ^>> (localIndentation Gt pExpr)

let pFun =
    let p =
        localAbsoluteIndentation
        ^<| localIndentation Any
        ^<| localTokenMode (konst Gt)
        ^<| pipe3 pFunKw pName pArrow (fun _ _ _ -> ())

    node SyntaxKind.FunExpr ^<| (pCheckIndent p) ^>> (localIndentation Gt pExpr)

let pLetExpr =
    let inVar = (localIndentation Any ((localTokenMode (konst Any) pInKw) ^>>. pExpr))

    node SyntaxKind.LetExpr ^<| pLetDecl ^>> (inVar <|> (localIndentation Eq pExpr))

let pParenExpr =
    node SyntaxKind.ParenExpr
    ^<| between pLParen (localTokenMode (konst Any) (tryOrDiag expectedClosingParen pRParen))
    ^<| localIndentation Any pExpr

let pList =
    node SyntaxKind.ListExpr
    ^<| between pLBracket (localTokenMode (konst Any) (tryOrDiag expectedClosingBracket pRBracket))
    ^<| localIndentation Any
    // TODO: indent + optional trailing
    ^<| sepBy pExpr pSemi

let pIfThenElse =
    let p =
        pIf
        ^>> (localIndentation Gt pExpr)
        ^>> pThen
        ^>> (localIndentation Gt pExpr)
        ^>> pElse
        ^>> (localIndentation Gt pExpr)

    node SyntaxKind.IfExpr p

let pVar = node SyntaxKind.VarExpr pName

let pTerm = pVar <|> pFun <|> pLetExpr <|> pLiteral <|> pParenExpr <|> pList <|> pIfThenElse

let pPrefixOp =
    pChooseToken (fun t ->
        match t.Text, t.Kind with
        | "+", _ -> Some (5, SyntaxKind.UnaryExpr)
        | "-", _ -> Some (5, SyntaxKind.UnaryExpr)
        | _ -> None
    )
    |> withSignificant (Set [ SyntaxKind.Op ])

let pInfixOp =
    let isSignificant (t : Token) =
        pTerm.SignificantTokens.Contains t.Kind || t.Kind = SyntaxKind.ErrToken

    pChooseToken (fun t ->
        match t.Text, t.Kind with
        // TODO: introduce flag `InList`. Listmakers enable it, parens --- disable
        // | ";", _ -> Some (0, 0, SyntaxKind.SeqExpr)
        | ("+" | "-"), _ -> Some (1, 2, SyntaxKind.BinExpr)
        | ("*" | "/"), _ -> Some (3, 4, SyntaxKind.BinExpr)
        | ("=" | "<" | ">" | "<=" | ">="), _ -> Some (2, 3, SyntaxKind.BinExpr)
        | ("&&" | "||"), _ -> Some (0, 1, SyntaxKind.BinExpr)
        | _, SyntaxKind.Op -> Some (5, 6, SyntaxKind.BinExpr)
        | _ -> None
    )
    <|> (localIndentation Gt ^<| pCheckToken isSignificant
         |>> (konst (9, 10, SyntaxKind.AppExpr)))
    <|> (localIndentation Eq ^<| pCheckToken isSignificant
         |>> (konst (0, 0, SyntaxKind.SeqExpr)))
    |> withSignificant (
        Set.union pExpr.SignificantTokens
        <| Set [ SyntaxKind.Op ; SyntaxKind.Semicolon ]
    )

pExprRun.Value <- fun minbp -> (pPratt' pTerm pPrefixOp pInfixOp minbp).Run

let parseRoot (sourceText : string) : ParseEvent list * Trivia list * ParseDiagnostic list =
    let tokens = Lexer.tokenize sourceText
    let tokenSource = TokenSource.FromTokens sourceText tokens

    match runParser pExpr tokenSource with
    | Success (_, state) -> state.Finish ()
    | Failure _ -> failwith "Parsing failed unexpectedly"

open MLK.Compiler.Fusca

let exprRoot (source : string) (events : ParseEvent list) (trivias : Trivia list) : Expr =
    let sink = LosslessTreeSink(source, trivias)
    ParseEvent.processEvents sink [] events
    match sink.Finish() with
    | Some tree ->
        let green = GenericTree.intoGreen tree
        match AstNode.cast<Expr> (SyntaxNode.CreateRoot green) with
        | Some expr -> expr
        | None -> failwith "Failed to cast root node to Expr"
    | None -> failwith "Failed to construct syntax tree from events"
