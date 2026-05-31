module MLK.Compiler.Fusca.Rest

[<RequireQualifiedAccess>]
type TokenAtOffset<'T> =
    | None
    | Single of token : 'T
    | Between of left : 'T * right : 'T

module TokenAtOffset =
    let map f token =
        match token with
        | TokenAtOffset.None -> TokenAtOffset.None
        | TokenAtOffset.Single t -> TokenAtOffset.Single (f t)
        | TokenAtOffset.Between (l, r) -> TokenAtOffset.Between (f l, f r)

    let rightBiased token =
        match token with
        | TokenAtOffset.None -> TokenAtOffset.None
        | TokenAtOffset.Single t -> TokenAtOffset.Single t
        | TokenAtOffset.Between (_, r) -> TokenAtOffset.Single r

    let leftBiased token =
        match token with
        | TokenAtOffset.None -> TokenAtOffset.None
        | TokenAtOffset.Single t -> TokenAtOffset.Single t
        | TokenAtOffset.Between (l, _) -> TokenAtOffset.Single l

    let toSeq token =
        match token with
        | TokenAtOffset.None -> Seq.empty
        | TokenAtOffset.Single t -> Seq.singleton t
        | TokenAtOffset.Between (l, r) ->
            seq {
                yield l
                yield r
            }
