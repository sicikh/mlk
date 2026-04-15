namespace MLK.Compiler.Fusca.Ast

open MLK.Compiler.Fusca

[<Interface>]
type IAstNodeFactory =
    static abstract CanCast : kind : RawSyntaxKind -> bool
    static abstract Cast : node : SyntaxNode -> AstNode option

and [<AbstractClass>] AstNode() =
    abstract member Syntax : SyntaxNode

    member this.Parent<'T when 'T :> IAstNodeFactory>() : AstNode option =
        this.Syntax.Parent |> Option.bind (fun parent -> parent |> 'T.Cast)

module AstNode =
    let canCast<'T when 'T :> IAstNodeFactory> (kind : RawSyntaxKind) : bool =
        'T.CanCast kind

    let cast<'T when 'T :> IAstNodeFactory> (node : SyntaxNode) : AstNode option =
        'T.Cast node

type TestNode private (node : SyntaxNode) =
    inherit AstNode()

    override this.Syntax = node

    interface IAstNodeFactory with
        static member CanCast (kind : RawSyntaxKind) : bool =
            // Replace with actual logic to determine if the kind corresponds to TestNode
            kind.Value = 1us // Example: assuming 1us corresponds to TestNode

        static member Cast (node : SyntaxNode) : AstNode option =
            if AstNode.canCast<TestNode> node.Kind then
                Some (TestNode node :> AstNode)
            else
                None


type SyntaxError =
    | MissingRequiredChild

type SyntaxResult<'t> = Result<'t, SyntaxError>

module Support =
    let node (slot : uint) (parent : SyntaxNode) : AstNode option =
        failwith "todo"
