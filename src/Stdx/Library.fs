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

    let inline tryGetValue (key : 'K) (dict : IDictionary<'K, 'V>) : 'V option =
        match dict.TryGetValue key with
        | true, value -> Some value
        | false, _ -> None

    let inline tryGetValueV (key : 'K) (dict : IDictionary<'K, 'V>) : 'V voption =
        match dict.TryGetValue key with
        | true, value -> ValueSome value
        | false, _ -> ValueNone

    let inline getOrAdd (key : 'K) ([<InlineIfLambda>] valueFactory : unit -> 'V) (dict : IDictionary<'K, 'V>) : 'V =
        match dict.TryGetValue key with
        | true, value -> value
        | false, _ ->
            let value = valueFactory ()
            dict[key] <- value
            value

module Option =
    let sequence (xs : 'a option seq) : 'a list option =
        let folder (state : 'a list option) (elem : 'a option) : 'a list option =
            match state, elem with
            | Some acc, Some v -> Some (v :: acc)
            | _, _ -> None

        Seq.fold folder (Some []) xs |> Option.map List.rev

    let bind2 (f : 'a -> 'b -> 'c option) (a : 'a option) (b : 'b option) : 'c option =
        match a, b with
        | Some a, Some b -> f a b
        | _ -> None

    let bindWithContext (f : 'a -> 'b option) (a : 'a option) : ('a * 'b) option =
        match a with
        | Some a -> f a |> Option.map (fun b -> a, b)
        | _ -> None

// let traverseList (f : 'a -> 'b option) (list : 'a list) : 'b list option =
//     let folder head tail = f head |> Option.bind (fun head -> tail |> Option.bind (fun tail -> Some (head :: tail)))
//     List.foldBack folder list (Some [])
//
// let sequenceList (list : 'a option list) : 'a list option =
//     traverseList id list


type ControlFlow<'a, 'b> =
    | Continue of 'a
    | Break of 'b

    member x.isContinue =
        match x with
        | Continue _ -> true
        | Break _ -> false

    member x.isBreak =
        match x with
        | Continue _ -> false
        | Break _ -> true

    member x.ContinueValue =
        match x with
        | Continue v -> v
        | Break _ -> failwith "not a continue value"

    member x.BreakValue =
        match x with
        | Break v -> v
        | Continue _ -> failwith "not a break value"

module ControlFlow =
    let toResult (cf : ControlFlow<'a, 'b>) =
        match cf with
        | Continue v -> Ok v
        | Break e -> Error e

    let fromResult (r : Result<'a, 'b>) =
        match r with
        | Ok v -> Continue v
        | Error e -> Break e

    let toOption (cf : ControlFlow<'a, 'b>) =
        match cf with
        | Continue v -> Some v
        | Break _ -> None

    let fromOption (o : 'a option) =
        match o with
        | Some v -> Continue v
        | None -> Break ()

    let unwrap (onBreak : 'b -> 'a) (cf : ControlFlow<'a, 'b>) =
        match cf with
        | Continue v -> v
        | Break e -> onBreak e

module Seq =
    let tryFold (folder : 's -> 'a -> ControlFlow<'s, 'e>) (state : 's) (source : 'a seq) : ControlFlow<'s, 'e> =
        use e = source.GetEnumerator ()
        let f = OptimizedClosures.FSharpFunc<_, _, _>.Adapt folder
        let mutable state = Continue state

        while e.MoveNext () && state.isContinue do
            state <- f.Invoke (state.ContinueValue, e.Current)

        state

    let tryFold2
        (folder : 's -> 'a -> 'b -> ControlFlow<'s, 'e>)
        (state : 's)
        (source1 : 'a seq)
        (source2 : 'b seq)
        : ControlFlow<'s, 'e>
        =
        use e1 = source1.GetEnumerator ()
        use e2 = source2.GetEnumerator ()

        let f = OptimizedClosures.FSharpFunc<_, _, _, _>.Adapt folder

        let mutable state = Continue state

        while e1.MoveNext () && e2.MoveNext () && state.isContinue do
            state <- f.Invoke (state.ContinueValue, e1.Current, e2.Current)

        state

    let tryIter action source = tryFold (fun _ -> action) () source

    let tryFoldResult folder state source =
        tryFold (fun s i -> folder s i |> ControlFlow.fromResult) state source
        |> ControlFlow.toResult

    let tryFoldOption folder state source =
        tryFold (fun s i -> folder s i |> ControlFlow.fromOption) state source
        |> ControlFlow.toOption

    let tryFold2Option folder state source1 source2 =
        tryFold2 (fun s i1 i2 -> folder s i1 i2 |> ControlFlow.fromOption) state source1 source2
        |> ControlFlow.toOption

    /// Compare two sequences with the given predicate. If sequences have different lengths, returns false.
    let forall2Eq (predicate : 'a -> 'b -> bool) (source1 : 'a seq) (source2 : 'b seq) =
        use e1 = source1.GetEnumerator ()
        use e2 = source2.GetEnumerator ()

        let mutable areEqual = true

        while areEqual && e1.MoveNext () && e2.MoveNext () do
            areEqual <- predicate e1.Current e2.Current

        areEqual && not (e1.MoveNext () || e2.MoveNext ())

    let assoc (key : 'K) (pairs : seq<'K * 'V>) : 'V option =
        pairs |> Seq.tryFind (fun (k, _) -> k = key) |> Option.map snd

module List =
    let assoc (key : 'K) (pairs : list<'K * 'V>) : 'V option =
        pairs |> List.tryFind (fun (k, _) -> k = key) |> Option.map snd

module String =
    // TODO: Rewrite to handle more cases (see biome-text-case crate)
    let toPascalCase (s : string) : string =
        if System.String.IsNullOrEmpty s then
            s
        else
            let firstChar = s[0]
            let upperFirstChar = System.Char.ToUpperInvariant firstChar

            if s.Length = 1 then
                string upperFirstChar
            else
                upperFirstChar.ToString () + s[1..]
