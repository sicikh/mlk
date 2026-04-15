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

    member this.Kind : RawSyntaxKind =
        this.Green.Kind

type SyntaxToken =
    {
        Parent : SyntaxNode
        Offset : TextSize
        Green : GreenToken
        Slot : uint
    }

    member this.Kind : RawSyntaxKind =
        this.Green.Kind

type SyntaxElement = NodeOrToken<SyntaxNode, SyntaxToken>
