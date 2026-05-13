module MLK.Compiler.Web.TranspileApi

let transpileToJs (source : string) : string =
    // TODO:
    let line = $"console.log(\"{source}\");"
    line + "\n" + line + "\n" + line + "\n" + line
