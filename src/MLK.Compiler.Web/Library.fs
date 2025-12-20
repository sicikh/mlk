namespace MLK.Compiler.Web

open MLK.Compiler
open MLK.Compiler.Parser
open MLK.Compiler.Text

module Parser =
    let parseSource (sourceText : string) =
        let events, trivias, diags = parseRoot sourceText
        let sink = LosslessTreeSink (sourceText, trivias)
        ParseEvent.processEvents sink diags events
        let tree = sink.Finish ()
        tree, diags

module AstApi =
    type NodeDto = {
        name: string
        children: NodeDto list
        range: (int * int) option
        ``type``: string option
    }

    type DiagnosticDto = {
        message: string
    }

    type AstResponseDto = {
        diagnostics: DiagnosticDto list
        tree: NodeDto option
    }

    let textRangeToTuple (r: TextRange) : int * int =
        TextSize.toInt r.Start, TextSize.toInt r.End

    let rec genericTreeToDto (tree: GenericTree) : NodeDto =
        match tree with
        | Token (kind, text, range, _leadingTrivia, _trailingTrivia) ->
            {
                name = $"{kind} \"{text}\""
                children = []
                range = Some (textRangeToTuple range)
                ``type`` = None
            }
        | Node (kind, children, range) ->
            {
                name = kind.ToString()
                children = children |> List.map genericTreeToDto
                range = Some (textRangeToTuple range)
                ``type`` = None
            }

    let buildAstFromSource (source: string) : AstResponseDto =
        let events, trivias, diags = parseRoot source

        let sink = LosslessTreeSink(source, trivias)
        ParseEvent.processEvents sink diags events
        let treeOpt = sink.Finish()

        let diagDtos =
            diags
            |> List.map (fun d -> { message = d.Message })

        let treeDto =
            treeOpt
            |> Option.map genericTreeToDto

        {
            diagnostics = diagDtos
            tree = treeDto
        }
