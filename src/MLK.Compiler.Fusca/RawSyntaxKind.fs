namespace MLK.Compiler.Fusca

[<Struct>]
type RawSyntaxKind =
    | RawSyntaxKind of uint16

    member this.Value =
        let (RawSyntaxKind v) = this
        v
