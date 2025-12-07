module MLK.Compiler.Tools.Codegen

open System.IO
open MLK.Compiler.Ungrammar

let projectRoot = __SOURCE_DIRECTORY__
let solutionRoot = Path.GetFullPath(Path.Combine(projectRoot, "..", ".."))

[<EntryPoint>]
let main _argv =
    let grammarInputPath = Path.Combine(projectRoot, "mlk.ungram")
    let grammarInput = File.ReadAllText grammarInputPath
    let _grammar = Grammar.Parse(grammarInput)

    0
