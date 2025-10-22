namespace MLK.Compiler.Text

[<Struct>]
type TextSize =
    | TextSize of uint32

    override this.ToString() =
        let (TextSize size) = this
        size.ToString ()

    static member (+)(TextSize a, TextSize b) : TextSize = TextSize (a + b)

    static member (-)(TextSize a, TextSize b) : TextSize =
        if b > a then
            invalidOp "Resulting TextSize cannot be negative."

        TextSize (a - b)

module TextSize =
    let zero = TextSize 0u

    let ofInt (size : int) : TextSize =
        if size < 0 then
            invalidArg "size" "Size cannot be negative."

        TextSize (uint32 size)

    let toInt (TextSize size) : int = int size
    let ofUint (size : uint32) : TextSize = TextSize size
    let toUint (TextSize size) : uint32 = size

    let ofStr (s : string) : TextSize = TextSize (uint32 s.Length)

    let checkedAdd (TextSize a) (TextSize b) : TextSize voption =
        let sum = uint64 a + uint64 b

        if sum > uint64 System.UInt32.MaxValue then
            ValueNone
        else
            ValueSome (TextSize (uint32 sum))

    let checkedSub (TextSize a) (TextSize b) : TextSize voption =
        if b > a then ValueNone else ValueSome (TextSize (a - b))

type TextSize with
    static member Zero = TextSize.zero

[<Struct>]
type TextRange =
    private
        { Starts : TextSize
          Ends : TextSize }

    member this.Start = this.Starts
    member this.End = this.Ends

    override this.ToString() =
        sprintf "%s..%s" (this.Start.ToString ()) (this.End.ToString ())

module TextRange =
    let create (start : TextSize) (ends : TextSize) : TextRange =
        assert (start <= ends)
        { Starts = start; Ends = ends }

    let at (offset : TextSize) (length : TextSize) : TextRange = create offset (offset + length)

    let emptyAt (offset : TextSize) : TextRange = create offset offset

    let empty = emptyAt TextSize.zero

    let upTo (ends : TextSize) : TextRange = create TextSize.zero ends

    let length (range : TextRange) : TextSize = range.End - range.Start

    let isEmpty (range : TextRange) : bool = range.Start = range.End

    let contains (offset : TextSize) (range : TextRange) : bool =
        range.Start <= offset && offset < range.End

    let containsInclusive (offset : TextSize) (range : TextRange) : bool =
        range.Start <= offset && offset <= range.End

    let containsRange (inner : TextRange) (outer : TextRange) : bool =
        outer.Start <= inner.Start && inner.End <= outer.End

    let intersect (a : TextRange) (b : TextRange) : TextRange voption =
        let start = max a.Start b.Start
        let ends = min a.End b.End

        if ends < start then
            ValueNone
        else
            ValueSome (create start ends)

    let cover (a : TextRange) (b : TextRange) : TextRange =
        let start = min a.Start b.Start
        let ends = max a.End b.End
        create start ends

    let coverOffset (offset : TextSize) (range : TextRange) : TextRange = cover (emptyAt offset) range

    let checkedAdd (offset : TextSize) (range : TextRange) : TextRange voption =
        match TextSize.checkedAdd range.Start offset, TextSize.checkedAdd range.End offset with
        | ValueSome newStart, ValueSome newEnd -> ValueSome (create newStart newEnd)
        | _ -> ValueNone

    let checkedSub (offset : TextSize) (range : TextRange) : TextRange voption =
        match TextSize.checkedSub range.Start offset, TextSize.checkedSub range.End offset with
        | ValueSome newStart, ValueSome newEnd -> ValueSome (create newStart newEnd)
        | _ -> ValueNone

    let addStart (amount : TextSize) (range : TextRange) : TextRange = create (range.Start + amount) range.End

    let subStart (amount : TextSize) (range : TextRange) : TextRange = create (range.Start - amount) range.End

    let addEnd (amount : TextSize) (range : TextRange) : TextRange = create range.Start (range.End + amount)

    let subEnd (amount : TextSize) (range : TextRange) : TextRange = create range.Start (range.End - amount)

    let fromUint (start : uint32) (ends : uint32) : TextRange =
        create (TextSize.ofUint start) (TextSize.ofUint ends)

[<AutoOpen>]
module TextRangeOps =
    let (|TextRange|) (range : TextRange) = struct (range.Start, range.End)

type TextRange with
    static member (+)(range : TextRange, offset : TextSize) : TextRange =
        match TextRange.checkedAdd offset range with
        | ValueSome newRange -> newRange
        | ValueNone -> invalidOp "TextRange +offset overflowed"

    static member (-)(range : TextRange, offset : TextSize) : TextRange =
        match TextRange.checkedSub offset range with
        | ValueSome newRange -> newRange
        | ValueNone -> invalidOp "TextRange -offset overflowed"

    static member Zero = TextRange.empty

[<AutoOpen>]
module StringOps =
    type System.String with
        member this.Size : TextSize = TextSize.ofStr this

        member this.Slice(range : TextRange) : string =
            let start = TextSize.toInt range.Start
            let length = TextSize.toInt (TextRange.length range)
            this.Substring (start, length)
