namespace MLK.Compiler.Syntax

type SyntaxKind =
    | Eof = 0uy
    | LParen = 1uy
    | RParen = 2uy
    | Comma = 3uy
    | Arrow = 4uy
    | Dot = 5uy
    | Colon = 6uy
    | Semicolon = 7uy
    | LBracket = 8uy
    | RBracket = 9uy
    | Underscore = 10uy
    | LBrace = 11uy
    | RBrace = 12uy
    | AndKw = 13uy
    | AsKw = 14uy
    | ElseKw = 15uy
    | FalseKw = 16uy
    | FunKw = 17uy
    | IfKw = 18uy
    | InKw = 19uy
    | LetKw = 20uy
    | MatchKw = 21uy
    | ModuleKw = 22uy
    | OpenKw = 23uy
    | RecKw = 24uy
    | ThenKw = 25uy
    | TrueKw = 26uy
    | WhenKw = 27uy
    | WithKw = 28uy
    | Comment = 29uy
    | ErrToken = 30uy
    | Ident = 31uy
    | MultilineComment = 32uy
    | Newline = 33uy
    | Whitespace = 34uy
    | AndPat = 35uy
    | AppExpr = 36uy
    | ArgPat = 37uy
    | ArgPats = 38uy
    | AsPat = 39uy
    | Binding = 40uy
    | BindingList = 41uy
    | BoolLiteral = 42uy
    | CharLiteral = 43uy
    | ConsPat = 44uy
    | ErrDecl = 45uy
    | ErrExpr = 46uy
    | ErrNode = 47uy
    | ErrPat = 48uy
    | ErrTy = 49uy
    | Expr = 50uy
    | FnTy = 51uy
    | FunExpr = 52uy
    | FuncPat = 53uy
    | IfExpr = 54uy
    | InferTy = 55uy
    | InnerModuleDecl = 56uy
    | IntLiteral = 57uy
    | LetDecl = 58uy
    | LetExpr = 59uy
    | ListExpr = 60uy
    | ListExprElements = 61uy
    | ListPat = 62uy
    | ListPatElements = 63uy
    | Literal = 64uy
    | LiteralPat = 65uy
    | MatchCase = 66uy
    | MatchCaseList = 67uy
    | MatchExpr = 68uy
    | MatchGuard = 69uy
    | ModuleDecl = 70uy
    | ModuleDeclList = 71uy
    | ModulePreamble = 72uy
    | ModuleRoot = 73uy
    | Name = 74uy
    | NamePatField = 75uy
    | NamedPat = 76uy
    | OpenDecl = 77uy
    | OrPat = 78uy
    | ParenExpr = 79uy
    | ParenPat = 80uy
    | ParenTy = 81uy
    | Pat = 82uy
    | QName = 83uy
    | QNameSegment = 84uy
    | QTy = 85uy
    | RecordFields = 86uy
    | RecordPat = 87uy
    | SeqExpr = 88uy
    | StringLiteral = 89uy
    | TupleExpr = 90uy
    | TuplePat = 91uy
    | TupleTy = 92uy
    | Ty = 93uy
    | TypedExpr = 94uy
    | TypedPat = 95uy
    | UnitLiteral = 96uy
    | WildPat = 97uy

