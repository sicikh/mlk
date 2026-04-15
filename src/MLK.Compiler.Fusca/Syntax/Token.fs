namespace MLK.Compiler.Fusca

open MLK.Compiler.Text

type SyntaxNode =
    {
        Parent : SyntaxNode option
        Offset : TextSize
        Green : GreenNode
        Slot : uint
    }

    static member CreateRoot (node : GreenNode) : SyntaxNode =
        {
            Parent = None
            Offset = TextSize.zero
            Green = node
            Slot = 0u
        }

    member this.Kind : RawSyntaxKind = this.Green.Kind

type SyntaxToken =
    {
        Parent : SyntaxNode
        Offset : TextSize
        Green : GreenToken
        Slot : uint
    }

    member this.Kind : RawSyntaxKind = this.Green.Kind

type SyntaxElement = NodeOrToken<SyntaxNode, SyntaxToken>

[<RequireQualifiedAccess>]
type SyntaxSlot =
    | Node of SyntaxNode
    | Token of SyntaxToken
    | Empty of parent : SyntaxNode * index : TextSize

type SyntaxNode with
    member this.Slots : SyntaxSlot seq =
        this.Green.Slots
        |> Seq.mapi (fun i slot ->
            match slot with
            | Slot.Node (relOffset, node) ->
                SyntaxSlot.Node
                    {
                        Parent = Some this
                        Offset = this.Offset + relOffset
                        Green = node
                        Slot = uint i
                    }
            | Slot.Token (relOffset, token) ->
                SyntaxSlot.Token
                    {
                        Parent = this
                        Offset = this.Offset + relOffset
                        Green = token
                        Slot = uint i
                    }
            | Slot.Empty _relOffset -> SyntaxSlot.Empty (this, TextSize.ofInt i)
        )

    member this.FirstChildOrToken () : SyntaxElement option =
        this.Slots
        |> Seq.tryHead
        |> Option.bind (
            function
            | SyntaxSlot.Node node -> Some (SyntaxElement.Node node)
            | SyntaxSlot.Token token -> Some (SyntaxElement.Token token)
            | SyntaxSlot.Empty _ -> None
        )

type SyntaxElementChildren = SyntaxElement seq

module SyntaxElementChildren =
    let mk (parent : SyntaxNode) : SyntaxElementChildren =
        parent.Slots
        |> Seq.choose (
            function
            | SyntaxSlot.Node node -> Some (SyntaxElement.Node node)
            | SyntaxSlot.Token token -> Some (SyntaxElement.Token token)
            | SyntaxSlot.Empty _ -> None
        )

type SyntaxList =
    | SyntaxList of SyntaxNode

    member this.Node = let (SyntaxList node) = this in node

    member this.Slots = this.Node.Slots

type SyntaxNode with
    member this.IntoList () : SyntaxList =
        SyntaxList this
