namespace MLK.Compiler.Syntax

type SyntaxKind =
    | Eof = 0uy
    | Amp = 1uy
    | LParen = 2uy
    | RParen = 3uy
    | Star = 4uy
    | Comma = 5uy
    | Arrow = 6uy
    | Dot = 7uy
    | Colon = 8uy
    | Colon2 = 9uy
    | Semicolon = 10uy
    | Eq = 11uy
    | LBracket = 12uy
    | RBracket = 13uy
    | Underscore = 14uy
    | LBrace = 15uy
    | Pipe = 16uy
    | RBrace = 17uy
    | AndKw = 18uy
    | AsKw = 19uy
    | ElseKw = 20uy
    | FalseKw = 21uy
    | FunKw = 22uy
    | IfKw = 23uy
    | InKw = 24uy
    | LetKw = 25uy
    | MatchKw = 26uy
    | ModuleKw = 27uy
    | OpenKw = 28uy
    | RecKw = 29uy
    | ThenKw = 30uy
    | TrueKw = 31uy
    | WhenKw = 32uy
    | WithKw = 33uy
    | Comment = 34uy
    | ErrToken = 35uy
    | Ident = 36uy
    | MultilineComment = 37uy
    | Newline = 38uy
    | Whitespace = 39uy
    | AndPat = 40uy
    | AppExpr = 41uy
    | ArgPat = 42uy
    | ArgPats = 43uy
    | AsPat = 44uy
    | Binding = 45uy
    | BindingList = 46uy
    | BoolLiteral = 47uy
    | CharLiteral = 48uy
    | ConsPat = 49uy
    | ErrDecl = 50uy
    | ErrExpr = 51uy
    | ErrNode = 52uy
    | ErrPat = 53uy
    | ErrTy = 54uy
    | Expr = 55uy
    | FnTy = 56uy
    | FunExpr = 57uy
    | FuncPat = 58uy
    | IfExpr = 59uy
    | InferTy = 60uy
    | InnerModuleDecl = 61uy
    | IntLiteral = 62uy
    | LetDecl = 63uy
    | LetExpr = 64uy
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
    | OrPat = 81uy
    | ParenExpr = 82uy
    | ParenPat = 83uy
    | ParenTy = 84uy
    | Pat = 85uy
    | QName = 86uy
    | QNameSegment = 87uy
    | QTy = 88uy
    | RecordFields = 89uy
    | RecordPat = 90uy
    | SeqExpr = 91uy
    | StringLiteral = 92uy
    | TupleExpr = 93uy
    | TuplePat = 94uy
    | TupleTy = 95uy
    | Ty = 96uy
    | TypedExpr = 97uy
    | TypedPat = 98uy
    | UnitLiteral = 99uy
    | WildPat = 100uy

module SyntaxKind =
    let isPunct (kind : SyntaxKind) : bool =
        match kind with
        | SyntaxKind.Amp
        | SyntaxKind.LParen
        | SyntaxKind.RParen
        | SyntaxKind.Star
        | SyntaxKind.Comma
        | SyntaxKind.Arrow
        | SyntaxKind.Dot
        | SyntaxKind.Colon
        | SyntaxKind.Colon2
        | SyntaxKind.Semicolon
        | SyntaxKind.Eq
        | SyntaxKind.LBracket
        | SyntaxKind.RBracket
        | SyntaxKind.Underscore
        | SyntaxKind.LBrace
        | SyntaxKind.Pipe
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
        | "&" -> SyntaxKind.Amp
        | "(" -> SyntaxKind.LParen
        | ")" -> SyntaxKind.RParen
        | "*" -> SyntaxKind.Star
        | "," -> SyntaxKind.Comma
        | "->" -> SyntaxKind.Arrow
        | "." -> SyntaxKind.Dot
        | ":" -> SyntaxKind.Colon
        | "::" -> SyntaxKind.Colon2
        | ";" -> SyntaxKind.Semicolon
        | "=" -> SyntaxKind.Eq
        | "[" -> SyntaxKind.LBracket
        | "]" -> SyntaxKind.RBracket
        | "_" -> SyntaxKind.Underscore
        | "{" -> SyntaxKind.LBrace
        | "|" -> SyntaxKind.Pipe
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
