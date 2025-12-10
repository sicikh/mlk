namespace MLK.Compiler.Syntax

type SyntaxKind =
    | Tombstone = 0uy
    | Eof = 1uy
    | LParen = 2uy
    | RParen = 3uy
    | Comma = 4uy
    | Arrow = 5uy
    | Dot = 6uy
    | Colon = 7uy
    | Semicolon = 8uy
    | LBracket = 9uy
    | RBracket = 10uy
    | Underscore = 11uy
    | LBrace = 12uy
    | RBrace = 13uy
    | AndKw = 14uy
    | AsKw = 15uy
    | ElseKw = 16uy
    | FalseKw = 17uy
    | FunKw = 18uy
    | IfKw = 19uy
    | InKw = 20uy
    | LetKw = 21uy
    | MatchKw = 22uy
    | ModuleKw = 23uy
    | OpenKw = 24uy
    | RecKw = 25uy
    | ThenKw = 26uy
    | TrueKw = 27uy
    | WhenKw = 28uy
    | WithKw = 29uy
    | Comment = 30uy
    | ErrToken = 31uy
    | Ident = 32uy
    | MultilineComment = 33uy
    | Newline = 34uy
    | Op = 35uy
    | Whitespace = 36uy
    | AndPat = 37uy
    | AppExpr = 38uy
    | ArgPat = 39uy
    | ArgPats = 40uy
    | AsPat = 41uy
    | BinExpr = 42uy
    | Binding = 43uy
    | BindingList = 44uy
    | BoolLiteral = 45uy
    | CharLiteral = 46uy
    | ConsPat = 47uy
    | ErrDecl = 48uy
    | ErrExpr = 49uy
    | ErrNode = 50uy
    | ErrPat = 51uy
    | ErrTy = 52uy
    | Expr = 53uy
    | FnTy = 54uy
    | FunExpr = 55uy
    | FuncPat = 56uy
    | IfExpr = 57uy
    | InferTy = 58uy
    | InnerModuleDecl = 59uy
    | IntLiteral = 60uy
    | LetDecl = 61uy
    | LetExpr = 62uy
    | ListExpr = 63uy
    | ListExprElements = 64uy
    | ListPat = 65uy
    | ListPatElements = 66uy
    | Literal = 67uy
    | LiteralPat = 68uy
    | MatchCase = 69uy
    | MatchCaseList = 70uy
    | MatchExpr = 71uy
    | MatchGuard = 72uy
    | ModuleDecl = 73uy
    | ModuleDeclList = 74uy
    | ModulePreamble = 75uy
    | ModuleRoot = 76uy
    | Name = 77uy
    | NamePatField = 78uy
    | NamedPat = 79uy
    | OpenDecl = 80uy
    | Operator = 81uy
    | OrPat = 82uy
    | ParenExpr = 83uy
    | ParenPat = 84uy
    | ParenTy = 85uy
    | Pat = 86uy
    | QName = 87uy
    | QNameSegment = 88uy
    | QTy = 89uy
    | RecordFields = 90uy
    | RecordPat = 91uy
    | SeqExpr = 92uy
    | StringLiteral = 93uy
    | TupleExpr = 94uy
    | TuplePat = 95uy
    | TupleTy = 96uy
    | Ty = 97uy
    | TypedExpr = 98uy
    | TypedPat = 99uy
    | UnaryExpr = 100uy
    | UnitLiteral = 101uy
    | WildPat = 102uy

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
