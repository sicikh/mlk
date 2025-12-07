namespace MLK.Compiler.Ungrammar

/// <summary>A node, like <c>A = 'b' | 'c'</c>.</summary>
type Node = | Node of uint32
/// <summary>A token, denoted with single quotes, like <c>'struct'</c> or <c>'+'</c>.</summary>
type Token = | Token of uint32

/// <summary>A production rule.</summary>
type Rule =
    /// <summary>A labeled rule, like <c>a:B</c> (<c>a</c> is the label, <c>B</c> is the rule).</summary>
    | RLabeled of label : string * rule : Rule
    /// <summary>A node, like <c>A</c>.</summary>
    | RNode of Node
    /// <summary>A token, like <c>'struct'</c>.</summary>
    | RToken of Token
    /// <summary>A sequence of rules, like <c>A 'b' C</c>.</summary>
    | RSeq of Rule list
    /// <summary>An alternative between many rules, like <c>'+'</c> | <c>'-'</c> | <c>'*'</c> | <c>'/'</c>.</summary>
    | RAlt of Rule list
    /// <summary>An optional rule, like <c>A?</c>.</summary>
    | ROpt of Rule
    /// <summary>A repeated rule, like <c>A*</c>.</summary>
    | RStar of Rule

/// <summary>Data about a token.</summary>
[<Sealed>]
type TokenData =
    new : name : string -> TokenData

    /// <summary>The name of the token.</summary>
    member Name : string

[<AutoOpen>]
module TokenData =
    /// <summary>Active pattern to deconstruct a TokenData.</summary>
    val (|TokenData|) : tokenData : TokenData -> string

/// <summary>Data about a node.</summary>
[<Sealed>]
type NodeData =
    new : name : string * rule : Rule -> NodeData

    /// <summary>The name of the node.</summary>
    /// <remarks>In the rule <c>A = 'b' | 'c'</c>, the name is <c>"A"</c>.</remarks>
    member Name : string

    /// <summary>The rule for this node.</summary>
    /// <remarks>In the rule <c>A = 'b' | 'c'</c>, this represents <c>'b' | 'c'</c>.</remarks>
    member Rule : Rule

[<AutoOpen>]
module NodeData =
    /// <summary>Active pattern to deconstruct a NodeData.</summary>
    val (|NodeData|) : nodeData : NodeData -> name : string * rule : Rule

/// <summary>An Ungrammar grammar.</summary>
[<Sealed>]
type Grammar =
    static member Parse : input : string -> Grammar

    member Tokens : Token seq
    member Nodes : Node seq
    member Token : index : Token -> TokenData
    member Node : index : Node -> NodeData
