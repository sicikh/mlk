[<AutoOpen>]
module MLK.Compiler.Parser.Impl

open MLK.Compiler.Parser
open MLK.Compiler.Syntax

val parseRoot : sourceText : string -> ParseEvent list * Trivia list * ParseDiagnostic list

val exprRoot : source : string -> events : ParseEvent list -> trivias : Trivia list -> Expr
