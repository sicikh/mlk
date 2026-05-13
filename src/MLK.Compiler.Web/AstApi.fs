module MLK.Compiler.Web.AstApi

open MLK.Compiler.Parser
open MLK.Compiler.Text
open MLK.Compiler.Syntax

type Range = int * int

type Node =
    {
        name : string
        ``type`` : string
        range : Range
        children : Node list
    }

type Diagnostic = { message : string }

type AstResponse =
    {
        diagnostics : Diagnostic list
        tree : Node option
    }

let private mapDiagnostic (d : ParseDiagnostic) : Diagnostic = { message = d.Message }

let private rangeToTuple (r : TextRange) : Range =
    (TextSize.toInt r.Start, TextSize.toInt r.End)

let private kindName (kind : SyntaxKind) : string =
    match SyntaxKind.toString kind with
    | ValueSome s -> s
    | ValueNone ->
        System.Enum.GetName (typeof<SyntaxKind>, kind)
        |> Option.ofObj
        |> Option.defaultValue "Unknown"

let rec private mapGenericTree (t : GenericTree) : Node =
    match t with
    | Token (kind, text, range, _, _) ->
        let kindNameStr = kindName kind

        {
            name = text
            ``type`` = kindNameStr
            range = rangeToTuple range
            children = []
        }
    | Node (kind, children, range) ->
        let kindNameStr = kindName kind

        {
            name = kindNameStr
            ``type`` = kindNameStr
            range = rangeToTuple range
            children = children |> List.map mapGenericTree
        }

let buildAstFromSource (code : string) : AstResponse =
    let events, trivias, diagnostics = parseRoot code
    let sink = LosslessTreeSink (code, trivias)
    ParseEvent.processEvents sink diagnostics events
    let treeOpt = sink.Finish ()

    {
        diagnostics = diagnostics |> List.map mapDiagnostic
        tree = treeOpt |> Option.map mapGenericTree
    }
