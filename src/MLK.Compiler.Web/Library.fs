namespace MLK.Compiler.Web

open MLK.Compiler
open MLK.Compiler.Parser
open MLK.Compiler.Text

module Parser =
    let parseSource (sourceText : string) =
        let events, trivias, diags = parseRoot sourceText
        let sink = LosslessTreeSink (sourceText, trivias)
        ParseEvent.processEvents sink diags events
        let tree = sink.Finish ()
        tree, diags
