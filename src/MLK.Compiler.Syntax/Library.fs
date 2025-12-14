namespace MLK.Compiler.Syntax

open Stdx
open System.Text

[<AutoOpen>]
module OperatorEncoding =
    /// Try to encode an operator string to a valid identifier string.
    /// Returns None if op is not an operator.
    let encodeOpToIdent (op : string) : string option =
        let encodeOpIdent (opC : char) : string option =
            match opC with
            | '+' -> Some "Plus"
            | '-' -> Some "Minus"
            | '*' -> Some "Mult"
            | '/' -> Some "Div"
            | '=' -> Some "Eq"
            | '<' -> Some "Lt"
            | '>' -> Some "Gt"
            | '%' -> Some "Percent"
            | '&' -> Some "Amp"
            | '|' -> Some "Bar"
            | '^' -> Some "Hat"
            | '~' -> Some "Tilde"
            | '!' -> Some "Bang"
            | '?' -> Some "Qmark"
            | ':' -> Some "Colon"
            | '.' -> Some "Dot"
            | '$' -> Some "Dollar"
            | _ -> None

        let aux (builder : StringBuilder) opC =
            match encodeOpIdent opC with
            | Some enc -> Some <| builder.Append enc
            | None -> None

        op |> Seq.tryFoldOption aux (StringBuilder "op_") |> Option.map _.ToString()


    let decodeIdentToOp (_ident : string) : string option =
        failwith "todo"
