module MLK.Compiler.Tools.Codegen.Syntax.Writer

open System.IO
open MLK.Compiler.Ungrammar
open MLK.Compiler.Tools.Codegen.Syntax.Kinds

let writeAndFormat (filePath : string) (contents : string) : unit =
    // TODO: call fantomas
    File.WriteAllText (filePath, contents)

type GenerateOptions = { SyntaxDirPath : string }

let generateSyntax (options : GenerateOptions) (languageSrc : ILanguageSrc) (astSrc : AstSrc) : unit =
    let syntaxKindsPath = Path.Combine (options.SyntaxDirPath, "Kind.fs")
    let syntaxKinds = generateSyntaxKinds languageSrc astSrc

    writeAndFormat syntaxKindsPath syntaxKinds

let loadUngrammarFile (path : string) : AstSrc =
    let input = File.ReadAllText path
    let grammar = Grammar.Parse input
    AstSrc.fromGrammar grammar
