namespace MLK.Compiler.Parser

open MLK.Compiler.Text

type ParseDiagnostic =
    | ParseDiagnostic of message : string * range : TextRange

    member this.Message : string =
        match this with
        | ParseDiagnostic (message, _) -> message

    member this.Range : TextRange =
        match this with
        | ParseDiagnostic (_, range) -> range
