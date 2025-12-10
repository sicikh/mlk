module MLK.Compiler.Parser.Tests

open MLK.Compiler.Parser

open NUnit.Framework


[<SetUp>]
let Setup () =
    ()

[<Test>]
let Test1 () =
    let events, diags = parseRoot "[f(]"

    printfn "%A" events
    printfn "%A" diags

    Assert.Pass()
