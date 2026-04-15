namespace MLK.Compiler.Fusca

[<Interface>]
type IAstNode =
    abstract Syntax : SyntaxNode

[<Interface>]
type IAstNodeFactory<'T when 'T :> IAstNode> =
    static abstract CanCast : kind : RawSyntaxKind -> bool
    static abstract Cast : node : SyntaxNode -> 'T option

[<Interface>]
type IAstNodeList<'T when 'T :> IAstNode and 'T :> IAstNodeFactory<'T>> =
    abstract SyntaxList : SyntaxList

[<Interface>]
type IAstSeparatedList<'T when 'T :> IAstNode and 'T :> IAstNodeFactory<'T>> =
    abstract SyntaxList : SyntaxList

module AstNode =
    let canCast<'T when 'T :> IAstNodeFactory<'T>> (kind : RawSyntaxKind) : bool = 'T.CanCast kind

    let cast<'T when 'T :> IAstNodeFactory<'T>> (node : SyntaxNode) : 'T option = 'T.Cast node

    let unwrapCast<'T when 'T :> IAstNodeFactory<'T>> (node : SyntaxNode) : 'T =
        match cast<'T> node with
        | Some astNode -> astNode
        | None -> failwithf "Failed to cast SyntaxNode of kind %A to the expected AST node type." node.Kind

module AstNodeList =
    let elements<'T when 'T :> IAstNodeFactory<'T>> (list : IAstNodeList<'T>) : 'T seq =
        list.SyntaxList.Slots
        |> Seq.map (
            function
            | SyntaxSlot.Node node -> AstNode.unwrapCast<'T> node
            | SyntaxSlot.Empty _ -> failwith "Expected a node in the list, but found an empty slot"
            | SyntaxSlot.Token _ -> failwith "Expected a node in the list, but found a token"
        )

module AstSeparatedList =
    let elements<'T when 'T :> IAstNodeFactory<'T>> (list : IAstSeparatedList<'T>) : 'T seq =
        failwith "todo"

type SyntaxError = | MissingRequiredChild

type SyntaxResult<'t> = Result<'t, SyntaxError>

module Support =
    let node<'T when 'T :> IAstNodeFactory<'T>> (slot : uint) (parent : SyntaxNode) : 'T option =
        parent.Slots
        |> Seq.tryItem (int slot)
        |> Option.bind (
            function
            | SyntaxSlot.Empty _ -> None
            | SyntaxSlot.Node node -> AstNode.cast<'T> node
            | SyntaxSlot.Token _syntaxToken -> failwith "Expected a node, but found a token"
        )

    let requiredNode<'T when 'T :> IAstNodeFactory<'T>> (slot : uint) (parent : SyntaxNode) : SyntaxResult<'T> =
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

    let list<'T when 'T :> IAstNodeFactory<'T>> (slot : uint) (parent : SyntaxNode) : 'T =
        requiredNode<'T> slot parent
        |> Result.defaultWith (fun _ -> failwith "Expected a list node, but it was missing or of the wrong type")

    let elements (parent : SyntaxNode) : SyntaxElementChildren = SyntaxElementChildren.mk parent
