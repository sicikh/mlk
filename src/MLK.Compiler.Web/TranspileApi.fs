module MLK.Compiler.Web.TranspileApi

open System
open MLK.Compiler.Parser
open MLK.Compiler.Desugar

let transpileToJs (source : string) : string =
    let events, trivias, _ = parseRoot source
    let expr = exprRoot source events trivias
    let desugarCtx = Desugar.emptyCtx
    let expr, desugarCtx = Desugar.desugarExpr desugarCtx expr
    let transpiled = JsTranspile.transpileExpr desugarCtx expr
    // Console.WriteLine transpiled
    JsTranspile.JS_STD + $"console.log(({transpiled}));"
