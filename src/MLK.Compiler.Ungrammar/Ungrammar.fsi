namespace MLK.Compiler.Ungrammar

/// A node, like `A = 'b' | 'c'`.
type Node = | Node of uint32
/// A token, denoted with single quotes, like `'struct'` or `'+'`.
type Token = | Token of uint32

/// A production rule.
type Rule =
    /// A labeled rule, like `a:B` (`a` is the label, `B` is the rule).
    | RLabeled of label : string * rule : Rule
    /// A node, like `A`.
    | RNode of Node
    /// A token, like `'struct'`.
    | RToken of Token
    /// A sequence of rules, like `A 'b' C`.
    | RSeq of Rule list
    /// An alternative between many rules, like `'+' | '-' | '*' | '/'`.
    | RAlt of Rule list
    /// An optional rule, like `A?`.
    | ROpt of Rule
    /// A repeated rule, like `A*`.
    | RStar of Rule

/// Data about a token.
[<Sealed>]
type TokenData =
    new : name : string -> TokenData

    /// The name of the token.
    member Name : string

[<AutoOpen>]
module TokenData =
    /// Active pattern to deconstruct a TokenData.
    val (|TokenData|) : tokenData : TokenData -> string

/// Data about a node.
[<Sealed>]
type NodeData =
    new : name : string * rule : Rule -> NodeData

    /// The name of the node.
    ///
    /// In the rule `A = 'b' | 'c'`, the name is `"A"`.
    member Name : string

    /// The rule for this node.
    ///
    /// In the rule `A = 'b' | 'c'`, this represents `'b' | 'c'`.
    member Rule : Rule

[<AutoOpen>]
module NodeData =
    /// Active pattern to deconstruct a NodeData.
    val (|NodeData|) : nodeData : NodeData -> name : string * rule : Rule

/// An error encountered when parsing a Grammar.
[<Sealed>]
type SyntaxError = class end

/// An Ungrammar grammar.
[<Sealed>]
type Grammar =
    static member Parse : input : string -> Result<Grammar, SyntaxError>

    member Tokens : Token seq
    member Nodes : Node seq
    member Token : index : Token -> TokenData
    member Node : index : Node -> NodeData
