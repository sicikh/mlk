namespace MLK.Compiler.Fusca

[<Struct>]
type RawSyntaxKind =
    | RawSyntaxKind of uint16

    member this.Value =
        let (RawSyntaxKind v) = this
        v

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
        | Node n -> Some n
        | Token _ -> None

    let asToken self =
        match self with
        | Node _ -> None
        | Token t -> Some t

    let map onNode onToken self =
        match self with
        | Node n -> n |> onNode |> Node
        | Token t -> t |> onToken |> Token

    let consolidate onNode onToken self =
        match self with
        | Node n -> n |> onNode
        | Token t -> t |> onToken
