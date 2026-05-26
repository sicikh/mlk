namespace MLK.Compiler.Fusca

type ParsedChildren = | ParsedChildren of GreenElement list

module ParsedChildren =
    let elements (ParsedChildren children) = children

type SlotContent =
    | SlotPresent
    | SlotAbsent

type RawNodeSlots = | RawNodeSlots of SlotContent list

module RawNodeSlots =
    let empty = RawNodeSlots []

    let addPresent (RawNodeSlots slots) = RawNodeSlots (SlotPresent :: slots)
    let addAbsent (RawNodeSlots slots) = RawNodeSlots (SlotAbsent :: slots)

    let intoNode (kind : RawSyntaxKind) (ParsedChildren children) (RawNodeSlots slots) : GreenNode =
        let mutable children = children

        let slots =
            slots
            |> List.rev
            |> Seq.map (
                function
                | SlotPresent ->
                    match children with
                    | child :: rest ->
                        children <- rest
                        Some child
                    | [] -> failwith "Expected a present node according to the slot description"
                | SlotAbsent -> None
            )

        GreenNode.mk kind slots
