namespace MLK.Compiler.Parser

open MLK.Compiler.Text
open MLK.Compiler.Syntax

[<Interface>]
type ITreeSink =
    abstract member Token : kind : SyntaxKind -> endOffset : TextSize -> unit
    abstract member StartNode : kind : SyntaxKind -> unit
    abstract member FinishNode : unit -> unit

    abstract member EmitErrors : diagnostics : ParseDiagnostic list -> unit

type ParseEvent =
    // TODO: do we need `forwardParent : uint option`?
    | StartEvent of kind : SyntaxKind
    | FinishEvent
    | TokenEvent of kind : SyntaxKind * endOffset : TextSize

module ParseEvent =
    let processEvents (sink : ITreeSink) (errors : ParseDiagnostic list) (events : ParseEvent list) : unit =
        sink.EmitErrors errors

        for event in events do
            match event with
            | StartEvent kind -> sink.StartNode kind
            | FinishEvent -> sink.FinishNode ()
            | TokenEvent (kind, endOffset) -> sink.Token kind endOffset
