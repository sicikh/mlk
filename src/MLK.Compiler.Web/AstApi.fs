module MLK.Compiler.Web.AstApi

open MLK.Compiler.Parser
open MLK.Compiler.Text
open MLK.Compiler.Syntax

type Range = int * int

type Node =
    { name: string
      ``type``: string
      range: Range
      children: Node list }

type Diagnostic =
    { message: string }

type AstResponse =
    { diagnostics: Diagnostic list
      tree: Node option }

let private mapDiagnostic (d: ParseDiagnostic) : Diagnostic =
    { message = d.Message }

let private rangeToTuple (r: TextRange) : Range =
    (TextSize.toInt r.Start, TextSize.toInt r.End)

let private kindName (kind: SyntaxKind) : string =
    match SyntaxKind.toString kind with
    | ValueSome s -> s
    | ValueNone ->                         
        match kind with
        | SyntaxKind.Tombstone -> "Tombstone"
        | SyntaxKind.Eof -> "Eof"
        | SyntaxKind.LParen -> "LParen"
        | SyntaxKind.RParen -> "RParen"
        | SyntaxKind.Comma -> "Comma"
        | SyntaxKind.Arrow -> "Arrow"
        | SyntaxKind.Dot -> "Dot"
        | SyntaxKind.Colon -> "Colon"
        | SyntaxKind.Semicolon -> "Semicolon"
        | SyntaxKind.LBracket -> "LBracket"
        | SyntaxKind.RBracket -> "RBracket"
        | SyntaxKind.Underscore -> "Underscore"
        | SyntaxKind.LBrace -> "LBrace"
        | SyntaxKind.RBrace -> "RBrace"
        | SyntaxKind.AndKw -> "AndKw"
        | SyntaxKind.AsKw -> "AsKw"
        | SyntaxKind.ElseKw -> "ElseKw"
        | SyntaxKind.FalseKw -> "FalseKw"
        | SyntaxKind.FunKw -> "FunKw"
        | SyntaxKind.IfKw -> "IfKw"
        | SyntaxKind.InKw -> "InKw"
        | SyntaxKind.LetKw -> "LetKw"
        | SyntaxKind.MatchKw -> "MatchKw"
        | SyntaxKind.ModuleKw -> "ModuleKw"
        | SyntaxKind.OpenKw -> "OpenKw"
        | SyntaxKind.RecKw -> "RecKw"
        | SyntaxKind.ThenKw -> "ThenKw"
        | SyntaxKind.TrueKw -> "TrueKw"
        | SyntaxKind.WhenKw -> "WhenKw"
        | SyntaxKind.WithKw -> "WithKw"
        | SyntaxKind.Comment -> "Comment"
        | SyntaxKind.ErrToken -> "ErrToken"
        | SyntaxKind.Ident -> "Ident"
        | SyntaxKind.MultilineComment -> "MultilineComment"
        | SyntaxKind.Newline -> "Newline"
        | SyntaxKind.Op -> "Op"
        | SyntaxKind.Whitespace -> "Whitespace"
        | SyntaxKind.AndPat -> "AndPat"
        | SyntaxKind.AppExpr -> "AppExpr"
        | SyntaxKind.ArgPat -> "ArgPat"
        | SyntaxKind.ArgPats -> "ArgPats"
        | SyntaxKind.AsPat -> "AsPat"
        | SyntaxKind.BinExpr -> "BinExpr"
        | SyntaxKind.Binding -> "Binding"
        | SyntaxKind.BindingList -> "BindingList"
        | SyntaxKind.BoolLiteral -> "BoolLiteral"
        | SyntaxKind.CharLiteral -> "CharLiteral"
        | SyntaxKind.ConsPat -> "ConsPat"
        | SyntaxKind.ErrDecl -> "ErrDecl"
        | SyntaxKind.ErrExpr -> "ErrExpr"
        | SyntaxKind.ErrNode -> "ErrNode"
        | SyntaxKind.ErrPat -> "ErrPat"
        | SyntaxKind.ErrTy -> "ErrTy"
        | SyntaxKind.Expr -> "Expr"
        | SyntaxKind.FnTy -> "FnTy"
        | SyntaxKind.FunExpr -> "FunExpr"
        | SyntaxKind.FuncPat -> "FuncPat"
        | SyntaxKind.IfExpr -> "IfExpr"
        | SyntaxKind.InferTy -> "InferTy"
        | SyntaxKind.InnerModuleDecl -> "InnerModuleDecl"
        | SyntaxKind.IntLiteral -> "IntLiteral"
        | SyntaxKind.LetDecl -> "LetDecl"
        | SyntaxKind.LetExpr -> "LetExpr"
        | SyntaxKind.ListExpr -> "ListExpr"
        | SyntaxKind.ListExprElements -> "ListExprElements"
        | SyntaxKind.ListPat -> "ListPat"
        | SyntaxKind.ListPatElements -> "ListPatElements"
        | SyntaxKind.Literal -> "Literal"
        | SyntaxKind.LiteralPat -> "LiteralPat"
        | SyntaxKind.MatchCase -> "MatchCase"
        | SyntaxKind.MatchCaseList -> "MatchCaseList"
        | SyntaxKind.MatchExpr -> "MatchExpr"
        | SyntaxKind.MatchGuard -> "MatchGuard"
        | SyntaxKind.ModuleDecl -> "ModuleDecl"
        | SyntaxKind.ModuleDeclList -> "ModuleDeclList"
        | SyntaxKind.ModulePreamble -> "ModulePreamble"
        | SyntaxKind.ModuleRoot -> "ModuleRoot"
        | SyntaxKind.Name -> "Name"
        | SyntaxKind.NamePatField -> "NamePatField"
        | SyntaxKind.NamedPat -> "NamedPat"
        | SyntaxKind.OpenDecl -> "OpenDecl"
        | SyntaxKind.Operator -> "Operator"
        | SyntaxKind.OrPat -> "OrPat"
        | SyntaxKind.ParenExpr -> "ParenExpr"
        | SyntaxKind.ParenPat -> "ParenPat"
        | SyntaxKind.ParenTy -> "ParenTy"
        | SyntaxKind.Pat -> "Pat"
        | SyntaxKind.QName -> "QName"
        | SyntaxKind.QNameSegment -> "QNameSegment"
        | SyntaxKind.QTy -> "QTy"
        | SyntaxKind.RecordFields -> "RecordFields"
        | SyntaxKind.RecordPat -> "RecordPat"
        | SyntaxKind.SeqExpr -> "SeqExpr"
        | SyntaxKind.StringLiteral -> "StringLiteral"
        | SyntaxKind.TupleExpr -> "TupleExpr"
        | SyntaxKind.TuplePat -> "TuplePat"
        | SyntaxKind.TupleTy -> "TupleTy"
        | SyntaxKind.Ty -> "Ty"
        | SyntaxKind.TypedExpr -> "TypedExpr"
        | SyntaxKind.TypedPat -> "TypedPat"
        | SyntaxKind.UnaryExpr -> "UnaryExpr"
        | SyntaxKind.UnitLiteral -> "UnitLiteral"
        | SyntaxKind.WildPat -> "WildPat"
        | _ -> string (uint8 kind)

let rec private mapGenericTree (t: GenericTree) : Node =
    match t with
    | Token (kind, text, range, _, _) ->
        let kindNameStr = kindName kind
        { name = text
          ``type`` = kindNameStr
          range = rangeToTuple range
          children = [] }
    | Node (kind, children, range) ->
        let kindNameStr = kindName kind
        { name = kindNameStr
          ``type`` = kindNameStr
          range = rangeToTuple range
          children = children |> List.map mapGenericTree }

let buildAstFromSource (code: string) : AstResponse =
    let events, trivias, diagnostics = parseRoot code
    let sink = LosslessTreeSink(code, trivias)
    ParseEvent.processEvents sink diagnostics events
    let treeOpt = sink.Finish()
    { diagnostics = diagnostics |> List.map mapDiagnostic
      tree = treeOpt |> Option.map mapGenericTree }
