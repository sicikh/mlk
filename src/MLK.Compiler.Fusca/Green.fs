// Port of biome's rowan crate, that is fork of rust-analyzer's one.
// https://github.com/biomejs/biome/tree/main/crates/biome_rowan

namespace MLK.Compiler.Fusca

open System.Collections.Generic
open Stdx
open MLK.Compiler.Text

[<Struct>]
type RawSyntaxKind = RawSyntaxKind of uint16

type TriviaPieceKind =
    | Newline
    | Whitespace
    | SingleLineComment
    | MultiLineComment
    | Skipped

    member this.IsComment = this.IsSingleLineComment || this.IsMultiLineComment

type TriviaPiece =
    { Kind : TriviaPieceKind
      Length : TextSize }

module TriviaPiece =
    let create kind length = { Kind = kind; Length = length }

    let newline length = create TriviaPieceKind.Newline length

    let whitespace length =
        create TriviaPieceKind.Whitespace length

    let singleLineComment length =
        create TriviaPieceKind.SingleLineComment length

    let multiLineComment length =
        create TriviaPieceKind.MultiLineComment length

    let skipped length = create TriviaPieceKind.Skipped length

type NodeOrToken<'N, 'T> =
    | Node of node : 'N
    | Token of token : 'T

    override this.ToString() =
        match this with
        | Node n -> n.ToString ()
        | Token t -> t.ToString ()

module NodeOrToken =
    let asNode self =
        match self with
        | Node n -> Some n
        | Token _ -> None

    let asToken self =
        match self with
        | Node _ -> None
        | Token t -> Some t

    let map onNode onToken self =
        match self with
        | Node n -> n |> onNode |> Node
        | Token t -> t |> onToken |> Token

    let consolidate onNode onToken self =
        match self with
        | Node n -> n |> onNode
        | Token t -> t |> onToken

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

type GreenTrivia = { Pieces : TriviaPiece array }

module GreenTrivia =
    let create pieces = { Pieces = pieces }

    let empty = create [||]

    let length (self : GreenTrivia) = self.Pieces |> Array.sumBy _.Length
    let count (self : GreenTrivia) = self.Pieces.Length

    let piece (i : uint32) (self : GreenTrivia) : TriviaPiece voption =
        if i < uint32 self.Pieces.Length then
            ValueSome self.Pieces[int i]
        else
            ValueNone

type GreenTrivia with
    static member Zero = GreenTrivia.empty

    member this.Length = GreenTrivia.length this
    member this.Count = GreenTrivia.count this
    member this.Piece(i : uint32) = GreenTrivia.piece i this

type GreenToken =
    { Kind : RawSyntaxKind
      Text : string
      Leading : GreenTrivia
      Trailing : GreenTrivia }

module GreenToken =
    let withTrivia (kind : RawSyntaxKind) (text : string) (leading : GreenTrivia) (trailing : GreenTrivia) =
        { Kind = kind
          Text = text
          Leading = leading
          Trailing = trailing }

    let create (kind : RawSyntaxKind) (text : string) =
        withTrivia kind text GreenTrivia.empty GreenTrivia.empty

    let textTrimmed (self : GreenToken) : string =
        let starts = self.Leading.Length
        let ends = self.Text.Size - self.Trailing.Length
        self.Text.Slice (TextRange.create starts ends)

    let length (self : GreenToken) = self.Text.Size

type GreenToken with
    static member WithTrivia(kind : RawSyntaxKind, text : string, leading : GreenTrivia, trailing : GreenTrivia) =
        GreenToken.withTrivia kind text leading trailing

    static member Create(kind : RawSyntaxKind, text : string) = GreenToken.create kind text

    member this.TextTrimmed = GreenToken.textTrimmed this
    member this.Length = GreenToken.length this

[<RequireQualifiedAccess>]
type Slot =
    | Node of relOffset : TextSize * node : GreenNode
    | Token of relOffset : TextSize * token : GreenToken
    | Empty of relOffset : TextSize

and GreenNode =
    { Kind : RawSyntaxKind
      Length : TextSize
      Slots : Slot array }

