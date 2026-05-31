namespace MLK.Compiler.Fusca.Cache

open System.Collections.Generic
open MLK.Compiler.Text
open MLK.Compiler.Fusca

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
    {
        mutable Generation : Generation
        Node : GreenNode
        Hash : uint64
    }

type CachedToken =
    {
        mutable Generation : Generation
        Token : GreenToken
    }

type CachedTrivia =
    {
        mutable Generation : Generation
        Trivia : GreenTrivia
    }

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
        |> hashUint16 kind.Value
        |> hashString text

    let tokenHash (token : GreenToken) : uint64 = tokenHashOf token.Kind token.Text

type TriviaCache() =
    let trivias = Dictionary<uint64, ResizeArray<CachedTrivia>> ()

    let whitespace = GreenTrivia.create [| TriviaPiece.whitespace (TextSize 1u) |]

    let triviaHashOf (pieces : TriviaPiece array) : uint64 =
        pieces
        |> Array.fold (fun h p -> h |> hashInt32 (p.GetHashCode ())) (hashInt32 pieces.Length fnvOffsetBasis)

    member _.Get (generation : Generation, pieces : TriviaPiece array) : GreenTrivia =
        match pieces with
        | [||] -> GreenTrivia.empty
        | [| {
                 Kind = TriviaPieceKind.Whitespace
                 Length = TextSize 1u
             } |] -> whitespace
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
                            Array.forall2 (=) cachedPieces pieces
                    )

                match found with
                | Some cached ->
                    cached.Generation <- generation
                    cached.Trivia

                | None ->
                    let trivia = GreenTrivia.create pieces

                    bucket.Add
                        {
                            Generation = generation
                            Trivia = trivia
                        }

                    trivia

            | false, _ ->
                let trivia = GreenTrivia.create pieces

                trivias[hash] <-
                    ResizeArray
                        [
                            {
                                Generation = generation
                                Trivia = trivia
                            }
                        ]

                trivia

// TODO:
type NodeCache() =
    [<Literal>]
    let UncachedNodeHash = 0UL

    let nodes = Dictionary<uint64, ResizeArray<CachedNode>> ()
    let tokens = Dictionary<uint64, CachedToken> ()
