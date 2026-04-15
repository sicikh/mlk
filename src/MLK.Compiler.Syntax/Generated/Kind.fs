namespace MLK.Compiler.Syntax

type SyntaxKind =
    | Tombstone = 0uy
    | Eof = 1uy
    | Amp = 2uy
    | LParen = 3uy
    | RParen = 4uy
    | Star = 5uy
    | Comma = 6uy
    | Arrow = 7uy
    | Dot = 8uy
    | Colon = 9uy
    | Colon2 = 10uy
    | Semicolon = 11uy
    | Eq = 12uy
    | LBracket = 13uy
    | RBracket = 14uy
    | Underscore = 15uy
    | LBrace = 16uy
    | Pipe = 17uy
    | RBrace = 18uy
    | AndKw = 19uy
    | AsKw = 20uy
    | ElseKw = 21uy
    | FalseKw = 22uy
    | FunKw = 23uy
    | IfKw = 24uy
    | InKw = 25uy
    | LetKw = 26uy
    | MatchKw = 27uy
    | ModuleKw = 28uy
    | OpenKw = 29uy
    | RecKw = 30uy
    | ThenKw = 31uy
    | TrueKw = 32uy
    | WhenKw = 33uy
    | WithKw = 34uy
    | Comment = 35uy
    | ErrToken = 36uy
    | Ident = 37uy
    | MultilineComment = 38uy
    | Newline = 39uy
    | Op = 40uy
    | Whitespace = 41uy
    | AndPat = 42uy
    | AppExpr = 43uy
    | ArgPat = 44uy
    | ArgPats = 45uy
    | AsPat = 46uy
    | BinExpr = 47uy
    | Binding = 48uy
    | BindingList = 49uy
    | BoolLiteral = 50uy
    | CharLiteral = 51uy
    | ConsPat = 52uy
    | ErrDecl = 53uy
    | ErrExpr = 54uy
    | ErrNode = 55uy
    | ErrPat = 56uy
    | ErrTy = 57uy
    | Expr = 58uy
    | FnTy = 59uy
    | FunExpr = 60uy
    | FuncPat = 61uy
    | IfExpr = 62uy
    | InferTy = 63uy
    | InnerModuleDecl = 64uy
    | IntLiteral = 65uy
    | LetDecl = 66uy
    | LetExpr = 67uy
    | ListExpr = 68uy
    | ListExprElements = 69uy
    | ListPat = 70uy
    | ListPatElements = 71uy
    | Literal = 72uy
    | LiteralPat = 73uy
    | MatchCase = 74uy
    | MatchCaseList = 75uy
    | MatchExpr = 76uy
    | MatchGuard = 77uy
    | ModuleDecl = 78uy
    | ModuleDeclList = 79uy
    | ModulePreamble = 80uy
    | ModuleRoot = 81uy
    | Name = 82uy
    | NamePatField = 83uy
    | NamedPat = 84uy
    | OpenDecl = 85uy
    | Operator = 86uy
    | OrPat = 87uy
    | ParenExpr = 88uy
    | ParenPat = 89uy
    | ParenTy = 90uy
    | Pat = 91uy
    | QName = 92uy
    | QNameSegment = 93uy
    | QTy = 94uy
    | RecordFields = 95uy
    | RecordPat = 96uy
    | SeqExpr = 97uy
    | StringLiteral = 98uy
    | TupleExpr = 99uy
    | TuplePat = 100uy
    | TupleTy = 101uy
    | Ty = 102uy
    | TypedExpr = 103uy
    | TypedPat = 104uy
    | UnaryExpr = 105uy
    | UnitLiteral = 106uy
    | WildPat = 107uy

module SyntaxKind =
    let fromRaw (raw : RawSyntaxKind) : SyntaxKind =
        let v = byte raw.Value
        if v <= LanguagePrimitives.EnumToValue SyntaxKind.WildPat then
            LanguagePrimitives.EnumOfValue v
        else
            failwith "Invalid raw SyntaxKind."

    let toRaw (kind : SyntaxKind) : RawSyntaxKind =
        RawSyntaxKind (uint16 (byte kind))

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
