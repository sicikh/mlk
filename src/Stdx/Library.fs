namespace Stdx

[<AutoOpen>]
module Utils =
    let inline konst x _ = x

module ValueOption =
    let inline mapOrZero<'a, 'b when 'b : (static member Zero : 'b)> (f : 'a -> 'b) (vopt : 'a voption) : 'b =
        match vopt with
        | ValueSome v -> f v
        | ValueNone -> 'b.Zero
