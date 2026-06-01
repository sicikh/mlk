namespace MLK.Compiler.Fusca

open System
open System.Collections
open System.Collections.Generic
open Stdx
open MLK.Compiler.Text
open MLK.Compiler.Fusca.Green

type NodeKind =
    | RootNode of GreenElement
    | ChildNode of green : GreenElement * parent : NodeData

    member this.Parent =
        match this with
        | RootNode _ -> None
        | ChildNode (_, parent) -> Some parent

    member this.Green =
        match this with
        | RootNode green -> green
        | ChildNode (green, _) -> green

    member this.RawKind = this.Green.RawKind

and NodeData =
    {
        Kind : NodeKind
        Slot : int
        Offset : TextSize
    }

    member this.Parent = this.Kind.Parent

    member this.Green = this.Kind.Green

    member this.Range = this.Green.Length |> TextRange.at this.Offset

    member this.Detached =
        match this.Kind with
        | RootNode _ -> this
        | ChildNode (green, _) ->
            {
                Kind = RootNode green
                Slot = 0
                Offset = TextSize.zero
            }

    member this.RawKind = this.Green.RawKind

type SyntaxNode =
    internal
    | SyntaxNode of NodeData

    member internal this.Data =
        let (SyntaxNode data) = this
        data

    static member CreateRoot (green : GreenNode) : SyntaxNode =
        SyntaxNode
            {
                Kind = RootNode (GreenNode green)
                Slot = 0
                Offset = TextSize.zero
            }

    static member CreateChild (green : GreenNode, parent : SyntaxNode, slot : int, offset : TextSize) : SyntaxNode =
        SyntaxNode
            {
                Kind = ChildNode (GreenNode green, parent.Data)
                Slot = slot
                Offset = offset
            }

    member this.RawKind : RawSyntaxKind = this.Data.Green.RawKind

    member this.Offset = this.Data.Offset

    member this.Green = this.Data.Green.Node.Value

    member this.Detached = SyntaxNode.CreateRoot this.Green

    member this.Range = this.Data.Range

    member this.Index = this.Data.Slot

type NodeKind with
    member this.ParentNode =
        match this with
        | RootNode _ -> None
        | ChildNode (_, parent) -> Some (SyntaxNode parent)

type NodeData with
    member this.ParentNode = this.Kind.ParentNode

type SyntaxNode with
    member this.Parent = this.Data.ParentNode

type SyntaxToken =
    internal
    | SyntaxToken of NodeData

    member internal this.Data =
        let (SyntaxToken data) = this
        data

    static member Create (green : GreenToken, parent : SyntaxNode, slot : int, offset : TextSize) : SyntaxToken =
        SyntaxToken
            {
                Kind = ChildNode (GreenToken green, parent.Data)
                Slot = slot
                Offset = offset
            }

    static member CreateDetached (green : GreenToken) : SyntaxToken =
        SyntaxToken
            {
                Kind = RootNode (GreenToken green)
                Slot = 0
                Offset = TextSize.zero
            }

    member this.RawKind : RawSyntaxKind = this.Data.Green.Token.Value.RawKind

    member this.Offset = this.Data.Offset

    member this.Green = this.Data.Green.Token.Value

    member this.Range = this.Data.Range

    member this.Index = this.Data.Slot

    member this.RangeTrimmed =
        let greenToken = this.Green
        let leadingLength = greenToken.Leading.Length
        let trailingLength = greenToken.Trailing.Length

        let range = this.Range
        TextRange.Create (range.Start + leadingLength, range.End - trailingLength)

    member this.Text = this.Green.Text
    member this.TextTrimmed = this.Green.TextTrimmed

    member this.Parent = this.Data.ParentNode

    member this.Ancestors : SyntaxNode seq = this.Parent |> Seq.successors _.Parent

    member this.Detached = SyntaxToken.CreateDetached this.Green

