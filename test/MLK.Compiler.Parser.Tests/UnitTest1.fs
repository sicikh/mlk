module MLK.Compiler.Parser.Tests

open Stdx
open MLK.Compiler.Fusca
open MLK.Compiler.Syntax
open MLK.Compiler.Text
open MLK.Compiler.Parser
open MLK.Compiler.Desugar

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
    let source = "console.log 42"
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
//    let source = """
//let fix = fun f -> fun x -> f (fix f) x
//let factabs = fun fact -> fun x -> if x = 0 then 1 else x * fact (x - 1)
//fix factabs 5
//"""
    let source = """
let res = 42
console.log res
alert res
"""
    let events, trivias, diags = parseRoot source
    let tree =
        let sink = LosslessTreeSink(source, trivias)
        ParseEvent.processEvents sink diags events
        sink.Finish()

    match tree with
    | None -> Assert.Fail("Tree construction failed")
    | Some tree ->
        // debugPrintEvents source events
        printfn ""
        printfn "%s" <| tree.DebugPrint (Some source)
        printfn "\nDiagnostics:"
        diags |> List.iter (printfn "%A")

        let expr = exprRoot source events trivias
        let dCtx = Desugar.emptyCtx
        let (expr, dCtx) = Desugar.desugarExpr dCtx expr
        let transpiled = JsTranspile.transpileExpr dCtx expr
        printfn "\nTranspiled:\n%s" transpiled

    Assert.Pass()
