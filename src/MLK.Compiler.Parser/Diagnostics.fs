namespace MLK.Compiler.Parser

open MLK.Compiler.Text
open MLK.Compiler.Parser

type ParseDiagnostic =
    | ParseDiagnostic of message : string * range : TextRange

    member this.Message : string =
        match this with
        | ParseDiagnostic (message, _) -> message

    member this.Range : TextRange =
        match this with
        | ParseDiagnostic (_, range) -> range

module ParseDiagnostic =
    let create (message : string) (range : TextRange) : ParseDiagnostic = ParseDiagnostic (message, range)

    let mkSingleNode (name : string) (tokenSource : TokenSource) (range : TextRange) : ParseDiagnostic =
        let found =
            if tokenSource.IsEof then
                "the end of file"
            else
                let token = tokenSource.Input.Slice range
                $"'{token}'"

        let message = $"Expected {name} but instead found {found}."
        create message range