type SyntaxTrivia =
    internal
    | SyntaxTrivia of token : SyntaxToken * isLeading : bool

    static member internal Leading (token : SyntaxToken) = SyntaxTrivia (token, true)
    static member internal Trailing (token : SyntaxToken) = SyntaxTrivia (token, false)

    member this.GreenTrivia : GreenTrivia =
        let (SyntaxTrivia (token, isLeading)) = this

        if isLeading then
            token.Green.Leading
        else
            token.Green.Trailing

    member this.Range : TextRange =
        let (SyntaxTrivia (token, isLeading)) = this
        let length = this.GreenTrivia.Length
        let tokenRange = token.Range

        if isLeading then
            length |> TextRange.at tokenRange.Start
        else
            length |> TextRange.at (tokenRange.End - length)

    member this.Length : int = this.GreenTrivia.Count

    member this.Piece (index : int) : TriviaPiece option = this.GreenTrivia.Piece index

    member this.First : TriviaPiece option = this.GreenTrivia.Pieces |> Array.tryHead
    member this.Last : TriviaPiece option = this.GreenTrivia.Pieces |> Array.tryLast

    member this.Token : SyntaxToken =
        let (SyntaxTrivia (token, _)) = this
        token

    member this.Text : string =
        let triviaRange = this.Range

        let relativeRange =
            TextRange.length triviaRange
            |> TextRange.at (triviaRange.Start - this.Token.Offset)

        this.Token.Text.Slice relativeRange

    member this.Pieces : (TextSize * TriviaPiece) seq =
        this.GreenTrivia.Pieces
        |> Seq.mapFold (fun offset piece -> (offset, piece), offset + piece.Length) this.Range.Start
        |> fst

type SyntaxToken with
    member this.LeadingTrivia : SyntaxTrivia = SyntaxTrivia.Leading this

    member this.TrailingTrivia : SyntaxTrivia = SyntaxTrivia.Trailing this

type Siblings =
    internal
    | Siblings of parent : GreenNode * startSlot : int

    static member Create (parent : GreenNode, startSlot : int) : Siblings =
        assert (startSlot < parent.Slots.Length)
        Siblings (parent, startSlot)

    member this.Following : Child seq =
        let (Siblings (parent, startSlot)) = this

        parent.Slots
        |> Seq.enumerate
        |> Seq.skip (startSlot - 1)
        |> Seq.choose Child.Create

    member this.Previous : Child seq =
        let (Siblings (parent, startSlot)) = this

        parent.Slots
        |> Seq.enumerate
        |> Seq.rev
        |> Seq.skip (parent.Slots.Length - 1 - startSlot)
        |> Seq.choose Child.Create

type NodeData with
    member this.GreenSiblings : Siblings option =
        this.Parent
        |> Option.bind (fun parent ->
            match parent.Green with
            | GreenNode node -> Some (Siblings (node, this.Slot))
            | GreenToken _ ->
                // A token should never be a parent of a token or node
                assert false
                None
        )

    member this.NextSibling : SyntaxNode option =
        match this.ParentNode, this.GreenSiblings with
        | Some parent, Some siblings ->
            siblings.Following
            |> Seq.tryPick (fun child -> child.Element.Node |> Option.map (fun green -> child, green))
            |> Option.map (fun (child, green) ->
                SyntaxNode.CreateChild (green, parent, child.Slot, parent.Offset + child.RelOffset)
            )
        | _ -> None

    member this.PrevSibling : SyntaxNode option =
        match this.ParentNode, this.GreenSiblings with
        | Some parent, Some siblings ->
            siblings.Previous
            |> Seq.tryPick (fun child -> child.Element.Node |> Option.map (fun green -> child, green))
            |> Option.map (fun (child, green) ->
                SyntaxNode.CreateChild (green, parent, child.Slot, parent.Offset + child.RelOffset)
            )
        | _ -> None

type SyntaxElement =
    | SyntaxNode of SyntaxNode
    | SyntaxToken of SyntaxToken

    static member Create (element : GreenElement, parent : SyntaxNode, slot : int, offset : TextSize) : SyntaxElement =
        match element with
        | GreenNode node -> SyntaxNode (SyntaxNode.CreateChild (node, parent, slot, offset))
        | GreenToken token -> SyntaxToken (SyntaxToken.Create (token, parent, slot, offset))

    member this.RawKind =
        match this with
        | SyntaxNode node -> node.RawKind
        | SyntaxToken token -> token.RawKind

    member this.Range =
        match this with
        | SyntaxNode node -> node.Range
        | SyntaxToken token -> token.Range

    member this.Index =
        match this with
        | SyntaxNode node -> node.Index
        | SyntaxToken token -> token.Index

    member this.Parent =
        match this with
        | SyntaxNode node -> node.Parent
        | SyntaxToken token -> token.Parent

    member this.Ancestors =
        let first =
            match this with
            | SyntaxNode node -> Some node
            | SyntaxToken token -> token.Parent

        first |> Seq.successors _.Parent

    member this.Green =
        match this with
        | SyntaxNode node -> GreenNode node.Green
        | SyntaxToken token -> GreenToken token.Green

    member this.Detached =
        match this with
        | SyntaxNode node -> SyntaxNode node.Detached
        | SyntaxToken token -> SyntaxToken token.Detached

    member this.Token =
        match this with
        | SyntaxNode _ -> None
        | SyntaxToken token -> Some token

    member this.Node =
        match this with
        | SyntaxNode node -> Some node
        | SyntaxToken _ -> None

