module MLK.Compiler.Tools.Codegen.Utils

let withIndent (n : int) (s : string list) : string =
    let indent = String.replicate n " "
    s |> List.map (fun line -> $"{indent}{line}") |> String.concat "\n"

let changeIndent (n : int) (s : string) : string =
    let indent = String.replicate n " "
    s.Split('\n') |> Array.map (fun line -> $"{indent}{line}") |> String.concat "\n"
