[<AutoOpen>]
module MLK.Compiler.Parser.Impl

open MLK.Compiler.Parser

val parseRoot : sourceText : string -> ParseEvent list * Trivia list * ParseDiagnostic list
