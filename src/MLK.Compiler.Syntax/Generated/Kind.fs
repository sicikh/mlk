namespace MLK.Compiler.Syntax

open MLK.Compiler.Fusca

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
    | BoolLiteral = 35uy
    | CharLiteral = 36uy
    | IntLiteral = 37uy
    | StringLiteral = 38uy
    | Comment = 39uy
    | ErrToken = 40uy
    | Ident = 41uy
    | MultilineComment = 42uy
    | Newline = 43uy
    | Op = 44uy
    | Whitespace = 45uy
    | AndPat = 46uy
    | AppExpr = 47uy
    | ArgPats = 48uy
    | AsPat = 49uy
    | BinExpr = 50uy
    | Binding = 51uy
    | BoolLiteralExpr = 52uy
    | CharLiteralExpr = 53uy
    | ConsPat = 54uy
    | ErrDecl = 55uy
    | ErrExpr = 56uy
    | ErrNode = 57uy
    | ErrPat = 58uy
    | ErrTy = 59uy
    | FnTy = 60uy
    | FunExpr = 61uy
    | FuncPat = 62uy
    | IfExpr = 63uy
    | InferTy = 64uy
    | InnerModuleDecl = 65uy
    | IntLiteralExpr = 66uy
    | LetDecl = 67uy
    | LetExpr = 68uy
    | ListExpr = 69uy
    | ListExprElements = 70uy
    | ListPat = 71uy
    | ListPatElements = 72uy
    | MatchCase = 73uy
    | MatchCaseList = 74uy
    | MatchExpr = 75uy
    | MatchGuard = 76uy
    | MemberAccessExpr = 77uy
    | ModuleDeclList = 78uy
    | ModulePreamble = 79uy
    | ModuleRoot = 80uy
    | Name = 81uy
    | NamePatField = 82uy
    | NamedPat = 83uy
    | OpenDecl = 84uy
    | Operator = 85uy
    | OrPat = 86uy
    | ParenExpr = 87uy
    | ParenPat = 88uy
    | ParenTy = 89uy
    | QName = 90uy
    | QNameSegment = 91uy
    | QTy = 92uy
    | RecordFields = 93uy
    | RecordPat = 94uy
    | SeqExpr = 95uy
    | StringLiteralExpr = 96uy
    | TupleExpr = 97uy
    | TuplePat = 98uy
    | TupleTy = 99uy
    | TypedExpr = 100uy
    | TypedPat = 101uy
    | UnaryExpr = 102uy
    | UnitLiteralExpr = 103uy
    | VarExpr = 104uy
    | WildPat = 105uy

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

    let toErr (_kind : SyntaxKind) : SyntaxKind =
        // TODO: implement this properly
        SyntaxKind.ErrNode


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
