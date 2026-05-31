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
        let rec aux acc slots children =
            match slots with
            | [] -> List.rev acc
            | SlotPresent :: rest ->
                match children with
                | child :: restChildren ->
                    aux (Some child :: acc) rest restChildren
                | [] -> failwith "Expected a present node according to the slot description"
            | SlotAbsent :: rest ->
                aux (None :: acc) rest children

        aux [] (List.rev slots) children |> GreenNode.mk kind
