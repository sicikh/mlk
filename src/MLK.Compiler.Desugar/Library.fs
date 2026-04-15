namespace MLK.Compiler.Desugar

open Stdx
open MLK.Compiler.Syntax

type HirId = int

[<RequireQualifiedAccess>]
type HirExpr =
    | Int of id : HirId * value : int
    | Var of id : HirId * name : string
    | Let of id : HirId * name : string * expr : HirId * body : HirId
    | App of id : HirId * func : HirId * arg : HirId
    | Lam of id : HirId * param : string * body : HirId
    | If of id : HirId * cond : HirId * thenBranch : HirId * elseBranch : HirId
    | Hole of id : HirId

type DesugarCtx =
    {
        NextId : HirId
        ExprStore : Map<HirId, HirExpr>
    }

    member this.Id =
        let id = this.NextId
        id, { this with NextId = id + 1 }

    member this.AddExpr (id : HirId) (expr : HirExpr) : DesugarCtx =
        { this with
            ExprStore = this.ExprStore.Add (id, expr)
        }

module Desugar =
    let emptyCtx = { NextId = 0 ; ExprStore = Map.empty }

    let rec desugarExpr (ctx : DesugarCtx) (expr : Expr) : HirId * DesugarCtx =
        match expr with
        | ExprLiteral literal ->
            let intLiteral = literal.ValueToken.Value
            let value = int intLiteral.Green.Text
            let id, ctx = ctx.Id
            let expr = HirExpr.Int (id, value)
            id, ctx.AddExpr id expr
        | ExprApp appExpr ->
            let funcId, ctx = desugarExpr ctx appExpr.Func.Value
            let argId, ctx = desugarExpr ctx appExpr.Arg.Value
            let id, ctx = ctx.Id
            let expr = HirExpr.App (id, funcId, argId)
            id, ctx.AddExpr id expr
        // e1 op e2 => op e1 e2
        | ExprBin binExpr ->
            let funcName =
                match binExpr.OpToken.Value.Green.Text with
                | "+" -> "op_Add"
                | "-" -> "op_Sub"
                | "*" -> "op_Mul"
                | "/" -> "op_Div"
                | "=" -> "op_Eq"
                | "<>" -> "op_Neq"
                | _ -> failwith "unknown operator"

            let varId, ctx = ctx.Id
            let varExpr = HirExpr.Var (varId, funcName)
            let ctx = ctx.AddExpr varId varExpr

            let leftId, ctx = desugarExpr ctx binExpr.Left.Value
            let appId, ctx = ctx.Id
            let appExpr = HirExpr.App (appId, varId, leftId)
            let ctx = ctx.AddExpr appId appExpr

            let rightId, ctx = desugarExpr ctx binExpr.Right.Value
            let appId2, ctx = ctx.Id
            let appExpr2 = HirExpr.App (appId2, appId, rightId)
            let ctx = ctx.AddExpr appId2 appExpr2
            appId2, ctx
        | ExprErr _errExpr ->
            let id, ctx = ctx.Id
            let expr = HirExpr.Hole id
            id, ctx.AddExpr id expr
        | ExprFun funExpr ->
            let paramName = funExpr.Arg.Value.ValueToken.Value.Green.Text
            let bodyId, ctx = desugarExpr ctx funExpr.Body.Value
            let id, ctx = ctx.Id
            let expr = HirExpr.Lam (id, paramName, bodyId)
            id, ctx.AddExpr id expr
        | ExprIf ifExpr ->
            let condId, ctx = desugarExpr ctx ifExpr.Cond.Value
            let thenId, ctx = desugarExpr ctx ifExpr.ThenBranch.Value
            let elseId, ctx = desugarExpr ctx ifExpr.ElseBranch.Value
            let id, ctx = ctx.Id
            let expr = HirExpr.If (id, condId, thenId, elseId)
            id, ctx.AddExpr id expr
        | ExprLet letExpr ->
            let name = letExpr.Decl.Value.Name.Value.ValueToken.Value.Green.Text
            let exprId, ctx = desugarExpr ctx letExpr.Decl.Value.Expr.Value
            let bodyId, ctx = desugarExpr ctx letExpr.Body.Value
            let id, ctx = ctx.Id
            let expr = HirExpr.Let (id, name, exprId, bodyId)
            id, ctx.AddExpr id expr
        | ExprVar varExpr ->
            let name = varExpr.Name.Value.ValueToken.Value.Green.Text
            let id, ctx = ctx.Id
            let expr = HirExpr.Var (id, name)
            id, ctx.AddExpr id expr

        | ExprParen parenExpr -> desugarExpr ctx parenExpr.Expr.Value

module JsTranspile =
    [<Literal>]
    let JS_STD =
        """
const op_Add = (a) => (b) => a + b;
const op_Sub = (a) => (b) => a - b;
const op_Mul = (a) => (b) => a * b;
const op_Div = (a) => (b) => a / b;
const op_Eq = (a) => (b) => a === b;
const op_Neq = (a) => (b) => a !== b;
"""

    let transpileExpr (ctx : DesugarCtx) (id : HirId) : string =
        let rec transpileExpr (ctx : DesugarCtx) (id : HirId) : string =
            match ctx.ExprStore[id] with
            | HirExpr.Int (_, value) -> string value
            | HirExpr.Var (_, name) -> name
            | HirExpr.Lam (_, param, bodyId) ->
                let body = transpileExpr ctx bodyId
                $"({param}) => {body}"
            | HirExpr.App (_, funcId, argId) ->
                let func = transpileExpr ctx funcId
                let arg = transpileExpr ctx argId
                $"{func}({arg})"
            | HirExpr.If (_, condId, thenId, elseId) ->
                let cond = transpileExpr ctx condId
                let thenBranch = transpileExpr ctx thenId
                let elseBranch = transpileExpr ctx elseId
                $"({cond} ? {thenBranch} : {elseBranch})"
            | HirExpr.Let (_, name, exprId, bodyId) ->
                let expr = transpileExpr ctx exprId
                let body = transpileExpr ctx bodyId
                // TODO: introduce rec and nonrec let
                // $"(({name}) => {body})({expr})"
                $"(() => {{ const {name} = {expr}; return {body}; }})()"
            | HirExpr.Hole _ -> "undefined"

        JS_STD + "\n" + transpileExpr ctx id