type NodeData with
    member this.NextSiblingOrToken : SyntaxElement option =
        match this.ParentNode, this.GreenSiblings with
        | Some parent, Some siblings ->
            siblings.Following
            |> Seq.tryHead
            |> Option.map (fun child ->
                SyntaxElement.Create (child.Element, parent, child.Slot, parent.Offset + child.RelOffset)
            )
        | _ -> None

    member this.PrevSiblingOrToken : SyntaxElement option =
        match this.ParentNode, this.GreenSiblings with
        | Some parent, Some siblings ->
            siblings.Previous
            |> Seq.tryHead
            |> Option.map (fun child ->
                SyntaxElement.Create (child.Element, parent, child.Slot, parent.Offset + child.RelOffset)
            )
        | _ -> None

type SyntaxToken with
    member this.NextSiblingOrToken : SyntaxElement option = this.Data.NextSiblingOrToken
    member this.PrevSiblingOrToken : SyntaxElement option = this.Data.PrevSiblingOrToken

type SyntaxNode with
    member this.NextSibling : SyntaxNode option = this.Data.NextSibling
    member this.PrevSibling : SyntaxNode option = this.Data.PrevSibling

    member this.NextSiblingOrToken : SyntaxElement option = this.Data.NextSiblingOrToken
    member this.PrevSiblingOrToken : SyntaxElement option = this.Data.PrevSiblingOrToken

type SyntaxElement with
    member this.NextSiblingOrToken : SyntaxElement option =
        match this with
        | SyntaxNode node -> node.NextSiblingOrToken
        | SyntaxToken token -> token.NextSiblingOrToken

    member this.PrevSiblingOrToken : SyntaxElement option =
        match this with
        | SyntaxNode node -> node.PrevSiblingOrToken
        | SyntaxToken token -> token.PrevSiblingOrToken

type SyntaxToken with
    member this.SiblingsWithTokens (direction : Direction) : SyntaxElement seq =
        let sibling : SyntaxElement -> SyntaxElement option =
            match direction with
            | Direction.Next -> _.NextSiblingOrToken
            | Direction.Prev -> _.PrevSiblingOrToken

        Some (SyntaxElement.SyntaxToken this) |> Seq.successors sibling

type SyntaxNode with
    member this.Tokens : SyntaxToken seq =
        this.Green.Children
        |> Seq.choose (fun child ->
            child.Element.Token
            |> Option.map (fun token -> SyntaxToken.Create (token, this, child.Slot, this.Offset + child.RelOffset))
        )

    member this.Children : SyntaxNode seq =
        this.Green.Children
        |> Seq.choose (fun child ->
            child.Element.Node
            |> Option.map (fun node -> SyntaxNode.CreateChild (node, this, child.Slot, this.Offset + child.RelOffset))
        )

    member this.ChildrenWithTokens : SyntaxElement seq =
        this.Green.Children
        |> Seq.map (fun child -> SyntaxElement.Create (child.Element, this, child.Slot, this.Offset + child.RelOffset))

    member this.FirstChild : SyntaxNode option = this.Children |> Seq.tryHead

    member this.FirstChildOrToken : SyntaxElement option =
        this.ChildrenWithTokens |> Seq.tryHead

    member this.LastChild : SyntaxNode option = this.Children |> Seq.tryLast

    member this.LastChildOrToken : SyntaxElement option =
        this.ChildrenWithTokens |> Seq.tryLast

    member this.Siblings (direction : Direction) : SyntaxNode seq =
        let sibling : SyntaxNode -> SyntaxNode option =
            match direction with
            | Direction.Next -> _.NextSibling
            | Direction.Prev -> _.PrevSibling

        Some this |> Seq.successors sibling

    member this.SiblingsWithTokens (direction : Direction) : SyntaxElement seq =
        let sibling : SyntaxElement -> SyntaxElement option =
            match direction with
            | Direction.Next -> _.NextSiblingOrToken
            | Direction.Prev -> _.PrevSiblingOrToken

        Some (SyntaxElement.SyntaxNode this) |> Seq.successors sibling

