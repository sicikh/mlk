module MLK.Compiler.Tools.Codegen.Syntax.Kinds

open Stdx

let generateSyntaxKinds (languageSrc : ILanguageSrc) (astSrc : AstSrc) : string =
    let keywords = languageSrc.Keywords |> List.sort
    let literals = languageSrc.Literals |> List.sort
    let punct = languageSrc.Punct |> List.sortBy fst
    let tokens = languageSrc.Tokens |> List.sort
    let lists = astSrc.Lists |> Map.keys |> Seq.toList |> List.sort

    let allKeywords =
        keywords |> List.map (fun kw -> String.toPascalCase $"{kw}Kw")

    let punctSyntaxKinds = punct |> List.map snd

    let nodesSyntaxKinds =
        astSrc.Nodes
        |> List.map _.Name
        |> List.append (astSrc.Enums |> List.map _.Name)
        |> List.append lists
        |> List.append astSrc.Errors
        |> List.sort

    let allSyntaxKindsValues =
        "Tombstone" :: "Eof" :: (punctSyntaxKinds @ allKeywords @ tokens @ nodesSyntaxKinds)

    let syntaxKindsValues =
        allSyntaxKindsValues
        |> Seq.mapi (fun i kind -> $"    | {kind} = {i}uy")
        |> String.concat "\n"

    let valueToEnumValue v = $"SyntaxKind.{v}"

    let toCaseWithIndent n (s : string) =
        let indent = String.replicate n " "
        $"{indent}| {s}"

    let valuesToCases values =
        let trueCases =
            values
            |> Seq.map valueToEnumValue
            |> Seq.map (toCaseWithIndent 8)
            |> String.concat "\n"

        let falseCase = toCaseWithIndent 8 "_ -> false"
        $"{trueCases} -> true\n{falseCase}"

    let isPunctCases = valuesToCases punctSyntaxKinds
    let isLiteralCases = valuesToCases literals
    let isKeywordCases = valuesToCases allKeywords
    let isListCases = valuesToCases lists

    let kwStringAssocs =
        languageSrc.Keywords
        |> List.map (fun kw -> kw, valueToEnumValue (String.toPascalCase $"{kw}Kw"))

    let fromKeywordCases =
        kwStringAssocs
        |> Seq.map (fun (kw, enumVal) -> toCaseWithIndent 8 $"\"{kw}\" -> ValueSome {enumVal}")
        |> Seq.append
        <| Seq.singleton (toCaseWithIndent 8 "_ -> ValueNone")
        |> String.concat "\n"

    let toStringCases =
        kwStringAssocs
        |> Seq.map (fun (kw, enumVal) -> toCaseWithIndent 8 $"{enumVal} -> ValueSome \"{kw}\"")
        |> Seq.append
        <| [
            toCaseWithIndent 8 "SyntaxKind.Eof -> ValueSome \"EOF\""
            toCaseWithIndent 8 "SyntaxKind.StringLiteral -> ValueSome \"string literal\""
            toCaseWithIndent 8 "_ -> ValueNone"
        ]
        |> String.concat "\n"

    let tAssoc =
        (punct |> List.map (fun (kw, sk) -> kw, valueToEnumValue sk))
        @ kwStringAssocs
        @ [ "EOF", "SyntaxKind.Eof" ; "ident", "SyntaxKind.Ident" ]

    let tCases =
        tAssoc
        |> Seq.map (fun (str, enumVal) -> toCaseWithIndent 8 $"\"{str}\" -> {enumVal}")
        |> Seq.append
        <| Seq.singleton (toCaseWithIndent 8 "s -> failwithf \"Unknown syntax kind: %s\" s")
        |> String.concat "\n"

    $"\
namespace MLK.Compiler.Syntax

type SyntaxKind =
{syntaxKindsValues}

module SyntaxKind =
    let fromRaw (raw : RawSyntaxKind) : SyntaxKind =
        let v = byte raw.Value
        if v <= LanguagePrimitives.EnumToValue SyntaxKind.{allSyntaxKindsValues |> List.last} then
            LanguagePrimitives.EnumOfValue v
        else
            failwith \"Invalid raw SyntaxKind.\"

    let toRaw (kind : SyntaxKind) : RawSyntaxKind =
        RawSyntaxKind (uint16 (byte kind))

    let isPunct (kind : SyntaxKind) : bool =
        match kind with
{isPunctCases}

    let isLiteral (kind : SyntaxKind) : bool =
        match kind with
{isLiteralCases}

    let isKeyword (kind : SyntaxKind) : bool =
        match kind with
{isKeywordCases}

    let isList (kind : SyntaxKind) : bool =
        match kind with
{isListCases}

    let fromKeyword (keyword : string) : SyntaxKind voption =
        match keyword with
{fromKeywordCases}

    let toString (kind : SyntaxKind) : string voption =
        match kind with
{toStringCases}

[<AutoOpen>]
module SyntaxKindOps =
    // open System.Runtime.CompilerServices

    // [<MethodImpl(MethodImplOptions.AggressiveInlining)>]
    let inline t (str : string) : SyntaxKind =
        match str with
{tCases}

    let (|Punct|_|) (kind : SyntaxKind) : bool = SyntaxKind.isPunct kind
    let (|Literal|_|) (kind : SyntaxKind) : bool = SyntaxKind.isLiteral kind
    let (|Keyword|_|) (kind : SyntaxKind) : bool = SyntaxKind.isKeyword kind
    let (|List|_|) (kind : SyntaxKind) : bool = SyntaxKind.isList kind
    let (|T|_|) (str : string) (sk : SyntaxKind) : bool = t str = sk
"
