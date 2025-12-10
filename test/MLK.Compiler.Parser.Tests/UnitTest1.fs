module MLK.Compiler.Parser.Tests

open MLK.Compiler.Text
open MLK.Compiler.Parser

open NUnit.Framework

let debugPrintEvents (source : string) (events : ParseEvent list) =
    let getChar (offset : TextSize) =
        string <| source[TextSize.toInt offset - 1]

    // print with indentation
    let rec printEvents (events : ParseEvent list) (indent : int) =
        match events with
        | [] -> ()
        | e :: rest ->
            match e with
            | StartEvent (kind, forwardParent) ->
                let indentStr = String.replicate indent "  "
                printfn "%sStart: %A, forwardParent = %A" indentStr kind forwardParent
                printEvents rest (indent + 1)
            | FinishEvent ->
                let indentStr = String.replicate (indent - 1) "  "
                printfn "%sEnd" indentStr
                printEvents rest (indent - 1)
            | TokenEvent (kind, endOffset) ->
                let indentStr = String.replicate indent "  "
                printfn "%sToken: %A %s" indentStr kind (getChar endOffset)
                printEvents rest indent
    printEvents events 0

[<SetUp>]
let Setup () =
    ()

[<Test>]
let Test1 () =
    let source = "1 + 2 * +3 + +4"
    let events, _, diags = parseRoot source
    let processedEvents =
        let sink = DebugTreeSink()
        ParseEvent.processEvents sink diags events
        sink.Finish()

    debugPrintEvents source events

    printfn ""

    debugPrintEvents source processedEvents

    Assert.Pass()

[<Test>]
let Test2 () =
    let source = "(1 + 2) * (3 - 4)"
    let events, trivias, diags = parseRoot source
    let tree =
        let sink = LosslessTreeSink(source, trivias)
        ParseEvent.processEvents sink diags events
        sink.Finish()

    match tree with
    | None -> Assert.Fail("Tree construction failed")
    | Some tree ->
        printfn "%s" <| tree.DebugPrint (Some source)

    Assert.Pass()
