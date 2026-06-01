namespace MLK.Compiler.Fusca.Green

open System
open Stdx
open MLK.Compiler.Text
open MLK.Compiler.Fusca

type GreenTrivia =
    {
        Pieces : TriviaPiece array
    }

    static member Create (pieces : TriviaPiece array) = { Pieces = pieces }

    static member Empty = GreenTrivia.Create [||]

    member this.Length = this.Pieces |> Array.sumBy _.Length
    member this.Count = this.Pieces.Length

    member this.Piece (i : int) : TriviaPiece option =
        if i < this.Pieces.Length then Some this.Pieces[i] else None

type GreenToken =
    {
        RawKind : RawSyntaxKind
        Text : string
        Leading : GreenTrivia
        Trailing : GreenTrivia
    }

    static member WithTrivia (rawKind : RawSyntaxKind, text : string, leading : GreenTrivia, trailing : GreenTrivia) =
        {
            RawKind = rawKind
            Text = text
            Leading = leading
            Trailing = trailing
        }

    static member Create (rawKind : RawSyntaxKind, text : string) =
        GreenToken.WithTrivia (rawKind, text, GreenTrivia.Empty, GreenTrivia.Empty)

    member this.TextTrimmed =
        let starts = this.Leading.Length
        let ends = this.Text.Size - this.Trailing.Length
        this.Text.Slice (TextRange.Create (starts, ends))

    member this.Length = this.Text.Size

    member this.LeadingTrailingTotalLength =
        this.Leading.Length, this.Trailing.Length, this.Length


[<RequireQualifiedAccess>]
type Slot =
    | Node of relOffset : TextSize * node : GreenNode
    | Token of relOffset : TextSize * token : GreenToken
    | Empty of relOffset : TextSize

    member this.RelOffset =
        match this with
        | Slot.Node (relOffset, _)
        | Slot.Token (relOffset, _)
        | Slot.Empty relOffset -> relOffset

    member this.Element =
        match this with
        | Slot.Node (_, n) -> Some (GreenNode n)
        | Slot.Token (_, t) -> Some (GreenToken t)
        | Slot.Empty _ -> None

    member this.RelRange =
        this.Element |> Option.mapOrZero _.Length |> TextRange.at this.RelOffset

and [<CustomEquality ; NoComparison>] GreenNode =
    {
        RawKind : RawSyntaxKind
        Length : TextSize
        Slots : Slot array
        Hash : int
    }

    static member Create (rawKind : RawSyntaxKind, slots : GreenElement option seq) =
        let slots, (length, hash) =
            slots
            |> Seq.mapFold
                (fun (relOffset, hash) el ->
                    let hash = HashCode.Combine (hash, el.GetHashCode ())

                    match el with
                    | Some (GreenNode n) -> Slot.Node (relOffset, n), (relOffset + n.Length, hash)
                    | Some (GreenToken t) -> Slot.Token (relOffset, t), (relOffset + t.Length, hash)
                    | None -> Slot.Empty relOffset, (relOffset, hash)
                )
                (TextSize.zero, rawKind.GetHashCode ())

        {
            RawKind = rawKind
            Length = length
            Slots = slots |> Seq.toArray
            Hash = hash
        }

    member this.SlotAtRange (relRange : TextRange) : (int * TextSize * Slot) option =
        let idx =
            this.Slots
            |> Array.binarySearchBy (fun slot ->
                let childRange = slot.RelRange
                TextRange.ordering childRange relRange
            )
            |> Result.unwrapOrElse (fun idx -> max 0 (idx - 1))

        this.Slots
        |> Array.tryItem idx
        |> Option.filter (fun slot -> slot.RelRange.ContainsRange relRange)
        |> Option.map (fun slot -> idx, slot.RelOffset, slot)

    override this.Equals (other : obj) =
        match other with
        | :? GreenNode as other ->
            obj.ReferenceEquals (this, other)
            || (this.Hash = other.Hash
                && this.RawKind = other.RawKind
                && this.Length = other.Length
                && this.Slots.Length = other.Slots.Length
                && Array.forall2 (=) this.Slots other.Slots)
        | _ -> false

    override this.GetHashCode () = this.Hash

and GreenElement =
    | GreenNode of GreenNode
    | GreenToken of GreenToken

    member this.RawKind =
        match this with
        | GreenNode n -> n.RawKind
        | GreenToken t -> t.RawKind

    member this.Length =
        match this with
        | GreenNode n -> n.Length
        | GreenToken t -> t.Length

    member this.Node =
        match this with
        | GreenNode n -> Some n
        | GreenToken _ -> None

    member this.Token =
        match this with
        | GreenToken t -> Some t
        | GreenNode _ -> None

module Slot =
    let map (f : GreenElement -> 'a) (slot : Slot) : 'a option =
        match slot with
        | Slot.Node (_, n) -> Some (f (GreenNode n))
        | Slot.Token (_, t) -> Some (f (GreenToken t))
        | Slot.Empty _ -> None

type Child =
    internal
    | Child of element : GreenElement * slot : int * relOffset : TextSize

    static member Create (index : int, slot : Slot) =
        match slot with
        | Slot.Empty _ -> None
        | Slot.Node (relOffset, node) -> Some (Child (GreenNode node, index, relOffset))
        | Slot.Token (relOffset, token) -> Some (Child (GreenToken token, index, relOffset))

    member this.Element = let (Child (element, _, _)) = this in element
    member this.Slot = let (Child (_, slot, _)) = this in slot
    member this.RelOffset = let (Child (_, _, relOffset)) = this in relOffset

type GreenNode with
    member this.Children =
        this.Slots |> Seq.mapi (fun i slot -> Child.Create (i, slot)) |> Seq.choose id
