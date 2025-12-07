namespace Stdx

[<AutoOpen>]
module Operators =
    [<CompiledName("Konst")>]
    let inline konst x _ = x

    [<CompiledName("Flip")>]
    let inline flip f b a = f a b


module ValueOption =
    [<CompiledName("MapOrZero")>]
    let inline mapOrZero ([<InlineIfLambda>] f : 'a -> 'b) (vopt : 'a voption) : 'b =
        match vopt with
        | ValueSome v -> f v
        | ValueNone -> LanguagePrimitives.GenericZero

module VOption = ValueOption

module Dictionary =
    open System.Collections.Generic

    let inline tryGetValue (key : 'K) (dict : Dictionary<'K, 'V>) : 'V option =
        match dict.TryGetValue key with
        | true, value -> Some value
        | false, _ -> None

    let inline tryGetValueV (key : 'K) (dict : Dictionary<'K, 'V>) : 'V voption =
        match dict.TryGetValue key with
        | true, value -> ValueSome value
        | false, _ -> ValueNone

    let inline getOrAdd (key : 'K) ([<InlineIfLambda>] valueFactory : unit -> 'V) (dict : Dictionary<'K, 'V>) : 'V =
        match dict.TryGetValue key with
        | true, value -> value
        | false, _ ->
            let value = valueFactory ()
            dict[key] <- value
            value
