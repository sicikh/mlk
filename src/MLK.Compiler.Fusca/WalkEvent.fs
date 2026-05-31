namespace MLK.Compiler.Fusca

[<Struct>]
type Direction =
    | Next
    | Prev

type WalkEvent<'T> =
    | Enter of 'T
    | Leave of 'T

module WalkEvent =
    let map f event =
        match event with
        | Enter n -> Enter (f n)
        | Leave n -> Leave (f n)
