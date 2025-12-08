module MLK.Compiler.Tools.Codegen.Program

open System.IO
open MLK.Compiler.Ungrammar
open MLK.Compiler.Tools.Codegen.Ast

let projectRoot = __SOURCE_DIRECTORY__
let solutionRoot = Path.GetFullPath (Path.Combine (projectRoot, "..", ".."))

[<EntryPoint>]
let main _argv =
    let grammarInputPath = Path.Combine (projectRoot, "mlk.ungram")
    let grammarInput = File.ReadAllText grammarInputPath
    let grammar = Grammar.Parse grammarInput
    let _astSrc = AstSrc.fromGrammar grammar

    0
