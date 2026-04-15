module MLK.Compiler.Tools.Codegen.Syntax.Writer

open System.IO
open MLK.Compiler.Ungrammar
open MLK.Compiler.Tools.Codegen.Syntax.Kinds
open MLK.Compiler.Tools.Codegen.Syntax.Nodes

let writeAndFormat (filePath : string) (contents : string) : unit =
    // TODO: call fantomas
    File.WriteAllText (filePath, contents)

type GenerateOptions = { SyntaxDirPath : string }

let generateSyntax (options : GenerateOptions) (languageSrc : ILanguageSrc) (astSrc : AstSrc) : unit =
    let syntaxKindsPath = Path.Combine (options.SyntaxDirPath, "Kind.fs")
    let syntaxKinds = generateSyntaxKinds languageSrc astSrc

    let syntaxNodesPath = Path.Combine (options.SyntaxDirPath, "Node.fs")
    let syntaxNodes = generateSyntaxNodes languageSrc astSrc

    writeAndFormat syntaxKindsPath syntaxKinds
    writeAndFormat syntaxNodesPath syntaxNodes

let loadUngrammarFile (path : string) : AstSrc =
    let input = File.ReadAllText path
    let grammar = Grammar.Parse input
    AstSrc.fromGrammar grammar
