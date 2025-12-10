module MLK.Compiler.Parser.Tests

open MLK.Compiler.Parser

open NUnit.Framework

let debugPrintEvents (events : ParseEvent list) =
    // print with indentation
    let rec printEvents (events : ParseEvent list) (indent : int) =
        match events with
        | [] -> ()
        | e :: rest ->
            match e with
            | StartEvent kind ->
                let indentStr = String.replicate indent "  "
                printfn "%sStart: %A" indentStr kind
                printEvents rest (indent + 1)
            | FinishEvent ->
                let indentStr = String.replicate (indent - 1) "  "
                printfn "%sEnd" indentStr
                printEvents rest (indent - 1)
            | TokenEvent (kind, _endOffset) ->
                let indentStr = String.replicate indent "  "
                printfn "%sToken: %A" indentStr kind
                printEvents rest indent
    printEvents events 0

[<SetUp>]
let Setup () =
    ()

[<Test>]
let Test1 () =
    let events, _diags = parseRoot "1 + 2 * 3"

    debugPrintEvents events

    Assert.Pass()