module SyntaxKind =
    let isPunct (kind : SyntaxKind) : bool =
        match kind with
        | SyntaxKind.LParen
        | SyntaxKind.RParen
        | SyntaxKind.Comma
        | SyntaxKind.Arrow
        | SyntaxKind.Dot
        | SyntaxKind.Colon
        | SyntaxKind.Semicolon
        | SyntaxKind.LBracket
        | SyntaxKind.RBracket
        | SyntaxKind.Underscore
        | SyntaxKind.LBrace
        | SyntaxKind.RBrace -> true 
        | _ -> false

    let isLiteral (kind : SyntaxKind) : bool =
        match kind with
        | SyntaxKind.BoolLiteral
        | SyntaxKind.CharLiteral
        | SyntaxKind.IntLiteral
        | SyntaxKind.StringLiteral -> true 
        | _ -> false

    let isKeyword (kind : SyntaxKind) : bool =
        match kind with
        | SyntaxKind.AndKw
        | SyntaxKind.AsKw
        | SyntaxKind.ElseKw
        | SyntaxKind.FalseKw
        | SyntaxKind.FunKw
        | SyntaxKind.IfKw
        | SyntaxKind.InKw
        | SyntaxKind.LetKw
        | SyntaxKind.MatchKw
        | SyntaxKind.ModuleKw
        | SyntaxKind.OpenKw
        | SyntaxKind.RecKw
        | SyntaxKind.ThenKw
        | SyntaxKind.TrueKw
        | SyntaxKind.WhenKw
        | SyntaxKind.WithKw -> true 
        | _ -> false

    let isList (kind : SyntaxKind) : bool =
        match kind with
        | SyntaxKind.ArgPats
        | SyntaxKind.BindingList
        | SyntaxKind.ListExprElements
        | SyntaxKind.ListPatElements
        | SyntaxKind.MatchCaseList
        | SyntaxKind.ModuleDeclList
        | SyntaxKind.RecordFields
        | SyntaxKind.TupleExpr
        | SyntaxKind.TuplePat
        | SyntaxKind.TupleTy -> true 
        | _ -> false

    let fromKeyword (keyword : string) : SyntaxKind voption =
        match keyword with
        | "module" -> ValueSome SyntaxKind.ModuleKw
        | "open" -> ValueSome SyntaxKind.OpenKw
        | "let" -> ValueSome SyntaxKind.LetKw
        | "rec" -> ValueSome SyntaxKind.RecKw
        | "and" -> ValueSome SyntaxKind.AndKw
        | "in" -> ValueSome SyntaxKind.InKw
        | "fun" -> ValueSome SyntaxKind.FunKw
        | "if" -> ValueSome SyntaxKind.IfKw
        | "then" -> ValueSome SyntaxKind.ThenKw
        | "else" -> ValueSome SyntaxKind.ElseKw
        | "match" -> ValueSome SyntaxKind.MatchKw
        | "with" -> ValueSome SyntaxKind.WithKw
        | "when" -> ValueSome SyntaxKind.WhenKw
        | "as" -> ValueSome SyntaxKind.AsKw
        | "true" -> ValueSome SyntaxKind.TrueKw
        | "false" -> ValueSome SyntaxKind.FalseKw
        | _ -> ValueNone

    let toString (kind : SyntaxKind) : string voption =
        match kind with
        | SyntaxKind.ModuleKw -> ValueSome "module"
        | SyntaxKind.OpenKw -> ValueSome "open"
        | SyntaxKind.LetKw -> ValueSome "let"
        | SyntaxKind.RecKw -> ValueSome "rec"
        | SyntaxKind.AndKw -> ValueSome "and"
        | SyntaxKind.InKw -> ValueSome "in"
        | SyntaxKind.FunKw -> ValueSome "fun"
        | SyntaxKind.IfKw -> ValueSome "if"
        | SyntaxKind.ThenKw -> ValueSome "then"
        | SyntaxKind.ElseKw -> ValueSome "else"
        | SyntaxKind.MatchKw -> ValueSome "match"
        | SyntaxKind.WithKw -> ValueSome "with"
        | SyntaxKind.WhenKw -> ValueSome "when"
        | SyntaxKind.AsKw -> ValueSome "as"
        | SyntaxKind.TrueKw -> ValueSome "true"
        | SyntaxKind.FalseKw -> ValueSome "false"
        | SyntaxKind.Eof -> ValueSome "EOF"
        | SyntaxKind.StringLiteral -> ValueSome "string literal"
        | _ -> ValueNone

[<AutoOpen>]
module SyntaxKindOps =
    // open System.Runtime.CompilerServices

    // [<MethodImpl(MethodImplOptions.AggressiveInlining)>]
    let inline t (str : string) : SyntaxKind =
        match str with
        | "(" -> SyntaxKind.LParen
        | ")" -> SyntaxKind.RParen
        | "," -> SyntaxKind.Comma
        | "->" -> SyntaxKind.Arrow
        | "." -> SyntaxKind.Dot
        | ":" -> SyntaxKind.Colon
        | ";" -> SyntaxKind.Semicolon
        | "[" -> SyntaxKind.LBracket
        | "]" -> SyntaxKind.RBracket
        | "_" -> SyntaxKind.Underscore
        | "{" -> SyntaxKind.LBrace
        | "}" -> SyntaxKind.RBrace
        | "module" -> SyntaxKind.ModuleKw
        | "open" -> SyntaxKind.OpenKw
        | "let" -> SyntaxKind.LetKw
        | "rec" -> SyntaxKind.RecKw
        | "and" -> SyntaxKind.AndKw
        | "in" -> SyntaxKind.InKw
        | "fun" -> SyntaxKind.FunKw
        | "if" -> SyntaxKind.IfKw
        | "then" -> SyntaxKind.ThenKw
        | "else" -> SyntaxKind.ElseKw
        | "match" -> SyntaxKind.MatchKw
        | "with" -> SyntaxKind.WithKw
        | "when" -> SyntaxKind.WhenKw
        | "as" -> SyntaxKind.AsKw
        | "true" -> SyntaxKind.TrueKw
        | "false" -> SyntaxKind.FalseKw
        | "EOF" -> SyntaxKind.Eof
        | "ident" -> SyntaxKind.Ident
        | s -> failwithf "Unknown syntax kind: %s" s

    let (|Punct|_|) (kind : SyntaxKind) : bool = SyntaxKind.isPunct kind
    let (|Literal|_|) (kind : SyntaxKind) : bool = SyntaxKind.isLiteral kind
    let (|Keyword|_|) (kind : SyntaxKind) : bool = SyntaxKind.isKeyword kind
    let (|List|_|) (kind : SyntaxKind) : bool = SyntaxKind.isList kind
    let (|T|_|) (str : string) (sk : SyntaxKind) : bool = t str = sk
