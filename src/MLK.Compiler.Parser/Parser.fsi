[<AutoOpen>]
module MLK.Compiler.Parser.Impl

open MLK.Compiler.Parser

val parseRoot : sourceText : string -> ParseEvent list * ParseDiagnostic list
