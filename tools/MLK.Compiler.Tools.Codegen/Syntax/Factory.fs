module MLK.Compiler.Tools.Codegen.Syntax.Factory

open MLK.Compiler.Tools.Codegen.Utils

let generateSyntaxFactory (languageSrc : ILanguageSrc) (astSrc : AstSrc) : string =
    let fieldPredicate (field : Field) : string =
        let tokenNameToSyntaxKind (token : string) : string =
            let methodName = languageSrc.ToMethodName token
            if languageSrc.Keywords |> List.contains token then
                $"SyntaxKind.{methodName}Kw"
            else
                $"SyntaxKind.{methodName}"

        match field with
        | NodeField (ty = ty) -> $"{ty}.CanCast (GreenElement.kind element)"
        | TokenField (kind = TokenKind.SingleToken expected) ->
            $"SyntaxKind.fromRaw (GreenElement.kind element) = {tokenNameToSyntaxKind expected}"
        | TokenField (kind = TokenKind.ManyTokens expected) ->
            let expected = expected |> List.map tokenNameToSyntaxKind |> String.concat " ; "
            $"let kind = SyntaxKind.fromRaw (GreenElement.kind element) in [{expected}] |> List.exists ((=) kind)"

    let normalNodeArms =
        astSrc.Nodes
        |> List.map (fun node ->
            let kind = node.Name

            let fields =
                node.Fields
                |> List.map (fun field ->
                    let predicate = fieldPredicate field

                    $"let slots, elements =
    match elements with
    | element :: tail when {predicate} -> RawNodeSlots.addPresent slots, tail
    | _ -> RawNodeSlots.addAbsent slots, elements"
                )

            let fields = String.concat "\n\n" fields

            $"""| SyntaxKind.{kind} ->
    let slots = RawNodeSlots.empty
    let (ParsedChildren elements) = children

{fields |> changeIndent 4}

    match elements with
    | [] -> RawNodeSlots.intoNode (SyntaxKind.toRaw kind) children slots
    | _ -> GreenNode.mk (SyntaxKind.toErr kind |> SyntaxKind.toRaw) (ParsedChildren.elements children |> Seq.map Some)"""
        )
        |> String.concat "\n\n"

    let errorKinds = astSrc.Errors |> List.map (fun name -> $"SyntaxKind.{name}") |> String.concat " | "

    $"""namespace MLK.Compiler.Syntax.Factory

open MLK.Compiler.Fusca
open MLK.Compiler.Syntax

module SyntaxFactory =
    let makeSyntax (kind : RawSyntaxKind) (children : ParsedChildren) : GreenNode =
        let kind = SyntaxKind.fromRaw kind
        match kind with
        | {errorKinds} -> GreenNode.mk (SyntaxKind.toRaw kind) (ParsedChildren.elements children |> Seq.map Some)

{normalNodeArms |> changeIndent 8}

        | _ -> failwith $"Is {{kind}} a token?"
    """
