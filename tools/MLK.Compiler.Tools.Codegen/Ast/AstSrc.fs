namespace MLK.Compiler.Tools.Codegen.Ast

open Stdx
open MLK.Compiler.Ungrammar

[<Interface>]
type ILanguageSrc =
    abstract Punct : (string * string) list
    abstract Keywords : string list
    abstract Literals : string
    abstract Tokens : string

    abstract Nodes : string list

    abstract ToMethodName : tokenName : string -> string

type AstListSeparatorConfiguration =
    {
        SeparatorToken : string
        AllowTrailingSeparator : bool
    }

type AstListSrc =
    {
        ElementName : string
        Separator : AstListSeparatorConfiguration option
    }

type TokenKind =
    | SingleToken of name : string
    | ManyTokens of names : string list

type Field =
    | TokenField of name : string * kind : TokenKind * optional : bool
    | NodeField of name : string * ty : string * optional : bool

    member this.MethodName (languageSrc : ILanguageSrc) : string =
        match this with
        | TokenField (name = name) ->
            let name = languageSrc.ToMethodName name
            $"{name}Token"
        | NodeField (name = name) -> name

    member this.Ty : string =
        match this with
        | TokenField _ -> "SyntaxToken"
        | NodeField (ty = ty) -> ty

    member this.IsOptional : bool =
        match this with
        | TokenField (optional = optional)
        | NodeField (optional = optional) -> optional

type AstNodeSrc = { Name : string ; Fields : Field list }

type AstEnumSrc = { Name : string ; Cases : string list }

type AstSrc =
    {
        Nodes : AstNodeSrc list
        Enums : AstEnumSrc list
        Lists : Map<string, AstListSrc>
        Errors : string list
    }

    static member Zero : AstSrc =
        {
            Nodes = []
            Enums = []
            Lists = Map.empty
            Errors = []
        }

    member this.PushList (name : string) (src : AstListSrc) : AstSrc =
        { this with
            Lists = this.Lists.Add (name, src)
        }

    member this.PushUnion (src : AstEnumSrc) : AstSrc = { this with Enums = src :: this.Enums }

    member this.PushNode (src : AstNodeSrc) : AstSrc = { this with Nodes = src :: this.Nodes }

    member this.PushError (error : string) : AstSrc =
        { this with
            Errors = error :: this.Errors
        }

    member this.IsList (name : string) : bool = this.Lists.ContainsKey name

    member this.Sort () : AstSrc =
        let sortedNodes = this.Nodes |> List.sortBy _.Name

        let sortedEnums =
            this.Enums
            |> Seq.map (fun e -> { e with Cases = e.Cases |> List.sort })
            |> Seq.sortBy _.Name
            |> Seq.toList

        let errors = this.Errors |> List.sort

        { this with
            Nodes = sortedNodes
            Enums = sortedEnums
            Errors = errors
        }

[<AutoOpen>]
module AstConstants =
    [<Literal>]
    let SYNTAX_ELEMENT_TYPE = "SyntaxElement"

type CommaList =
    {
        NodeName : string
        SeparatorName : string
        TrailingSeparator : bool
    }

    // (T (',' T)* ','?)
    // (T (',' T)*)
    static member FromGrammar (grammar : Grammar) (rules : Rule list) : CommaList option =
        let data =
            match rules with
            | [ RNode node ; RRep repeat ; ROpt trailingSeparator ] -> Some (node, repeat, Some trailingSeparator)
            | [ RNode node ; RRep repeat ] -> Some (node, repeat, None)
            | _ -> None

        match data with
        | Some (node, RSeq [ comma ; RNode n ], trailingSeparator) ->
            let separatorMatchesTrailing =
                match trailingSeparator with
                | Some trailing -> comma = trailing
                | _ -> true

            if n = node && separatorMatchesTrailing then
                let separatorName =
                    match comma with
                    | RToken t -> grammar.Token(t).Name
                    | _ -> failwith "Separator must be a token"

                Some
                    {
                        NodeName = grammar.Node(node).Name
                        SeparatorName = separatorName
                        TrailingSeparator = trailingSeparator.IsSome
                    }
            else
                None
        | _ -> None

