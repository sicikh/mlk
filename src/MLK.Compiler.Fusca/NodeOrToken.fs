namespace MLK.Compiler.Fusca

[<Struct>]
type RawSyntaxKind =
    | RawSyntaxKind of uint16

    member this.Value =
        let (RawSyntaxKind v) = this
        v

[<RequireQualifiedAccess>]
type NodeOrToken<'N, 'T> =
    | Node of node : 'N
    | Token of token : 'T

    override this.ToString () =
        match this with
        | Node n -> n.ToString ()
        | Token t -> t.ToString ()

module NodeOrToken =
    let asNode self =
        match self with
        | NodeOrToken.Node n -> Some n
        | NodeOrToken.Token _ -> None

    let asToken self =
        match self with
        | NodeOrToken.Node _ -> None
        | NodeOrToken.Token t -> Some t

    let map onNode onToken self =
        match self with
        | NodeOrToken.Node n -> n |> onNode |> NodeOrToken.Node
        | NodeOrToken.Token t -> t |> onToken |> NodeOrToken.Token

    let consolidate onNode onToken self =
        match self with
        | NodeOrToken.Node n -> n |> onNode
        | NodeOrToken.Token t -> t |> onToken
