namespace MLK.Compiler.Syntax

type SyntaxKind =
    | Eof = 0uy
    | Dot = 1uy
    | Comma = 2uy
    | Colon = 3uy
    | Colon2 = 4uy
    | Semicolon = 5uy
    | Eq = 6uy
    | LParen = 7uy
    | RParen = 8uy
    | LBrace = 9uy
    | RBrace = 10uy
    | LBracket = 11uy
    | RBracket = 12uy
    | Pipe = 13uy
    | Amp = 14uy
    | Arrow = 15uy
    | Star = 16uy
    | Underscore = 17uy
    | ModuleKw = 18uy
    | OpenKw = 19uy
    | LetKw = 20uy
    | RecKw = 21uy
    | AndKw = 22uy
    | InKw = 23uy
    | IfKw = 24uy
    | ThenKw = 25uy
    | ElseKw = 26uy
    | MatchKw = 27uy
    | WithKw = 28uy
    | WhenKw = 29uy
    | AsKw = 30uy
    | TrueKw = 31uy
    | FalseKw = 32uy
    | Ident = 33uy
    | Newline = 34uy
    | Whitespace = 35uy
    | Comment = 36uy
    | MultilineComment = 37uy
    | ErrToken = 38uy
    | ErrTy = 39uy
    | ErrPat = 40uy
    | ErrExpr = 41uy
    | ErrDecl = 42uy
    | ErrNode = 43uy
    | ArgPats = 44uy
    | BindingList = 45uy
    | ListPatElements = 46uy
    | MatchCaseList = 47uy
    | ModuleDeclList = 48uy
    | RecordFields = 49uy
    | TupleExpr = 50uy
    | TuplePat = 51uy
    | TupleTy = 52uy
    | ArgPat = 53uy
    | Ty = 54uy
    | Literal = 55uy
    | Pat = 56uy
    | Expr = 57uy
    | ModuleDecl = 58uy
    | NamePatField = 59uy
    | AndPat = 60uy
    | OrPat = 61uy
    | RecordPat = 62uy
    | TypedPat = 63uy
    | ListPat = 64uy
    | ConsPat = 65uy
    | LiteralPat = 66uy
    | WildPat = 67uy
    | AsPat = 68uy
    | FuncPat = 69uy
    | NamedPat = 70uy
    | ParenPat = 71uy
    | InferTy = 72uy
    | ParenTy = 73uy
    | FnTy = 74uy
    | QTy = 75uy
    | StringLiteral = 76uy
    | CharLiteral = 77uy
    | UnitLiteral = 78uy
    | BoolLiteral = 79uy
    | IntLiteral = 80uy
    | MatchGuard = 81uy
    | MatchCase = 82uy
    | MatchExpr = 83uy
    | SeqExpr = 84uy
    | LetExpr = 85uy
    | IfExpr = 86uy
    | AppExpr = 87uy
    | TypedExpr = 88uy
    | ParenExpr = 89uy
    | Binding = 90uy
    | LetDecl = 91uy
    | OpenDecl = 92uy
    | InnerModuleDecl = 93uy
    | ModulePreamble = 94uy
    | ModuleRoot = 95uy
    | QNameSegment = 96uy
    | QName = 97uy
    | Name = 98uy

module SyntaxKind =
    let isPunct (kind : SyntaxKind) : bool =
        match kind with
        | SyntaxKind.Dot
        | SyntaxKind.Comma
        | SyntaxKind.Colon
        | SyntaxKind.Colon2
        | SyntaxKind.Semicolon
        | SyntaxKind.Eq
        | SyntaxKind.LParen
        | SyntaxKind.RParen
        | SyntaxKind.LBrace
        | SyntaxKind.RBrace
        | SyntaxKind.LBracket
        | SyntaxKind.RBracket
        | SyntaxKind.Pipe
        | SyntaxKind.Amp
        | SyntaxKind.Arrow
        | SyntaxKind.Star
        | SyntaxKind.Underscore -> true 
        | _ -> false

    let isLiteral (kind : SyntaxKind) : bool =
        match kind with
        | SyntaxKind.IntLiteral
        | SyntaxKind.BoolLiteral
        | SyntaxKind.StringLiteral
        | SyntaxKind.CharLiteral -> true 
        | _ -> false

    let isKeyword (kind : SyntaxKind) : bool =
        match kind with
        | SyntaxKind.ModuleKw
        | SyntaxKind.OpenKw
        | SyntaxKind.LetKw
        | SyntaxKind.RecKw
        | SyntaxKind.AndKw
        | SyntaxKind.InKw
        | SyntaxKind.IfKw
        | SyntaxKind.ThenKw
        | SyntaxKind.ElseKw
        | SyntaxKind.MatchKw
        | SyntaxKind.WithKw
        | SyntaxKind.WhenKw
        | SyntaxKind.AsKw
        | SyntaxKind.TrueKw
        | SyntaxKind.FalseKw -> true 
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
        | "." -> SyntaxKind.Dot
        | "," -> SyntaxKind.Comma
        | ":" -> SyntaxKind.Colon
        | "::" -> SyntaxKind.Colon2
        | ";" -> SyntaxKind.Semicolon
        | "=" -> SyntaxKind.Eq
        | "(" -> SyntaxKind.LParen
        | ")" -> SyntaxKind.RParen
        | "{" -> SyntaxKind.LBrace
        | "}" -> SyntaxKind.RBrace
        | "[" -> SyntaxKind.LBracket
        | "]" -> SyntaxKind.RBracket
        | "|" -> SyntaxKind.Pipe
        | "&" -> SyntaxKind.Amp
        | "->" -> SyntaxKind.Arrow
        | "*" -> SyntaxKind.Star
        | "_" -> SyntaxKind.Underscore
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
    let (|T|_|) (str : string) (sk : SyntaxKind) : SyntaxKind = t str = sk
