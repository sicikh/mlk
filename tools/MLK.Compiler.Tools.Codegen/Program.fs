module MLK.Compiler.Tools.Codegen.Program

open System.IO
open MLK.Compiler.Tools.Codegen.Syntax
open MLK.Compiler.Tools.Codegen.Syntax.Writer

let projectRoot = __SOURCE_DIRECTORY__
let solutionRoot = Path.GetFullPath (Path.Combine (projectRoot, "..", ".."))

[<EntryPoint>]
let main _argv =
    let grammarInputPath = Path.Combine (projectRoot, "mlk.ungram")
    let astSrc = loadUngrammarFile grammarInputPath

    let syntaxDirPath = Path.Combine (solutionRoot, "src", "MLK.Compiler.Syntax", "Generated")

    let generateOptions = { SyntaxDirPath = syntaxDirPath }

    generateSyntax generateOptions mlkLanguageSrc astSrc

    0
