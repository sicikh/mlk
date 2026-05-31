namespace MLK.Compiler.Fusca

open MLK.Compiler.Text

#if !FABLE_COMPILER
[<Struct>]
#endif
type TriviaPieceKind =
    | Newline
    | Whitespace
    | SingleLineComment
    | MultiLineComment
    | Skipped

    member this.IsComment = this.IsSingleLineComment || this.IsMultiLineComment

[<Struct>]
type TriviaPiece =
    {
        Kind : TriviaPieceKind
        Length : TextSize
    }

    static member Create (kind : TriviaPieceKind, length : TextSize) = { Kind = kind ; Length = length }

    static member Newline (length : TextSize) =
        TriviaPiece.Create (TriviaPieceKind.Newline, length)

    static member Whitespace (length : TextSize) =
        TriviaPiece.Create (TriviaPieceKind.Whitespace, length)

    static member SingleLineComment (length : TextSize) =
        TriviaPiece.Create (TriviaPieceKind.SingleLineComment, length)

    static member MultiLineComment (length : TextSize) =
        TriviaPiece.Create (TriviaPieceKind.MultiLineComment, length)

    static member Skipped (length : TextSize) =
        TriviaPiece.Create (TriviaPieceKind.Skipped, length)
