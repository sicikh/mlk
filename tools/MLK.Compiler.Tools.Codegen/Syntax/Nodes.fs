module MLK.Compiler.Tools.Codegen.Syntax.Nodes

open Stdx

let generateSyntaxNodes (languageSrc : ILanguageSrc) (astSrc : AstSrc) : string =
    let withIndent (n : int) (s : string list) : string =
        let indent = String.replicate n " "
        s |> List.map (fun line -> $"{indent}{line}") |> String.concat "\n"

    let generateList (name : string) (list : AstListSrc) : string =
        let {
                ElementName = elementName
                Separator = separator
            } =
            list

        let listInterface =
            match separator with
            | None -> $"    interface IAstNodeList<{elementName}> with"
            | Some sep -> $"    interface IAstSeparatedList<{elementName}> with"

        $"""type {name} =
    private
    | {name} of SyntaxList

    interface IAstNodeFactory<{name}> with
        static member CanCast (kind : RawSyntaxKind) : bool =
             SyntaxKind.fromRaw kind = SyntaxKind.{name}

        static member Cast (node : SyntaxNode) : {name} option =
            if AstNode.canCast<{name}> node.Kind then
                Some ({name} (SyntaxList node))
            else
                None

    interface IAstNode with
        member this.Syntax =
            let ({name} syntaxList) = this
            syntaxList.Node

{listInterface}
        member this.SyntaxList =
            let ({name} syntaxList) = this
            syntaxList
"""

    let generateErr (name : string) : string =

        $"""type {name} =
    private
    | {name} of SyntaxNode

    interface IAstNodeFactory<{name}> with
        static member CanCast (kind : RawSyntaxKind) : bool =
             SyntaxKind.fromRaw kind = SyntaxKind.{name}

        static member Cast (node : SyntaxNode) : {name} option =
            if AstNode.canCast<{name}> node.Kind then
                Some ({name} node)
            else
                None

    interface IAstNode with
        member this.Syntax =
            let ({name} node) = this
            node

    member this.Items : SyntaxElementChildren =
        Support.elements (this :> IAstNode).Syntax
"""

    let generateEnum (enum : AstEnumSrc) =
        let { Name = name ; Cases = cases } = enum

        let caseNames =
            cases
            |> List.map (fun (case : string) ->
                let case =
                    if case.EndsWith name then
                        case.Substring (0, case.Length - name.Length)
                    else
                        case

                name + case
            )

        let cases = List.zip caseNames cases

        let casesDef =
            cases |> List.map (fun (case, var) -> $"| {case} of {var}") |> withIndent 4

        let canCastDef =
            cases |> List.map (fun (_, var) -> $"| SyntaxKind.{var}") |> withIndent 12

        let castDef =
            cases
            |> List.map (fun (case, var) ->
                if astSrc.Enums |> List.exists (fun e -> e.Name = var) then
                    $"| SyntaxKind.{var} -> AstNode.cast<{var}> node |> Option.map {case}"
                else if astSrc.Lists |> Map.containsKey var then
                    $"| SyntaxKind.{var} -> Some ({case} ({var} (SyntaxList node)))"
                else
                    $"| SyntaxKind.{var} -> Some ({case} ({var} node))"
            )
            |> withIndent 12

        let syntaxDef =
            cases
            |> List.map (fun (case, _) -> $"| {case} it -> (it :> IAstNode).Syntax")
            |> withIndent 12

        $"""type {name} =
{casesDef}

    interface IAstNodeFactory<{name}> with
        static member CanCast (kind : RawSyntaxKind) : bool =
            match SyntaxKind.fromRaw kind with
{canCastDef} -> true
            | _ -> false

        static member Cast (node : SyntaxNode) : {name} option =
            match SyntaxKind.fromRaw node.Kind with
{castDef}
            | _ -> None

    interface IAstNode with
        member this.Syntax : SyntaxNode =
            match this with
{syntaxDef}
"""

    let generateNode (node : AstNodeSrc) : string =
        let { Name = name ; Fields = fields } = node

        let fields =
            fields
            |> List.mapi (fun slot field ->
                {|
                    Ty = field.Ty
                    MethodName = field.MethodName languageSrc
                    ReturnType =
                        if field.IsOptional then
                            $"Option<{field.Ty}>"
                        else
                            $"SyntaxResult<{field.Ty}>"
                    Slot = slot
                    SupportCall =
                        match field with
                        | TokenField (optional = true) -> "token"
                        | TokenField (optional = false) -> "requiredToken"
                        | NodeField (optional = true ; ty = ty) -> $"node<{ty}>"
                        | NodeField (optional = false ; ty = ty) -> $"requiredNode<{ty}>"
                |}
            )

        let fieldsFields =
            fields
            |> List.map (fun field -> $"{field.MethodName} : {field.ReturnType}")
            |> withIndent 8

        let members =
            fields
            |> List.collect (fun field ->
                [
                    $"member this.{field.MethodName} : {field.ReturnType} ="
                    $"    let ({name} syntax) = this"
                    $"    Support.{field.SupportCall} {field.Slot}u syntax\n"
                ]
            )
            |> withIndent 4

        let asFields =
            fields
            |> List.map (fun field -> $"{field.MethodName} = this.{field.MethodName}")
            |> withIndent 12

        let recognizers =
            fields
            |> List.map (fun field ->
                $"let (|{field.MethodName}|) (value : {name}) : {field.ReturnType} = value.{field.MethodName}"
            )
            |> withIndent 4

        $"""type {name}Fields =
    {{
{fieldsFields}
    }}

type {name} =
    private
    | {name} of SyntaxNode

    interface IAstNodeFactory<{name}> with
        static member CanCast (kind : RawSyntaxKind) : bool =
             SyntaxKind.fromRaw kind = SyntaxKind.{name}

        static member Cast (node : SyntaxNode) : {name} option =
            if AstNode.canCast<{name}> node.Kind then
                Some ({name} node)
            else
                None

    interface IAstNode with
        member this.Syntax =
            let ({name} node) = this
            node

{members}
    member this.AsFields : {name}Fields =
        {{
{asFields}
        }}

module {name} =
{recognizers}
"""


    let nodes = astSrc.Nodes |> List.map generateNode |> String.concat "\n"
    let enums = astSrc.Enums |> List.map generateEnum |> String.concat "\n"
    let errs = astSrc.Errors |> List.map generateErr |> String.concat "\n"

    let lists =
        astSrc.Lists
        |> Map.toArray
        |> Array.map (fun (name, list) -> generateList name list)
        |> String.concat "\n"

    $"""namespace rec MLK.Compiler.Syntax

open MLK.Compiler.Fusca

{nodes}
{enums}
{lists}
{errs}
"""