type Preorder =
    internal
    | Preorder of start : SyntaxNode * next : WalkEvent<SyntaxNode> option * skipSubtree : bool

    static member Create (start : SyntaxNode) : Preorder =
        Preorder (start, Some (WalkEvent.Enter start), false)

    member this.SkipSubtree () =
        let (Preorder (start, next, _)) = this
        Preorder (start, next, true)

    member private this.DoSkip () =
        let (Preorder (start, next, skipSubtree)) = this

        if not skipSubtree then
            this
        else
            let next =
                next
                |> Option.map (
                    function
                    | WalkEvent.Enter firstChild -> WalkEvent.Leave firstChild.Parent.Value
                    | WalkEvent.Leave parent -> WalkEvent.Leave parent
                )

            Preorder (start, next, false)

    member this.Next () : (WalkEvent<SyntaxNode> * Preorder) option =
        let Preorder (start, next, false) | Never (start, next) = this.DoSkip ()

        let newNext =
            match next with
            | None -> None
            | Some (WalkEvent.Enter node) ->
                match node.FirstChild with
                | Some firstChild -> Some (WalkEvent.Enter firstChild)
                | None -> Some (WalkEvent.Leave node)
            | Some (WalkEvent.Leave node) ->
                if node = start then
                    None
                else
                    match node.NextSibling with
                    | Some sibling -> Some (WalkEvent.Enter sibling)
                    | None -> node.Parent |> Option.map WalkEvent.Leave

        next |> Option.map (fun next -> next, Preorder (start, newNext, false))

    interface IEnumerable<WalkEvent<SyntaxNode>> with
        member this.GetEnumerator () =
            (this |> Seq.unfold _.Next() :> IEnumerable<_>).GetEnumerator ()

    interface IEnumerable with
        member this.GetEnumerator () =
            (this :> IEnumerable<_>).GetEnumerator ()

type PreorderWithTokens =
    internal
    | PreorderWithTokens of
        start : SyntaxElement *
        next : WalkEvent<SyntaxElement> option *
        skipSubtree : bool *
        direction : Direction

    static member Create (start : SyntaxElement, direction : Direction) : PreorderWithTokens =
        PreorderWithTokens (start, Some (WalkEvent.Enter start), false, direction)

    member this.SkipSubtree () =
        let (PreorderWithTokens (start, next, _, direction)) = this
        PreorderWithTokens (start, next, true, direction)

    member private this.DoSkip () =
        let (PreorderWithTokens (start, next, skipSubtree, direction)) = this

        if not skipSubtree then
            this
        else
            let next =
                next
                |> Option.map (
                    function
                    | WalkEvent.Enter firstChild -> WalkEvent.Leave (SyntaxNode firstChild.Parent.Value)
                    | WalkEvent.Leave parent -> WalkEvent.Leave parent
                )

            PreorderWithTokens (start, next, false, direction)

    member this.Next () : (WalkEvent<SyntaxElement> * PreorderWithTokens) option =
        let PreorderWithTokens (start, next, false, direction) | Never (start, next, direction) =
            this.DoSkip ()

        let newNext =
            match next with
            | None -> None
            | Some (WalkEvent.Enter el) ->
                match el with
                | SyntaxNode node ->
                    let next =
                        match direction with
                        | Direction.Next -> node.FirstChildOrToken
                        | Direction.Prev -> node.LastChildOrToken

                    match next with
                    | Some child -> Some (WalkEvent.Enter child)
                    | None -> Some (WalkEvent.Leave el)
                | SyntaxToken _ -> Some (WalkEvent.Leave el)
            | Some (WalkEvent.Leave el) when el = start -> None
            | Some (WalkEvent.Leave el) ->
                let next =
                    match direction with
                    | Direction.Next -> el.NextSiblingOrToken
                    | Direction.Prev -> el.PrevSiblingOrToken

                match next with
                | Some sibling -> Some (WalkEvent.Enter sibling)
                | None -> el.Parent |> Option.map (SyntaxNode >> WalkEvent.Leave)

        next
        |> Option.map (fun next -> next, PreorderWithTokens (start, newNext, false, direction))

    interface IEnumerable<WalkEvent<SyntaxElement>> with
        member this.GetEnumerator () =
            (this |> Seq.unfold _.Next() :> IEnumerable<_>).GetEnumerator ()

    interface IEnumerable with
        member this.GetEnumerator () =
            (this :> IEnumerable<_>).GetEnumerator ()

