module MLK.Compiler.Tools.Codegen

open System.IO
open MLK.Compiler.Ungrammar

let grammarInputPath = "../mlk.ungram"
let grammarInput = File.ReadAllText grammarInputPath
let grammar = Grammar.Parse(grammarInput)
match grammar with
| Error err -> printfn $"Error: {err}"
| Ok _grammar ->
    failwith "todo"
