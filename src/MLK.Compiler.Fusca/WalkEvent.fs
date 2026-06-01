namespace MLK.Compiler.Fusca

[<Struct>]
type Direction =
    | Next
    | Prev

type WalkEvent<'T> =
    | Enter of 'T
    | Leave of 'T

    member this.AsEnter =
        match this with
        | WalkEvent.Enter enter -> Some enter
        | WalkEvent.Leave _ -> None

    member this.AsLeave =
        match this with
        | WalkEvent.Leave leave -> Some leave
        | WalkEvent.Enter _ -> None

module WalkEvent =
    let map f event =
        match event with
        | Enter n -> Enter (f n)
        | Leave n -> Leave (f n)