type SyntaxNode with
    member this.Preorder : Preorder = Preorder.Create this

    member this.PreorderWithTokens (direction : Direction) : PreorderWithTokens =
        PreorderWithTokens.Create (SyntaxElement.SyntaxNode this, direction)

    member this.Descendants : SyntaxNode seq = this.Preorder |> Seq.choose _.AsEnter

    member this.DescendantsWithTokens (direction : Direction) : SyntaxElement seq =
        this.PreorderWithTokens direction |> Seq.choose _.AsEnter

    member this.FirstToken : SyntaxToken option =
        this.DescendantsWithTokens Direction.Next |> Seq.tryPick _.Token

    member this.LastToken : SyntaxToken option =
        this.DescendantsWithTokens Direction.Prev |> Seq.tryPick _.Token

type SyntaxElement with
    member this.FirstToken : SyntaxToken option =
        match this with
        | SyntaxNode node -> node.FirstToken
        | SyntaxToken token -> Some token

    member this.LastToken : SyntaxToken option =
        match this with
        | SyntaxNode node -> node.LastToken
        | SyntaxToken token -> Some token

type SyntaxToken with
    member private this.NextTokenImpl (direction : Direction) : SyntaxToken option =
        let rec aux (current : WalkEvent<SyntaxElement>) : SyntaxToken option =
            match current with
            | WalkEvent.Enter (SyntaxElement.SyntaxToken token) -> Some token
            | WalkEvent.Enter (SyntaxElement.SyntaxNode node) ->
                let firstChild =
                    match direction with
                    | Direction.Next -> node.FirstChildOrToken
                    | Direction.Prev -> node.LastChildOrToken

                match firstChild with
                // If node is empty, leave parent
                | None -> aux (WalkEvent.Leave (SyntaxElement.SyntaxNode node))
                // Otherwise traverse full sub-tree
                | Some child -> aux (WalkEvent.Enter child)
            | WalkEvent.Leave element ->
                let rec go (currentElement : SyntaxElement) : WalkEvent<SyntaxElement> option =
                    // Only traverse the left (pref) / right (next) sibligns othe parent
                    // to avoid traversing into the same children again.
                    let sibling =
                        match direction with
                        | Direction.Next -> currentElement.NextSiblingOrToken
                        | Direction.Prev -> currentElement.PrevSiblingOrToken

                    match sibling with
                    // Traverse all children of the sibling
                    | Some sibling -> Some (WalkEvent.Enter sibling)
                    | None ->
                        match currentElement.Parent with
                        | Some node -> go (SyntaxElement.SyntaxNode node)
                        // Reached root, no token found
                        | None -> None

                match go element with
                | Some next -> aux next
                | None -> None

        aux (WalkEvent.Leave (SyntaxElement.SyntaxToken this))

    member this.NextToken : SyntaxToken option = this.NextTokenImpl Direction.Next

    member this.PrevToken : SyntaxToken option = this.NextTokenImpl Direction.Prev

type PreorderTokens =
    internal
    | PreorderTokens of next : SyntaxToken option * direction : Direction

    static member Create (start : SyntaxNode, direction : Direction) : PreorderTokens =
        let next =
            match direction with
            | Direction.Next -> start.FirstToken
            | Direction.Prev -> start.LastToken

        PreorderTokens (next, direction)

    member this.Next () : (SyntaxToken * PreorderTokens) option =
        let (PreorderTokens (next, direction)) = this

        let newNext =
            next
            |> Option.bind (fun token ->
                match direction with
                | Direction.Next -> token.NextToken
                | Direction.Prev -> token.PrevToken
            )

        next |> Option.map (fun next -> next, PreorderTokens (newNext, direction))

    interface IEnumerable<SyntaxToken> with
        member this.GetEnumerator () =
            (this |> Seq.unfold _.Next() :> IEnumerable<_>).GetEnumerator ()

    interface IEnumerable with
        member this.GetEnumerator () =
            (this :> IEnumerable<_>).GetEnumerator ()


