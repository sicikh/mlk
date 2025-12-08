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
    | IfKw = 22uy
    | InKw = 23uy
    | LetKw = 24uy
    | MatchKw = 25uy
    | ModuleKw = 26uy
    | OpenKw = 27uy
    | RecKw = 28uy
    | ThenKw = 29uy
    | TrueKw = 30uy
    | WhenKw = 31uy
    | WithKw = 32uy
    | Comment = 33uy
    | ErrToken = 34uy
    | Ident = 35uy
    | MultilineComment = 36uy
    | Newline = 37uy
    | Whitespace = 38uy
    | AndPat = 39uy
    | AppExpr = 40uy
    | ArgPat = 41uy
    | ArgPats = 42uy
    | AsPat = 43uy
    | Binding = 44uy
    | BindingList = 45uy
    | BoolLiteral = 46uy
    | CharLiteral = 47uy
    | ConsPat = 48uy
    | ErrDecl = 49uy
    | ErrExpr = 50uy
    | ErrNode = 51uy
    | ErrPat = 52uy
    | ErrTy = 53uy
    | Expr = 54uy
    | FnTy = 55uy
    | FuncPat = 56uy
    | IfExpr = 57uy
    | InferTy = 58uy
    | InnerModuleDecl = 59uy
    | IntLiteral = 60uy
    | LetDecl = 61uy
    | LetExpr = 62uy
    | ListPat = 63uy
    | ListPatElements = 64uy
    | Literal = 65uy
    | LiteralPat = 66uy
    | MatchCase = 67uy
    | MatchCaseList = 68uy
    | MatchExpr = 69uy
    | MatchGuard = 70uy
    | ModuleDecl = 71uy
    | ModuleDeclList = 72uy
    | ModulePreamble = 73uy
    | ModuleRoot = 74uy
    | Name = 75uy
    | NamePatField = 76uy
    | NamedPat = 77uy
    | OpenDecl = 78uy
    | OrPat = 79uy
    | ParenExpr = 80uy
    | ParenPat = 81uy
    | ParenTy = 82uy
    | Pat = 83uy
    | QName = 84uy
    | QNameSegment = 85uy
    | QTy = 86uy
    | RecordFields = 87uy
    | RecordPat = 88uy
    | SeqExpr = 89uy
    | StringLiteral = 90uy
    | TupleExpr = 91uy
    | TuplePat = 92uy
    | TupleTy = 93uy
    | Ty = 94uy
    | TypedExpr = 95uy
    | TypedPat = 96uy
    | UnitLiteral = 97uy
    | WildPat = 98uy

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
