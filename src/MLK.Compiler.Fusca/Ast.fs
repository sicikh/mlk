namespace MLK.Compiler.Fusca

[<Interface>]
type IAstNode =
    abstract Syntax : SyntaxNode

[<Interface>]
type IAstNodeList<'T when 'T :> IAstNode> =
    abstract SyntaxList : SyntaxList

[<Interface>]
type IAstSeparatedList<'T when 'T :> IAstNode> =
    abstract SyntaxList : SyntaxList

type AstNode<'T
    when 'T :> IAstNode
    and 'T : (static member CanCast : RawSyntaxKind -> bool)
    and 'T : (static member Cast : SyntaxNode -> 'T option)> = 'T

module AstNode =
    let inline canCast<'T when AstNode<'T>> (kind : RawSyntaxKind) : bool = 'T.CanCast kind

    let inline cast<'T when AstNode<'T>> (node : SyntaxNode) : 'T option = 'T.Cast node

    let inline unwrapCast<'T when AstNode<'T>> (node : SyntaxNode) : 'T =
        match cast<'T> node with
        | Some astNode -> astNode
        | None -> failwithf "Failed to cast SyntaxNode of kind %A to the expected AST node type." node.Kind

module AstNodeList =
    let inline elements<'T when AstNode<'T>> (list : IAstNodeList<'T>) : 'T seq =
        list.SyntaxList.Slots
        |> Seq.map (
            function
            | SyntaxSlot.Node node -> AstNode.unwrapCast<'T> node
            | SyntaxSlot.Empty _ -> failwith "Expected a node in the list, but found an empty slot"
            | SyntaxSlot.Token _ -> failwith "Expected a node in the list, but found a token"
        )

module AstSeparatedList =
    let inline elements<'T when AstNode<'T>> (_list : IAstSeparatedList<'T>) : 'T seq = failwith "todo"

type SyntaxError = | MissingRequiredChild

type SyntaxResult<'t> = Result<'t, SyntaxError>

module Support =
    let inline node<'T when AstNode<'T>> (slot : uint) (parent : SyntaxNode) : 'T option =
        parent.Slots
        |> Seq.tryItem (int slot)
        |> Option.bind (
            function
            | SyntaxSlot.Empty _ -> None
            | SyntaxSlot.Node node -> AstNode.cast<'T> node
            | SyntaxSlot.Token _syntaxToken -> failwith "Expected a node, but found a token"
        )

    let inline requiredNode<'T when AstNode<'T>> (slot : uint) (parent : SyntaxNode) : SyntaxResult<'T> =
        match node<'T> slot parent with
        | Some node -> Ok node
        | None -> Error MissingRequiredChild

    let token (slot : uint) (parent : SyntaxNode) : SyntaxToken option =
        parent.Slots
        |> Seq.tryItem (int slot)
        |> Option.bind (
            function
            | SyntaxSlot.Empty _ -> None
            | SyntaxSlot.Node _ -> failwith "Expected a token, but found a node"
            | SyntaxSlot.Token token -> Some token
        )

    let requiredToken (slot : uint) (parent : SyntaxNode) : SyntaxResult<SyntaxToken> =
        match token slot parent with
        | Some token -> Ok token
        | None -> Error MissingRequiredChild

    let inline list<'T when AstNode<'T>> (slot : uint) (parent : SyntaxNode) : 'T =
        requiredNode<'T> slot parent
        |> Result.defaultWith (fun _ -> failwith "Expected a list node, but it was missing or of the wrong type")

    let elements (parent : SyntaxNode) : SyntaxElementChildren = SyntaxElementChildren.mk parent