//[<CustomEquality ; NoComparison>]
//type SyntaxNode =
//    {
//        Parent : SyntaxNode option
//        Offset : TextSize
//        Green : GreenNode
//        Slot : uint
//    }
//
//    static member CreateRoot (node : GreenNode) : SyntaxNode =
//        {
//            Parent = None
//            Offset = TextSize.zero
//            Green = node
//            Slot = 0u
//        }
//
//    member this.RawKind : RawSyntaxKind = this.Green.RawKind
//
//    member this.Range = this.Green.Length |> TextRange.at this.Offset
//
//    override this.Equals (other : obj) =
//        match other with
//        | :? SyntaxNode as other ->
//            this.Parent = other.Parent
//            && this.Offset = other.Offset
//            && obj.ReferenceEquals (this.Green, other.Green)
//        | _ -> false
//
//    override this.GetHashCode () =
//        HashCode.Combine (this.Parent, this.Offset, this.Green.GetHashCode ())
//
//[<CustomEquality ; NoComparison>]
//type SyntaxToken =
//    {
//        Parent : SyntaxNode option
//        Offset : TextSize
//        Green : GreenToken
//        Slot : uint
//    }
//
//    member this.RawKind : RawSyntaxKind = this.Green.RawKind
//
//    member this.Range = this.Green.Length |> TextRange.at this.Offset
//
//    override this.Equals (other : obj) =
//        match other with
//        | :? SyntaxToken as other ->
//            this.Parent = other.Parent
//            && this.Offset = other.Offset
//            && obj.ReferenceEquals (this.Green, other.Green)
//        | _ -> false
//
//    override this.GetHashCode () =
//        HashCode.Combine (this.Parent, this.Offset, this.Green.GetHashCode ())
//
//type SyntaxElement =
//    | SyntaxNode of SyntaxNode
//    | SyntaxToken of SyntaxToken
//
//    static member Create (element : GreenElement, parent : SyntaxNode, slot : uint, offset : TextSize) : SyntaxElement =
//        match element with
//        | GreenNode node ->
//            SyntaxNode
//                {
//                    Parent = Some parent
//                    Offset = offset
//                    Green = node
//                    Slot = slot
//                }
//        | GreenToken token ->
//            SyntaxToken
//                {
//                    Parent = Some parent
//                    Offset = offset
//                    Green = token
//                    Slot = slot
//                }
//
//    member this.RawKind =
//        match this with
//        | SyntaxNode node -> node.RawKind
//        | SyntaxToken token -> token.RawKind
//
//    member this.Range =
//        match this with
//        | SyntaxNode node -> node.Range
//        | SyntaxToken token -> token.Range
//
//    member this.Slot =
//        match this with
//        | SyntaxNode node -> node.Slot
//        | SyntaxToken token -> token.Slot
//
//    member this.Parent =
//        match this with
//        | SyntaxNode node -> node.Parent
//        | SyntaxToken token -> token.Parent
//
//    member this.Green =
//        match this with
//        | SyntaxNode node -> GreenNode node.Green
//        | SyntaxToken token -> GreenToken token.Green
//
//[<RequireQualifiedAccess>]
//type SyntaxSlot =
//    | Node of SyntaxNode
//    | Token of SyntaxToken
//    | Empty of parent : SyntaxNode * index : uint
//
//    member this.Element =
//        match this with
//        | SyntaxSlot.Node node -> Some (SyntaxNode node)
//        | SyntaxSlot.Token token -> Some (SyntaxToken token)
//        | SyntaxSlot.Empty _ -> None
//
//type SyntaxNode with
//    member this.Slots : SyntaxSlot seq =
//        this.Green.Slots
//        |> Seq.mapi (fun i slot ->
//            match slot with
//            | Slot.Node (relOffset, node) ->
//                SyntaxSlot.Node
//                    {
//                        Parent = Some this
//                        Offset = this.Offset + relOffset
//                        Green = node
//                        Slot = uint i
//                    }
//            | Slot.Token (relOffset, token) ->
//                SyntaxSlot.Token
//                    {
//                        Parent = this
//                        Offset = this.Offset + relOffset
//                        Green = token
//                        Slot = uint i
//                    }
//            | Slot.Empty _relOffset -> SyntaxSlot.Empty (this, uint i)
//        )
//
//    member this.FirstChildOrToken () : SyntaxElement option =
//        this.Slots |> Seq.tryHead |> Option.bind _.Element
//
//type SyntaxElementChildren = SyntaxElement seq
//
//module SyntaxElementChildren =
//    let mk (parent : SyntaxNode) : SyntaxElementChildren =
//        parent.Slots
//        |> Seq.choose (
//            function
//            | SyntaxSlot.Node node -> Some (SyntaxNode node)
//            | SyntaxSlot.Token token -> Some (SyntaxToken token)
//            | SyntaxSlot.Empty _ -> None
//        )
//
//type SyntaxList =
//    | SyntaxList of SyntaxNode
//
//    member this.Node = let (SyntaxList node) = this in node
//
//    member this.Slots = this.Node.Slots
//
//type SyntaxNode with
//    member this.IntoList () : SyntaxList = SyntaxList this