type GreenElement = NodeOrToken<GreenNode, GreenToken>

module GreenElement =
    let length (element : GreenElement) : TextSize =
        match element with
        | Node n -> n.Length
        | Token t -> t.Length

module Slot =
    let relOffset (slot : Slot) =
        match slot with
        | Slot.Node (relOffset, _)
        | Slot.Token (relOffset, _)
        | Slot.Empty relOffset -> relOffset

    let asElement (slot : Slot) : GreenElement voption =
        match slot with
        | Slot.Node (_, n) -> ValueSome (Node n)
        | Slot.Token (_, t) -> ValueSome (Token t)
        | Slot.Empty _ -> ValueNone

    let relRange (slot : Slot) : TextRange =
        slot
        |> asElement
        |> ValueOption.mapOrZero GreenElement.length
        |> TextRange.at (relOffset slot)

type Generation =
    | GenA = 0u
    | GenB = 1u

module Generation =
    let flip (generation : Generation) : Generation =
        if generation = Generation.GenA then
            Generation.GenB
        else
            Generation.GenA

type CachedNode =
    { mutable Generation : Generation
      Node : GreenNode
      Hash : uint64 }

type CachedToken =
    { mutable Generation : Generation
      Token : GreenToken }

type CachedTrivia =
    { mutable Generation : Generation
      Trivia : GreenTrivia }

[<AutoOpen>]
module private Hash =
    let fnvOffsetBasis = 14695981039346656037UL
    let fnvPrime = 1099511628211UL

    let hashUint16 (value : uint16) (hash : uint64) : uint64 =
        let hash = hash ^^^ uint64 value
        hash * fnvPrime

    let hashInt32 (value : int32) (hash : uint64) : uint64 =
        let hash = hash ^^^ uint64 value
        hash * fnvPrime

    let hashString (value : string) (hash : uint64) : uint64 =
        value |> Seq.fold (fun h c -> h |> hashUint16 (uint16 c)) hash

    let tokenHashOf (kind : RawSyntaxKind) (text : string) : uint64 =
        fnvOffsetBasis
        |> hashUint16 (let (RawSyntaxKind k) = kind in k)
        |> hashString text

    let tokenHash (token : GreenToken) : uint64 = tokenHashOf token.Kind token.Text

type TriviaCache () =
    let trivias = Dictionary<uint64, ResizeArray<CachedTrivia>> ()

    let whitespace = GreenTrivia.create [| TriviaPiece.whitespace (TextSize 1u) |]

    let triviaHashOf (pieces : TriviaPiece array) : uint64 =
        pieces
        |> Array.fold (fun h p -> h |> hashInt32 (p.GetHashCode ())) fnvOffsetBasis

    member _.Get(generation : Generation, pieces : TriviaPiece array) : GreenTrivia =
        match pieces with
        | [||] -> GreenTrivia.empty
        | [| { Kind = TriviaPieceKind.Whitespace
               Length = TextSize 1u } |] -> whitespace
        | _ ->
            let hash = triviaHashOf pieces

            match trivias.TryGetValue hash with
            | true, bucket ->
                let found =
                    bucket
                    |> Seq.tryFind (fun cached ->
                        // Compare the pieces for equality
                        let cachedPieces = cached.Trivia.Pieces

                        if cachedPieces.Length <> pieces.Length then
                            false
                        else
                            Array.forall2 (=) cachedPieces pieces)

                match found with
                | Some cached -> cached.Trivia
                | None ->
                    let trivia = GreenTrivia.create pieces

                    bucket.Add
                        { Generation = generation
                          Trivia = trivia }

                    trivia

            | false, _ ->
                let trivia = GreenTrivia.create pieces

                trivias[hash] <-
                    ResizeArray
                        [ { Generation = generation
                            Trivia = trivia } ]

                trivia



type NodeCache () =
    [<Literal>]
    let UncachedNodeHash = 0UL

    let nodes = Dictionary<uint64, ResizeArray<CachedNode>> ()
    let tokens = Dictionary<uint64, CachedToken> ()