[<RequireQualifiedAccess>]
type NodeRuleClassification =
    | Union of string list
    | Node
    | Error
    | List of elementName : string * separator : AstListSeparatorConfiguration option

    static member FromGrammar (grammar : Grammar) (rule : Rule) : NodeRuleClassification =
        match rule with
        | RAlt alternatives ->
            alternatives
            |> Seq.map (
                function
                | RNode it -> Some (grammar.Node(it).Name)
                | _ -> None
            )
            |> Option.sequence
            |> function
                | Some names -> NodeRuleClassification.Union (names |> Seq.toList)
                | None -> NodeRuleClassification.Node
        | RRep rule ->
            let elementType =
                match rule with
                | RNode n -> grammar.Node(n).Name
                | _ -> failwith "Lists should only be over node types"

            if elementType = SYNTAX_ELEMENT_TYPE then
                NodeRuleClassification.Error
            else
                NodeRuleClassification.List (elementType, None)
        | RSeq rules ->
            match CommaList.FromGrammar grammar rules with
            | Some commaList ->
                NodeRuleClassification.List (
                    commaList.NodeName,
                    Some
                        {
                            SeparatorToken = commaList.SeparatorName
                            AllowTrailingSeparator = commaList.TrailingSeparator
                        }
                )
            | None -> NodeRuleClassification.Node
        | _ -> NodeRuleClassification.Node


module AstSrc =
    let rec handleRule
        (fields : Field list)
        (grammar : Grammar)
        (rule : Rule)
        (label : string option)
        (optional : bool)
        : Field list
        =
        let debugRule optional label (rule : Rule) =
            let label =
                match label with
                | Some l -> $"{l}:"
                | None -> ""
            $"{label}{rule.PrettyPrint(grammar)} (optional={optional})"

        let handleTokensInUnions (grammar : Grammar) rule label optional =
            let rule, optional =
                match rule with
                | ROpt rule -> (rule, true)
                | _ -> (rule, optional)

            match rule with
            | RAlt rules ->
                rules
                |> Seq.tryFoldOption
                    (fun tokenKinds rule ->
                        match rule with
                        | RToken token -> Some <| grammar.Token(token).Name :: tokenKinds
                        | _ -> None
                    )
                    []
                |> Option.map (fun tokenKinds -> TokenField (label, ManyTokens tokenKinds, optional))
            | _ -> None

        match rule with
        | RLabeled (label, rule) ->
            match handleTokensInUnions grammar rule label optional with
            | Some tokenField -> tokenField :: fields
            | None -> handleRule fields grammar rule (Some label) optional
        | ROpt rule -> handleRule fields grammar rule label true
        | RNode node ->
            let ty = grammar.Node(node).Name
            let name = label |> Option.defaultValue ty
            let field = NodeField (name, ty, optional)
            field :: fields
        | RToken token ->
            let tokenName = grammar.Token token
            let name = label |> Option.defaultValue tokenName.Name
            let kind = SingleToken name
            let field = TokenField (name, kind, optional)
            field :: fields
        | RRep _ -> failwith $"Create a list node for *many* children: {debugRule optional label rule}"
        | RAlt rules ->
            if optional then
                failwith "Alternates cannot be nested within an optional Rule. Use a new node to contain the alternates"

            rules
            |> List.fold (fun fields rule -> handleRule fields grammar rule label false) fields
        | RSeq rules ->
            rules
            |> List.fold (fun fields rule -> handleRule fields grammar rule label optional) fields

    let fromGrammar (grammar : Grammar) : AstSrc =
        let nodes =
            grammar.Nodes
            |> Seq.map grammar.Node
            |> Seq.filter (fun nd -> nd.Name <> SYNTAX_ELEMENT_TYPE)

        let mutable astSrc = AstSrc.Zero

        for node in nodes do
            let name = node.Name
            let rule = node.Rule

            match NodeRuleClassification.FromGrammar grammar rule with
            | NodeRuleClassification.Union cases ->
                let enumSrc = { Name = name ; Cases = cases }
                astSrc <- astSrc.PushUnion enumSrc
            | NodeRuleClassification.Error -> astSrc <- astSrc.PushError name
            | NodeRuleClassification.List (elementName, separator) ->
                let listSrc =
                    {
                        ElementName = elementName
                        Separator = separator
                    }

                astSrc <- astSrc.PushList name listSrc
            | NodeRuleClassification.Node ->
                let fields = handleRule [] grammar rule None false
                let nodeSrc = { Name = name ; Fields = List.rev fields }
                astSrc <- astSrc.PushNode nodeSrc

        astSrc
