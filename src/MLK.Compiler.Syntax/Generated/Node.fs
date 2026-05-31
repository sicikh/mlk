namespace rec MLK.Compiler.Syntax

open MLK.Compiler.Fusca

type AndPatFields =
    {
        Lhs : SyntaxResult<Pat>
        AmpToken : SyntaxResult<SyntaxToken>
        Rhs : SyntaxResult<Pat>
    }

type AndPat =
    private
    | AndPat of SyntaxNode

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.AndPat

    static member Cast (node : SyntaxNode) : AndPat option =
        if AstNode.canCast<AndPat> node.RawKind then
            Some (AndPat node)
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (AndPat node) = this
            node

    member this.Lhs : SyntaxResult<Pat> =
        let (AndPat syntax) = this
        Support.requiredNode<Pat> 0u syntax

    member this.AmpToken : SyntaxResult<SyntaxToken> =
        let (AndPat syntax) = this
        Support.requiredToken 1u syntax

    member this.Rhs : SyntaxResult<Pat> =
        let (AndPat syntax) = this
        Support.requiredNode<Pat> 2u syntax

    member this.AsFields : AndPatFields =
        {
            Lhs = this.Lhs
            AmpToken = this.AmpToken
            Rhs = this.Rhs
        }

module AndPat =
    let (|Lhs|) (value : AndPat) : SyntaxResult<Pat> = value.Lhs
    let (|AmpToken|) (value : AndPat) : SyntaxResult<SyntaxToken> = value.AmpToken
    let (|Rhs|) (value : AndPat) : SyntaxResult<Pat> = value.Rhs

type AppExprFields =
    {
        Func : SyntaxResult<Expr>
        Arg : SyntaxResult<Expr>
    }

type AppExpr =
    private
    | AppExpr of SyntaxNode

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.AppExpr

    static member Cast (node : SyntaxNode) : AppExpr option =
        if AstNode.canCast<AppExpr> node.RawKind then
            Some (AppExpr node)
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (AppExpr node) = this
            node

    member this.Func : SyntaxResult<Expr> =
        let (AppExpr syntax) = this
        Support.requiredNode<Expr> 0u syntax

    member this.Arg : SyntaxResult<Expr> =
        let (AppExpr syntax) = this
        Support.requiredNode<Expr> 1u syntax

    member this.AsFields : AppExprFields =
        {
            Func = this.Func
            Arg = this.Arg
        }

module AppExpr =
    let (|Func|) (value : AppExpr) : SyntaxResult<Expr> = value.Func
    let (|Arg|) (value : AppExpr) : SyntaxResult<Expr> = value.Arg

type AsPatFields =
    {
        Lhs : SyntaxResult<Pat>
        AsToken : SyntaxResult<SyntaxToken>
        Rhs : SyntaxResult<Pat>
    }

type AsPat =
    private
    | AsPat of SyntaxNode

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.AsPat

    static member Cast (node : SyntaxNode) : AsPat option =
        if AstNode.canCast<AsPat> node.RawKind then
            Some (AsPat node)
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (AsPat node) = this
            node

    member this.Lhs : SyntaxResult<Pat> =
        let (AsPat syntax) = this
        Support.requiredNode<Pat> 0u syntax

    member this.AsToken : SyntaxResult<SyntaxToken> =
        let (AsPat syntax) = this
        Support.requiredToken 1u syntax

    member this.Rhs : SyntaxResult<Pat> =
        let (AsPat syntax) = this
        Support.requiredNode<Pat> 2u syntax

    member this.AsFields : AsPatFields =
        {
            Lhs = this.Lhs
            AsToken = this.AsToken
            Rhs = this.Rhs
        }

module AsPat =
    let (|Lhs|) (value : AsPat) : SyntaxResult<Pat> = value.Lhs
    let (|AsToken|) (value : AsPat) : SyntaxResult<SyntaxToken> = value.AsToken
    let (|Rhs|) (value : AsPat) : SyntaxResult<Pat> = value.Rhs

type BinExprFields =
    {
        Left : SyntaxResult<Expr>
        OpToken : SyntaxResult<SyntaxToken>
        Right : SyntaxResult<Expr>
    }

type BinExpr =
    private
    | BinExpr of SyntaxNode

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.BinExpr

    static member Cast (node : SyntaxNode) : BinExpr option =
        if AstNode.canCast<BinExpr> node.RawKind then
            Some (BinExpr node)
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (BinExpr node) = this
            node

    member this.Left : SyntaxResult<Expr> =
        let (BinExpr syntax) = this
        Support.requiredNode<Expr> 0u syntax

    member this.OpToken : SyntaxResult<SyntaxToken> =
        let (BinExpr syntax) = this
        Support.requiredToken 1u syntax

    member this.Right : SyntaxResult<Expr> =
        let (BinExpr syntax) = this
        Support.requiredNode<Expr> 2u syntax

    member this.AsFields : BinExprFields =
        {
            Left = this.Left
            OpToken = this.OpToken
            Right = this.Right
        }

module BinExpr =
    let (|Left|) (value : BinExpr) : SyntaxResult<Expr> = value.Left
    let (|OpToken|) (value : BinExpr) : SyntaxResult<SyntaxToken> = value.OpToken
    let (|Right|) (value : BinExpr) : SyntaxResult<Expr> = value.Right

type BindingFields =
    {
        Pat : SyntaxResult<Pat>
        EqToken : SyntaxResult<SyntaxToken>
        Expr : SyntaxResult<Expr>
    }

type Binding =
    private
    | Binding of SyntaxNode

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.Binding

    static member Cast (node : SyntaxNode) : Binding option =
        if AstNode.canCast<Binding> node.RawKind then
            Some (Binding node)
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (Binding node) = this
            node

    member this.Pat : SyntaxResult<Pat> =
        let (Binding syntax) = this
        Support.requiredNode<Pat> 0u syntax

    member this.EqToken : SyntaxResult<SyntaxToken> =
        let (Binding syntax) = this
        Support.requiredToken 1u syntax

    member this.Expr : SyntaxResult<Expr> =
        let (Binding syntax) = this
        Support.requiredNode<Expr> 2u syntax

    member this.AsFields : BindingFields =
        {
            Pat = this.Pat
            EqToken = this.EqToken
            Expr = this.Expr
        }

module Binding =
    let (|Pat|) (value : Binding) : SyntaxResult<Pat> = value.Pat
    let (|EqToken|) (value : Binding) : SyntaxResult<SyntaxToken> = value.EqToken
    let (|Expr|) (value : Binding) : SyntaxResult<Expr> = value.Expr

type BoolLiteralExprFields =
    {
        ValueToken : SyntaxResult<SyntaxToken>
    }

type BoolLiteralExpr =
    private
    | BoolLiteralExpr of SyntaxNode

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.BoolLiteralExpr

    static member Cast (node : SyntaxNode) : BoolLiteralExpr option =
        if AstNode.canCast<BoolLiteralExpr> node.RawKind then
            Some (BoolLiteralExpr node)
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (BoolLiteralExpr node) = this
            node

    member this.ValueToken : SyntaxResult<SyntaxToken> =
        let (BoolLiteralExpr syntax) = this
        Support.requiredToken 0u syntax

    member this.AsFields : BoolLiteralExprFields =
        {
            ValueToken = this.ValueToken
        }

module BoolLiteralExpr =
    let (|ValueToken|) (value : BoolLiteralExpr) : SyntaxResult<SyntaxToken> = value.ValueToken

type CharLiteralExprFields =
    {
        ValueToken : SyntaxResult<SyntaxToken>
    }

type CharLiteralExpr =
    private
    | CharLiteralExpr of SyntaxNode

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.CharLiteralExpr

    static member Cast (node : SyntaxNode) : CharLiteralExpr option =
        if AstNode.canCast<CharLiteralExpr> node.RawKind then
            Some (CharLiteralExpr node)
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (CharLiteralExpr node) = this
            node

    member this.ValueToken : SyntaxResult<SyntaxToken> =
        let (CharLiteralExpr syntax) = this
        Support.requiredToken 0u syntax

    member this.AsFields : CharLiteralExprFields =
        {
            ValueToken = this.ValueToken
        }

module CharLiteralExpr =
    let (|ValueToken|) (value : CharLiteralExpr) : SyntaxResult<SyntaxToken> = value.ValueToken

type ConsPatFields =
    {
        Head : SyntaxResult<Pat>
        Colon2Token : SyntaxResult<SyntaxToken>
        Tail : SyntaxResult<Pat>
    }

type ConsPat =
    private
    | ConsPat of SyntaxNode

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.ConsPat

    static member Cast (node : SyntaxNode) : ConsPat option =
        if AstNode.canCast<ConsPat> node.RawKind then
            Some (ConsPat node)
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (ConsPat node) = this
            node

    member this.Head : SyntaxResult<Pat> =
        let (ConsPat syntax) = this
        Support.requiredNode<Pat> 0u syntax

    member this.Colon2Token : SyntaxResult<SyntaxToken> =
        let (ConsPat syntax) = this
        Support.requiredToken 1u syntax

    member this.Tail : SyntaxResult<Pat> =
        let (ConsPat syntax) = this
        Support.requiredNode<Pat> 2u syntax

    member this.AsFields : ConsPatFields =
        {
            Head = this.Head
            Colon2Token = this.Colon2Token
            Tail = this.Tail
        }

module ConsPat =
    let (|Head|) (value : ConsPat) : SyntaxResult<Pat> = value.Head
    let (|Colon2Token|) (value : ConsPat) : SyntaxResult<SyntaxToken> = value.Colon2Token
    let (|Tail|) (value : ConsPat) : SyntaxResult<Pat> = value.Tail

type FnTyFields =
    {
        Arg : SyntaxResult<Ty>
        ArrowToken : SyntaxResult<SyntaxToken>
        Ret : SyntaxResult<Ty>
    }

type FnTy =
    private
    | FnTy of SyntaxNode

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.FnTy

    static member Cast (node : SyntaxNode) : FnTy option =
        if AstNode.canCast<FnTy> node.RawKind then
            Some (FnTy node)
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (FnTy node) = this
            node

    member this.Arg : SyntaxResult<Ty> =
        let (FnTy syntax) = this
        Support.requiredNode<Ty> 0u syntax

    member this.ArrowToken : SyntaxResult<SyntaxToken> =
        let (FnTy syntax) = this
        Support.requiredToken 1u syntax

    member this.Ret : SyntaxResult<Ty> =
        let (FnTy syntax) = this
        Support.requiredNode<Ty> 2u syntax

    member this.AsFields : FnTyFields =
        {
            Arg = this.Arg
            ArrowToken = this.ArrowToken
            Ret = this.Ret
        }

module FnTy =
    let (|Arg|) (value : FnTy) : SyntaxResult<Ty> = value.Arg
    let (|ArrowToken|) (value : FnTy) : SyntaxResult<SyntaxToken> = value.ArrowToken
    let (|Ret|) (value : FnTy) : SyntaxResult<Ty> = value.Ret

type FunExprFields =
    {
        FunToken : SyntaxResult<SyntaxToken>
        Arg : SyntaxResult<Name>
        ArrowToken : SyntaxResult<SyntaxToken>
        Body : SyntaxResult<Expr>
    }

type FunExpr =
    private
    | FunExpr of SyntaxNode

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.FunExpr

    static member Cast (node : SyntaxNode) : FunExpr option =
        if AstNode.canCast<FunExpr> node.RawKind then
            Some (FunExpr node)
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (FunExpr node) = this
            node

    member this.FunToken : SyntaxResult<SyntaxToken> =
        let (FunExpr syntax) = this
        Support.requiredToken 0u syntax

    member this.Arg : SyntaxResult<Name> =
        let (FunExpr syntax) = this
        Support.requiredNode<Name> 1u syntax

    member this.ArrowToken : SyntaxResult<SyntaxToken> =
        let (FunExpr syntax) = this
        Support.requiredToken 2u syntax

    member this.Body : SyntaxResult<Expr> =
        let (FunExpr syntax) = this
        Support.requiredNode<Expr> 3u syntax

    member this.AsFields : FunExprFields =
        {
            FunToken = this.FunToken
            Arg = this.Arg
            ArrowToken = this.ArrowToken
            Body = this.Body
        }

module FunExpr =
    let (|FunToken|) (value : FunExpr) : SyntaxResult<SyntaxToken> = value.FunToken
    let (|Arg|) (value : FunExpr) : SyntaxResult<Name> = value.Arg
    let (|ArrowToken|) (value : FunExpr) : SyntaxResult<SyntaxToken> = value.ArrowToken
    let (|Body|) (value : FunExpr) : SyntaxResult<Expr> = value.Body

type FuncPatFields =
    {
        Func : SyntaxResult<Pat>
        Args : SyntaxResult<ArgPats>
    }

type FuncPat =
    private
    | FuncPat of SyntaxNode

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.FuncPat

    static member Cast (node : SyntaxNode) : FuncPat option =
        if AstNode.canCast<FuncPat> node.RawKind then
            Some (FuncPat node)
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (FuncPat node) = this
            node

    member this.Func : SyntaxResult<Pat> =
        let (FuncPat syntax) = this
        Support.requiredNode<Pat> 0u syntax

    member this.Args : SyntaxResult<ArgPats> =
        let (FuncPat syntax) = this
        Support.requiredNode<ArgPats> 1u syntax

    member this.AsFields : FuncPatFields =
        {
            Func = this.Func
            Args = this.Args
        }

module FuncPat =
    let (|Func|) (value : FuncPat) : SyntaxResult<Pat> = value.Func
    let (|Args|) (value : FuncPat) : SyntaxResult<ArgPats> = value.Args

type IfExprFields =
    {
        IfToken : SyntaxResult<SyntaxToken>
        Cond : SyntaxResult<Expr>
        ThenToken : SyntaxResult<SyntaxToken>
        ThenBranch : SyntaxResult<Expr>
        ElseToken : Option<SyntaxToken>
        ElseBranch : Option<Expr>
    }

type IfExpr =
    private
    | IfExpr of SyntaxNode

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.IfExpr

    static member Cast (node : SyntaxNode) : IfExpr option =
        if AstNode.canCast<IfExpr> node.RawKind then
            Some (IfExpr node)
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (IfExpr node) = this
            node

    member this.IfToken : SyntaxResult<SyntaxToken> =
        let (IfExpr syntax) = this
        Support.requiredToken 0u syntax

    member this.Cond : SyntaxResult<Expr> =
        let (IfExpr syntax) = this
        Support.requiredNode<Expr> 1u syntax

    member this.ThenToken : SyntaxResult<SyntaxToken> =
        let (IfExpr syntax) = this
        Support.requiredToken 2u syntax

    member this.ThenBranch : SyntaxResult<Expr> =
        let (IfExpr syntax) = this
        Support.requiredNode<Expr> 3u syntax

    member this.ElseToken : Option<SyntaxToken> =
        let (IfExpr syntax) = this
        Support.token 4u syntax

    member this.ElseBranch : Option<Expr> =
        let (IfExpr syntax) = this
        Support.node<Expr> 5u syntax

    member this.AsFields : IfExprFields =
        {
            IfToken = this.IfToken
            Cond = this.Cond
            ThenToken = this.ThenToken
            ThenBranch = this.ThenBranch
            ElseToken = this.ElseToken
            ElseBranch = this.ElseBranch
        }

module IfExpr =
    let (|IfToken|) (value : IfExpr) : SyntaxResult<SyntaxToken> = value.IfToken
    let (|Cond|) (value : IfExpr) : SyntaxResult<Expr> = value.Cond
    let (|ThenToken|) (value : IfExpr) : SyntaxResult<SyntaxToken> = value.ThenToken
    let (|ThenBranch|) (value : IfExpr) : SyntaxResult<Expr> = value.ThenBranch
    let (|ElseToken|) (value : IfExpr) : Option<SyntaxToken> = value.ElseToken
    let (|ElseBranch|) (value : IfExpr) : Option<Expr> = value.ElseBranch

type InferTyFields =
    {
        UnderscoreToken : SyntaxResult<SyntaxToken>
    }

type InferTy =
    private
    | InferTy of SyntaxNode

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.InferTy

    static member Cast (node : SyntaxNode) : InferTy option =
        if AstNode.canCast<InferTy> node.RawKind then
            Some (InferTy node)
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (InferTy node) = this
            node

    member this.UnderscoreToken : SyntaxResult<SyntaxToken> =
        let (InferTy syntax) = this
        Support.requiredToken 0u syntax

    member this.AsFields : InferTyFields =
        {
            UnderscoreToken = this.UnderscoreToken
        }

module InferTy =
    let (|UnderscoreToken|) (value : InferTy) : SyntaxResult<SyntaxToken> = value.UnderscoreToken

type InnerModuleDeclFields =
    {
        Preamble : Option<ModulePreamble>
        EqToken : SyntaxResult<SyntaxToken>
        Decls : SyntaxResult<ModuleDeclList>
    }

type InnerModuleDecl =
    private
    | InnerModuleDecl of SyntaxNode

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.InnerModuleDecl

    static member Cast (node : SyntaxNode) : InnerModuleDecl option =
        if AstNode.canCast<InnerModuleDecl> node.RawKind then
            Some (InnerModuleDecl node)
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (InnerModuleDecl node) = this
            node

    member this.Preamble : Option<ModulePreamble> =
        let (InnerModuleDecl syntax) = this
        Support.node<ModulePreamble> 0u syntax

    member this.EqToken : SyntaxResult<SyntaxToken> =
        let (InnerModuleDecl syntax) = this
        Support.requiredToken 1u syntax

    member this.Decls : SyntaxResult<ModuleDeclList> =
        let (InnerModuleDecl syntax) = this
        Support.requiredNode<ModuleDeclList> 2u syntax

    member this.AsFields : InnerModuleDeclFields =
        {
            Preamble = this.Preamble
            EqToken = this.EqToken
            Decls = this.Decls
        }

module InnerModuleDecl =
    let (|Preamble|) (value : InnerModuleDecl) : Option<ModulePreamble> = value.Preamble
    let (|EqToken|) (value : InnerModuleDecl) : SyntaxResult<SyntaxToken> = value.EqToken
    let (|Decls|) (value : InnerModuleDecl) : SyntaxResult<ModuleDeclList> = value.Decls

type IntLiteralExprFields =
    {
        ValueToken : SyntaxResult<SyntaxToken>
    }

type IntLiteralExpr =
    private
    | IntLiteralExpr of SyntaxNode

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.IntLiteralExpr

    static member Cast (node : SyntaxNode) : IntLiteralExpr option =
        if AstNode.canCast<IntLiteralExpr> node.RawKind then
            Some (IntLiteralExpr node)
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (IntLiteralExpr node) = this
            node

    member this.ValueToken : SyntaxResult<SyntaxToken> =
        let (IntLiteralExpr syntax) = this
        Support.requiredToken 0u syntax

    member this.AsFields : IntLiteralExprFields =
        {
            ValueToken = this.ValueToken
        }

module IntLiteralExpr =
    let (|ValueToken|) (value : IntLiteralExpr) : SyntaxResult<SyntaxToken> = value.ValueToken

type LetDeclFields =
    {
        LetToken : SyntaxResult<SyntaxToken>
        RecToken : Option<SyntaxToken>
        Name : SyntaxResult<Name>
        EqToken : SyntaxResult<SyntaxToken>
        Expr : SyntaxResult<Expr>
    }

type LetDecl =
    private
    | LetDecl of SyntaxNode

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.LetDecl

    static member Cast (node : SyntaxNode) : LetDecl option =
        if AstNode.canCast<LetDecl> node.RawKind then
            Some (LetDecl node)
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (LetDecl node) = this
            node

    member this.LetToken : SyntaxResult<SyntaxToken> =
        let (LetDecl syntax) = this
        Support.requiredToken 0u syntax

    member this.RecToken : Option<SyntaxToken> =
        let (LetDecl syntax) = this
        Support.token 1u syntax

    member this.Name : SyntaxResult<Name> =
        let (LetDecl syntax) = this
        Support.requiredNode<Name> 2u syntax

    member this.EqToken : SyntaxResult<SyntaxToken> =
        let (LetDecl syntax) = this
        Support.requiredToken 3u syntax

    member this.Expr : SyntaxResult<Expr> =
        let (LetDecl syntax) = this
        Support.requiredNode<Expr> 4u syntax

    member this.AsFields : LetDeclFields =
        {
            LetToken = this.LetToken
            RecToken = this.RecToken
            Name = this.Name
            EqToken = this.EqToken
            Expr = this.Expr
        }

module LetDecl =
    let (|LetToken|) (value : LetDecl) : SyntaxResult<SyntaxToken> = value.LetToken
    let (|RecToken|) (value : LetDecl) : Option<SyntaxToken> = value.RecToken
    let (|Name|) (value : LetDecl) : SyntaxResult<Name> = value.Name
    let (|EqToken|) (value : LetDecl) : SyntaxResult<SyntaxToken> = value.EqToken
    let (|Expr|) (value : LetDecl) : SyntaxResult<Expr> = value.Expr

type LetExprFields =
    {
        Decl : SyntaxResult<LetDecl>
        Body : SyntaxResult<Expr>
    }

type LetExpr =
    private
    | LetExpr of SyntaxNode

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.LetExpr

    static member Cast (node : SyntaxNode) : LetExpr option =
        if AstNode.canCast<LetExpr> node.RawKind then
            Some (LetExpr node)
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (LetExpr node) = this
            node

    member this.Decl : SyntaxResult<LetDecl> =
        let (LetExpr syntax) = this
        Support.requiredNode<LetDecl> 0u syntax

    member this.Body : SyntaxResult<Expr> =
        let (LetExpr syntax) = this
        Support.requiredNode<Expr> 1u syntax

    member this.AsFields : LetExprFields =
        {
            Decl = this.Decl
            Body = this.Body
        }

module LetExpr =
    let (|Decl|) (value : LetExpr) : SyntaxResult<LetDecl> = value.Decl
    let (|Body|) (value : LetExpr) : SyntaxResult<Expr> = value.Body

type ListExprFields =
    {
        LBracketToken : SyntaxResult<SyntaxToken>
        Elements : Option<ListExprElements>
        RBracketToken : SyntaxResult<SyntaxToken>
    }

type ListExpr =
    private
    | ListExpr of SyntaxNode

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.ListExpr

    static member Cast (node : SyntaxNode) : ListExpr option =
        if AstNode.canCast<ListExpr> node.RawKind then
            Some (ListExpr node)
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (ListExpr node) = this
            node

    member this.LBracketToken : SyntaxResult<SyntaxToken> =
        let (ListExpr syntax) = this
        Support.requiredToken 0u syntax

    member this.Elements : Option<ListExprElements> =
        let (ListExpr syntax) = this
        Support.node<ListExprElements> 1u syntax

    member this.RBracketToken : SyntaxResult<SyntaxToken> =
        let (ListExpr syntax) = this
        Support.requiredToken 2u syntax

    member this.AsFields : ListExprFields =
        {
            LBracketToken = this.LBracketToken
            Elements = this.Elements
            RBracketToken = this.RBracketToken
        }

module ListExpr =
    let (|LBracketToken|) (value : ListExpr) : SyntaxResult<SyntaxToken> = value.LBracketToken
    let (|Elements|) (value : ListExpr) : Option<ListExprElements> = value.Elements
    let (|RBracketToken|) (value : ListExpr) : SyntaxResult<SyntaxToken> = value.RBracketToken

type ListPatFields =
    {
        LBracketToken : SyntaxResult<SyntaxToken>
        Elements : Option<ListPatElements>
        RBracketToken : SyntaxResult<SyntaxToken>
    }

type ListPat =
    private
    | ListPat of SyntaxNode

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.ListPat

    static member Cast (node : SyntaxNode) : ListPat option =
        if AstNode.canCast<ListPat> node.RawKind then
            Some (ListPat node)
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (ListPat node) = this
            node

    member this.LBracketToken : SyntaxResult<SyntaxToken> =
        let (ListPat syntax) = this
        Support.requiredToken 0u syntax

    member this.Elements : Option<ListPatElements> =
        let (ListPat syntax) = this
        Support.node<ListPatElements> 1u syntax

    member this.RBracketToken : SyntaxResult<SyntaxToken> =
        let (ListPat syntax) = this
        Support.requiredToken 2u syntax

    member this.AsFields : ListPatFields =
        {
            LBracketToken = this.LBracketToken
            Elements = this.Elements
            RBracketToken = this.RBracketToken
        }

module ListPat =
    let (|LBracketToken|) (value : ListPat) : SyntaxResult<SyntaxToken> = value.LBracketToken
    let (|Elements|) (value : ListPat) : Option<ListPatElements> = value.Elements
    let (|RBracketToken|) (value : ListPat) : SyntaxResult<SyntaxToken> = value.RBracketToken

type MatchCaseFields =
    {
        Pat : SyntaxResult<Pat>
        Guard : Option<MatchGuard>
        ArrowToken : SyntaxResult<SyntaxToken>
        Body : SyntaxResult<Expr>
    }

type MatchCase =
    private
    | MatchCase of SyntaxNode

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.MatchCase

    static member Cast (node : SyntaxNode) : MatchCase option =
        if AstNode.canCast<MatchCase> node.RawKind then
            Some (MatchCase node)
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (MatchCase node) = this
            node

    member this.Pat : SyntaxResult<Pat> =
        let (MatchCase syntax) = this
        Support.requiredNode<Pat> 0u syntax

    member this.Guard : Option<MatchGuard> =
        let (MatchCase syntax) = this
        Support.node<MatchGuard> 1u syntax

    member this.ArrowToken : SyntaxResult<SyntaxToken> =
        let (MatchCase syntax) = this
        Support.requiredToken 2u syntax

    member this.Body : SyntaxResult<Expr> =
        let (MatchCase syntax) = this
        Support.requiredNode<Expr> 3u syntax

    member this.AsFields : MatchCaseFields =
        {
            Pat = this.Pat
            Guard = this.Guard
            ArrowToken = this.ArrowToken
            Body = this.Body
        }

module MatchCase =
    let (|Pat|) (value : MatchCase) : SyntaxResult<Pat> = value.Pat
    let (|Guard|) (value : MatchCase) : Option<MatchGuard> = value.Guard
    let (|ArrowToken|) (value : MatchCase) : SyntaxResult<SyntaxToken> = value.ArrowToken
    let (|Body|) (value : MatchCase) : SyntaxResult<Expr> = value.Body

type MatchExprFields =
    {
        MatchToken : SyntaxResult<SyntaxToken>
        Scrutinee : SyntaxResult<Expr>
        WithToken : SyntaxResult<SyntaxToken>
        LeadingPipeToken : Option<SyntaxToken>
        Cases : SyntaxResult<MatchCaseList>
    }

type MatchExpr =
    private
    | MatchExpr of SyntaxNode

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.MatchExpr

    static member Cast (node : SyntaxNode) : MatchExpr option =
        if AstNode.canCast<MatchExpr> node.RawKind then
            Some (MatchExpr node)
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (MatchExpr node) = this
            node

    member this.MatchToken : SyntaxResult<SyntaxToken> =
        let (MatchExpr syntax) = this
        Support.requiredToken 0u syntax

    member this.Scrutinee : SyntaxResult<Expr> =
        let (MatchExpr syntax) = this
        Support.requiredNode<Expr> 1u syntax

    member this.WithToken : SyntaxResult<SyntaxToken> =
        let (MatchExpr syntax) = this
        Support.requiredToken 2u syntax

    member this.LeadingPipeToken : Option<SyntaxToken> =
        let (MatchExpr syntax) = this
        Support.token 3u syntax

    member this.Cases : SyntaxResult<MatchCaseList> =
        let (MatchExpr syntax) = this
        Support.requiredNode<MatchCaseList> 4u syntax

    member this.AsFields : MatchExprFields =
        {
            MatchToken = this.MatchToken
            Scrutinee = this.Scrutinee
            WithToken = this.WithToken
            LeadingPipeToken = this.LeadingPipeToken
            Cases = this.Cases
        }

module MatchExpr =
    let (|MatchToken|) (value : MatchExpr) : SyntaxResult<SyntaxToken> = value.MatchToken
    let (|Scrutinee|) (value : MatchExpr) : SyntaxResult<Expr> = value.Scrutinee
    let (|WithToken|) (value : MatchExpr) : SyntaxResult<SyntaxToken> = value.WithToken
    let (|LeadingPipeToken|) (value : MatchExpr) : Option<SyntaxToken> = value.LeadingPipeToken
    let (|Cases|) (value : MatchExpr) : SyntaxResult<MatchCaseList> = value.Cases

type MatchGuardFields =
    {
        WhenToken : SyntaxResult<SyntaxToken>
        Cond : SyntaxResult<Expr>
    }

type MatchGuard =
    private
    | MatchGuard of SyntaxNode

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.MatchGuard

    static member Cast (node : SyntaxNode) : MatchGuard option =
        if AstNode.canCast<MatchGuard> node.RawKind then
            Some (MatchGuard node)
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (MatchGuard node) = this
            node

    member this.WhenToken : SyntaxResult<SyntaxToken> =
        let (MatchGuard syntax) = this
        Support.requiredToken 0u syntax

    member this.Cond : SyntaxResult<Expr> =
        let (MatchGuard syntax) = this
        Support.requiredNode<Expr> 1u syntax

    member this.AsFields : MatchGuardFields =
        {
            WhenToken = this.WhenToken
            Cond = this.Cond
        }

module MatchGuard =
    let (|WhenToken|) (value : MatchGuard) : SyntaxResult<SyntaxToken> = value.WhenToken
    let (|Cond|) (value : MatchGuard) : SyntaxResult<Expr> = value.Cond

type MemberAccessExprFields =
    {
        Target : SyntaxResult<Expr>
        DotToken : SyntaxResult<SyntaxToken>
        Member : SyntaxResult<Name>
    }

type MemberAccessExpr =
    private
    | MemberAccessExpr of SyntaxNode

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.MemberAccessExpr

    static member Cast (node : SyntaxNode) : MemberAccessExpr option =
        if AstNode.canCast<MemberAccessExpr> node.RawKind then
            Some (MemberAccessExpr node)
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (MemberAccessExpr node) = this
            node

    member this.Target : SyntaxResult<Expr> =
        let (MemberAccessExpr syntax) = this
        Support.requiredNode<Expr> 0u syntax

    member this.DotToken : SyntaxResult<SyntaxToken> =
        let (MemberAccessExpr syntax) = this
        Support.requiredToken 1u syntax

    member this.Member : SyntaxResult<Name> =
        let (MemberAccessExpr syntax) = this
        Support.requiredNode<Name> 2u syntax

    member this.AsFields : MemberAccessExprFields =
        {
            Target = this.Target
            DotToken = this.DotToken
            Member = this.Member
        }

module MemberAccessExpr =
    let (|Target|) (value : MemberAccessExpr) : SyntaxResult<Expr> = value.Target
    let (|DotToken|) (value : MemberAccessExpr) : SyntaxResult<SyntaxToken> = value.DotToken
    let (|Member|) (value : MemberAccessExpr) : SyntaxResult<Name> = value.Member

type ModulePreambleFields =
    {
        ModuleToken : SyntaxResult<SyntaxToken>
        Name : SyntaxResult<QName>
    }

type ModulePreamble =
    private
    | ModulePreamble of SyntaxNode

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.ModulePreamble

    static member Cast (node : SyntaxNode) : ModulePreamble option =
        if AstNode.canCast<ModulePreamble> node.RawKind then
            Some (ModulePreamble node)
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (ModulePreamble node) = this
            node

    member this.ModuleToken : SyntaxResult<SyntaxToken> =
        let (ModulePreamble syntax) = this
        Support.requiredToken 0u syntax

    member this.Name : SyntaxResult<QName> =
        let (ModulePreamble syntax) = this
        Support.requiredNode<QName> 1u syntax

    member this.AsFields : ModulePreambleFields =
        {
            ModuleToken = this.ModuleToken
            Name = this.Name
        }

module ModulePreamble =
    let (|ModuleToken|) (value : ModulePreamble) : SyntaxResult<SyntaxToken> = value.ModuleToken
    let (|Name|) (value : ModulePreamble) : SyntaxResult<QName> = value.Name

type ModuleRootFields =
    {
        Preamble : Option<ModulePreamble>
        Decls : SyntaxResult<ModuleDeclList>
        EofToken : SyntaxResult<SyntaxToken>
    }

type ModuleRoot =
    private
    | ModuleRoot of SyntaxNode

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.ModuleRoot

    static member Cast (node : SyntaxNode) : ModuleRoot option =
        if AstNode.canCast<ModuleRoot> node.RawKind then
            Some (ModuleRoot node)
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (ModuleRoot node) = this
            node

    member this.Preamble : Option<ModulePreamble> =
        let (ModuleRoot syntax) = this
        Support.node<ModulePreamble> 0u syntax

    member this.Decls : SyntaxResult<ModuleDeclList> =
        let (ModuleRoot syntax) = this
        Support.requiredNode<ModuleDeclList> 1u syntax

    member this.EofToken : SyntaxResult<SyntaxToken> =
        let (ModuleRoot syntax) = this
        Support.requiredToken 2u syntax

    member this.AsFields : ModuleRootFields =
        {
            Preamble = this.Preamble
            Decls = this.Decls
            EofToken = this.EofToken
        }

module ModuleRoot =
    let (|Preamble|) (value : ModuleRoot) : Option<ModulePreamble> = value.Preamble
    let (|Decls|) (value : ModuleRoot) : SyntaxResult<ModuleDeclList> = value.Decls
    let (|EofToken|) (value : ModuleRoot) : SyntaxResult<SyntaxToken> = value.EofToken

type NameFields =
    {
        ValueToken : SyntaxResult<SyntaxToken>
    }

type Name =
    private
    | Name of SyntaxNode

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.Name

    static member Cast (node : SyntaxNode) : Name option =
        if AstNode.canCast<Name> node.RawKind then
            Some (Name node)
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (Name node) = this
            node

    member this.ValueToken : SyntaxResult<SyntaxToken> =
        let (Name syntax) = this
        Support.requiredToken 0u syntax

    member this.AsFields : NameFields =
        {
            ValueToken = this.ValueToken
        }

module Name =
    let (|ValueToken|) (value : Name) : SyntaxResult<SyntaxToken> = value.ValueToken

type NamePatFieldFields =
    {
        Name : SyntaxResult<QName>
        EqToken : SyntaxResult<SyntaxToken>
        Pat : SyntaxResult<Pat>
    }

type NamePatField =
    private
    | NamePatField of SyntaxNode

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.NamePatField

    static member Cast (node : SyntaxNode) : NamePatField option =
        if AstNode.canCast<NamePatField> node.RawKind then
            Some (NamePatField node)
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (NamePatField node) = this
            node

    member this.Name : SyntaxResult<QName> =
        let (NamePatField syntax) = this
        Support.requiredNode<QName> 0u syntax

    member this.EqToken : SyntaxResult<SyntaxToken> =
        let (NamePatField syntax) = this
        Support.requiredToken 1u syntax

    member this.Pat : SyntaxResult<Pat> =
        let (NamePatField syntax) = this
        Support.requiredNode<Pat> 2u syntax

    member this.AsFields : NamePatFieldFields =
        {
            Name = this.Name
            EqToken = this.EqToken
            Pat = this.Pat
        }

module NamePatField =
    let (|Name|) (value : NamePatField) : SyntaxResult<QName> = value.Name
    let (|EqToken|) (value : NamePatField) : SyntaxResult<SyntaxToken> = value.EqToken
    let (|Pat|) (value : NamePatField) : SyntaxResult<Pat> = value.Pat

type NamedPatFields =
    {
        Name : SyntaxResult<QName>
    }

type NamedPat =
    private
    | NamedPat of SyntaxNode

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.NamedPat

    static member Cast (node : SyntaxNode) : NamedPat option =
        if AstNode.canCast<NamedPat> node.RawKind then
            Some (NamedPat node)
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (NamedPat node) = this
            node

    member this.Name : SyntaxResult<QName> =
        let (NamedPat syntax) = this
        Support.requiredNode<QName> 0u syntax

    member this.AsFields : NamedPatFields =
        {
            Name = this.Name
        }

module NamedPat =
    let (|Name|) (value : NamedPat) : SyntaxResult<QName> = value.Name

type OpenDeclFields =
    {
        OpenToken : SyntaxResult<SyntaxToken>
        Module : SyntaxResult<QName>
    }

type OpenDecl =
    private
    | OpenDecl of SyntaxNode

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.OpenDecl

    static member Cast (node : SyntaxNode) : OpenDecl option =
        if AstNode.canCast<OpenDecl> node.RawKind then
            Some (OpenDecl node)
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (OpenDecl node) = this
            node

    member this.OpenToken : SyntaxResult<SyntaxToken> =
        let (OpenDecl syntax) = this
        Support.requiredToken 0u syntax

    member this.Module : SyntaxResult<QName> =
        let (OpenDecl syntax) = this
        Support.requiredNode<QName> 1u syntax

    member this.AsFields : OpenDeclFields =
        {
            OpenToken = this.OpenToken
            Module = this.Module
        }

module OpenDecl =
    let (|OpenToken|) (value : OpenDecl) : SyntaxResult<SyntaxToken> = value.OpenToken
    let (|Module|) (value : OpenDecl) : SyntaxResult<QName> = value.Module

type OperatorFields =
    {
        ValueToken : SyntaxResult<SyntaxToken>
    }

type Operator =
    private
    | Operator of SyntaxNode

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.Operator

    static member Cast (node : SyntaxNode) : Operator option =
        if AstNode.canCast<Operator> node.RawKind then
            Some (Operator node)
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (Operator node) = this
            node

    member this.ValueToken : SyntaxResult<SyntaxToken> =
        let (Operator syntax) = this
        Support.requiredToken 0u syntax

    member this.AsFields : OperatorFields =
        {
            ValueToken = this.ValueToken
        }

module Operator =
    let (|ValueToken|) (value : Operator) : SyntaxResult<SyntaxToken> = value.ValueToken

type OrPatFields =
    {
        Lhs : SyntaxResult<Pat>
        PipeToken : SyntaxResult<SyntaxToken>
        Rhs : SyntaxResult<Pat>
    }

type OrPat =
    private
    | OrPat of SyntaxNode

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.OrPat

    static member Cast (node : SyntaxNode) : OrPat option =
        if AstNode.canCast<OrPat> node.RawKind then
            Some (OrPat node)
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (OrPat node) = this
            node

    member this.Lhs : SyntaxResult<Pat> =
        let (OrPat syntax) = this
        Support.requiredNode<Pat> 0u syntax

    member this.PipeToken : SyntaxResult<SyntaxToken> =
        let (OrPat syntax) = this
        Support.requiredToken 1u syntax

    member this.Rhs : SyntaxResult<Pat> =
        let (OrPat syntax) = this
        Support.requiredNode<Pat> 2u syntax

    member this.AsFields : OrPatFields =
        {
            Lhs = this.Lhs
            PipeToken = this.PipeToken
            Rhs = this.Rhs
        }

module OrPat =
    let (|Lhs|) (value : OrPat) : SyntaxResult<Pat> = value.Lhs
    let (|PipeToken|) (value : OrPat) : SyntaxResult<SyntaxToken> = value.PipeToken
    let (|Rhs|) (value : OrPat) : SyntaxResult<Pat> = value.Rhs

type ParenExprFields =
    {
        LParenToken : SyntaxResult<SyntaxToken>
        Expr : SyntaxResult<Expr>
        RParenToken : SyntaxResult<SyntaxToken>
    }

type ParenExpr =
    private
    | ParenExpr of SyntaxNode

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.ParenExpr

    static member Cast (node : SyntaxNode) : ParenExpr option =
        if AstNode.canCast<ParenExpr> node.RawKind then
            Some (ParenExpr node)
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (ParenExpr node) = this
            node

    member this.LParenToken : SyntaxResult<SyntaxToken> =
        let (ParenExpr syntax) = this
        Support.requiredToken 0u syntax

    member this.Expr : SyntaxResult<Expr> =
        let (ParenExpr syntax) = this
        Support.requiredNode<Expr> 1u syntax

    member this.RParenToken : SyntaxResult<SyntaxToken> =
        let (ParenExpr syntax) = this
        Support.requiredToken 2u syntax

    member this.AsFields : ParenExprFields =
        {
            LParenToken = this.LParenToken
            Expr = this.Expr
            RParenToken = this.RParenToken
        }

module ParenExpr =
    let (|LParenToken|) (value : ParenExpr) : SyntaxResult<SyntaxToken> = value.LParenToken
    let (|Expr|) (value : ParenExpr) : SyntaxResult<Expr> = value.Expr
    let (|RParenToken|) (value : ParenExpr) : SyntaxResult<SyntaxToken> = value.RParenToken

type ParenPatFields =
    {
        LParenToken : SyntaxResult<SyntaxToken>
        Pat : SyntaxResult<Pat>
        RParenToken : SyntaxResult<SyntaxToken>
    }

type ParenPat =
    private
    | ParenPat of SyntaxNode

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.ParenPat

    static member Cast (node : SyntaxNode) : ParenPat option =
        if AstNode.canCast<ParenPat> node.RawKind then
            Some (ParenPat node)
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (ParenPat node) = this
            node

    member this.LParenToken : SyntaxResult<SyntaxToken> =
        let (ParenPat syntax) = this
        Support.requiredToken 0u syntax

    member this.Pat : SyntaxResult<Pat> =
        let (ParenPat syntax) = this
        Support.requiredNode<Pat> 1u syntax

    member this.RParenToken : SyntaxResult<SyntaxToken> =
        let (ParenPat syntax) = this
        Support.requiredToken 2u syntax

    member this.AsFields : ParenPatFields =
        {
            LParenToken = this.LParenToken
            Pat = this.Pat
            RParenToken = this.RParenToken
        }

module ParenPat =
    let (|LParenToken|) (value : ParenPat) : SyntaxResult<SyntaxToken> = value.LParenToken
    let (|Pat|) (value : ParenPat) : SyntaxResult<Pat> = value.Pat
    let (|RParenToken|) (value : ParenPat) : SyntaxResult<SyntaxToken> = value.RParenToken

type ParenTyFields =
    {
        LParenToken : SyntaxResult<SyntaxToken>
        Ty : SyntaxResult<Ty>
        RParenToken : SyntaxResult<SyntaxToken>
    }

type ParenTy =
    private
    | ParenTy of SyntaxNode

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.ParenTy

    static member Cast (node : SyntaxNode) : ParenTy option =
        if AstNode.canCast<ParenTy> node.RawKind then
            Some (ParenTy node)
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (ParenTy node) = this
            node

    member this.LParenToken : SyntaxResult<SyntaxToken> =
        let (ParenTy syntax) = this
        Support.requiredToken 0u syntax

    member this.Ty : SyntaxResult<Ty> =
        let (ParenTy syntax) = this
        Support.requiredNode<Ty> 1u syntax

    member this.RParenToken : SyntaxResult<SyntaxToken> =
        let (ParenTy syntax) = this
        Support.requiredToken 2u syntax

    member this.AsFields : ParenTyFields =
        {
            LParenToken = this.LParenToken
            Ty = this.Ty
            RParenToken = this.RParenToken
        }

module ParenTy =
    let (|LParenToken|) (value : ParenTy) : SyntaxResult<SyntaxToken> = value.LParenToken
    let (|Ty|) (value : ParenTy) : SyntaxResult<Ty> = value.Ty
    let (|RParenToken|) (value : ParenTy) : SyntaxResult<SyntaxToken> = value.RParenToken

type QNameFields =
    {
        Qualifier : Option<QName>
        DotToken : Option<SyntaxToken>
        Segment : SyntaxResult<QNameSegment>
    }

type QName =
    private
    | QName of SyntaxNode

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.QName

    static member Cast (node : SyntaxNode) : QName option =
        if AstNode.canCast<QName> node.RawKind then
            Some (QName node)
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (QName node) = this
            node

    member this.Qualifier : Option<QName> =
        let (QName syntax) = this
        Support.node<QName> 0u syntax

    member this.DotToken : Option<SyntaxToken> =
        let (QName syntax) = this
        Support.token 1u syntax

    member this.Segment : SyntaxResult<QNameSegment> =
        let (QName syntax) = this
        Support.requiredNode<QNameSegment> 2u syntax

    member this.AsFields : QNameFields =
        {
            Qualifier = this.Qualifier
            DotToken = this.DotToken
            Segment = this.Segment
        }

module QName =
    let (|Qualifier|) (value : QName) : Option<QName> = value.Qualifier
    let (|DotToken|) (value : QName) : Option<SyntaxToken> = value.DotToken
    let (|Segment|) (value : QName) : SyntaxResult<QNameSegment> = value.Segment

type QNameSegmentFields =
    {
        Name : SyntaxResult<Name>
    }

type QNameSegment =
    private
    | QNameSegment of SyntaxNode

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.QNameSegment

    static member Cast (node : SyntaxNode) : QNameSegment option =
        if AstNode.canCast<QNameSegment> node.RawKind then
            Some (QNameSegment node)
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (QNameSegment node) = this
            node

    member this.Name : SyntaxResult<Name> =
        let (QNameSegment syntax) = this
        Support.requiredNode<Name> 0u syntax

    member this.AsFields : QNameSegmentFields =
        {
            Name = this.Name
        }

module QNameSegment =
    let (|Name|) (value : QNameSegment) : SyntaxResult<Name> = value.Name

type QTyFields =
    {
        Name : SyntaxResult<QName>
    }

type QTy =
    private
    | QTy of SyntaxNode

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.QTy

    static member Cast (node : SyntaxNode) : QTy option =
        if AstNode.canCast<QTy> node.RawKind then
            Some (QTy node)
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (QTy node) = this
            node

    member this.Name : SyntaxResult<QName> =
        let (QTy syntax) = this
        Support.requiredNode<QName> 0u syntax

    member this.AsFields : QTyFields =
        {
            Name = this.Name
        }

module QTy =
    let (|Name|) (value : QTy) : SyntaxResult<QName> = value.Name

type RecordPatFields =
    {
        LBraceToken : SyntaxResult<SyntaxToken>
        Fields : Option<RecordFields>
        RBraceToken : SyntaxResult<SyntaxToken>
    }

type RecordPat =
    private
    | RecordPat of SyntaxNode

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.RecordPat

    static member Cast (node : SyntaxNode) : RecordPat option =
        if AstNode.canCast<RecordPat> node.RawKind then
            Some (RecordPat node)
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (RecordPat node) = this
            node

    member this.LBraceToken : SyntaxResult<SyntaxToken> =
        let (RecordPat syntax) = this
        Support.requiredToken 0u syntax

    member this.Fields : Option<RecordFields> =
        let (RecordPat syntax) = this
        Support.node<RecordFields> 1u syntax

    member this.RBraceToken : SyntaxResult<SyntaxToken> =
        let (RecordPat syntax) = this
        Support.requiredToken 2u syntax

    member this.AsFields : RecordPatFields =
        {
            LBraceToken = this.LBraceToken
            Fields = this.Fields
            RBraceToken = this.RBraceToken
        }

module RecordPat =
    let (|LBraceToken|) (value : RecordPat) : SyntaxResult<SyntaxToken> = value.LBraceToken
    let (|Fields|) (value : RecordPat) : Option<RecordFields> = value.Fields
    let (|RBraceToken|) (value : RecordPat) : SyntaxResult<SyntaxToken> = value.RBraceToken

type SeqExprFields =
    {
        First : SyntaxResult<Expr>
        SemicolonToken : Option<SyntaxToken>
        Second : SyntaxResult<Expr>
    }

type SeqExpr =
    private
    | SeqExpr of SyntaxNode

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.SeqExpr

    static member Cast (node : SyntaxNode) : SeqExpr option =
        if AstNode.canCast<SeqExpr> node.RawKind then
            Some (SeqExpr node)
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (SeqExpr node) = this
            node

    member this.First : SyntaxResult<Expr> =
        let (SeqExpr syntax) = this
        Support.requiredNode<Expr> 0u syntax

    member this.SemicolonToken : Option<SyntaxToken> =
        let (SeqExpr syntax) = this
        Support.token 1u syntax

    member this.Second : SyntaxResult<Expr> =
        let (SeqExpr syntax) = this
        Support.requiredNode<Expr> 2u syntax

    member this.AsFields : SeqExprFields =
        {
            First = this.First
            SemicolonToken = this.SemicolonToken
            Second = this.Second
        }

module SeqExpr =
    let (|First|) (value : SeqExpr) : SyntaxResult<Expr> = value.First
    let (|SemicolonToken|) (value : SeqExpr) : Option<SyntaxToken> = value.SemicolonToken
    let (|Second|) (value : SeqExpr) : SyntaxResult<Expr> = value.Second

type StringLiteralExprFields =
    {
        ValueToken : SyntaxResult<SyntaxToken>
    }

type StringLiteralExpr =
    private
    | StringLiteralExpr of SyntaxNode

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.StringLiteralExpr

    static member Cast (node : SyntaxNode) : StringLiteralExpr option =
        if AstNode.canCast<StringLiteralExpr> node.RawKind then
            Some (StringLiteralExpr node)
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (StringLiteralExpr node) = this
            node

    member this.ValueToken : SyntaxResult<SyntaxToken> =
        let (StringLiteralExpr syntax) = this
        Support.requiredToken 0u syntax

    member this.AsFields : StringLiteralExprFields =
        {
            ValueToken = this.ValueToken
        }

module StringLiteralExpr =
    let (|ValueToken|) (value : StringLiteralExpr) : SyntaxResult<SyntaxToken> = value.ValueToken

type TypedExprFields =
    {
        Expr : SyntaxResult<Expr>
        ColonToken : SyntaxResult<SyntaxToken>
        Target : SyntaxResult<Ty>
    }

type TypedExpr =
    private
    | TypedExpr of SyntaxNode

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.TypedExpr

    static member Cast (node : SyntaxNode) : TypedExpr option =
        if AstNode.canCast<TypedExpr> node.RawKind then
            Some (TypedExpr node)
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (TypedExpr node) = this
            node

    member this.Expr : SyntaxResult<Expr> =
        let (TypedExpr syntax) = this
        Support.requiredNode<Expr> 0u syntax

    member this.ColonToken : SyntaxResult<SyntaxToken> =
        let (TypedExpr syntax) = this
        Support.requiredToken 1u syntax

    member this.Target : SyntaxResult<Ty> =
        let (TypedExpr syntax) = this
        Support.requiredNode<Ty> 2u syntax

    member this.AsFields : TypedExprFields =
        {
            Expr = this.Expr
            ColonToken = this.ColonToken
            Target = this.Target
        }

module TypedExpr =
    let (|Expr|) (value : TypedExpr) : SyntaxResult<Expr> = value.Expr
    let (|ColonToken|) (value : TypedExpr) : SyntaxResult<SyntaxToken> = value.ColonToken
    let (|Target|) (value : TypedExpr) : SyntaxResult<Ty> = value.Target

type TypedPatFields =
    {
        Pat : SyntaxResult<Pat>
        ColonToken : SyntaxResult<SyntaxToken>
        Target : SyntaxResult<Ty>
    }

type TypedPat =
    private
    | TypedPat of SyntaxNode

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.TypedPat

    static member Cast (node : SyntaxNode) : TypedPat option =
        if AstNode.canCast<TypedPat> node.RawKind then
            Some (TypedPat node)
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (TypedPat node) = this
            node

    member this.Pat : SyntaxResult<Pat> =
        let (TypedPat syntax) = this
        Support.requiredNode<Pat> 0u syntax

    member this.ColonToken : SyntaxResult<SyntaxToken> =
        let (TypedPat syntax) = this
        Support.requiredToken 1u syntax

    member this.Target : SyntaxResult<Ty> =
        let (TypedPat syntax) = this
        Support.requiredNode<Ty> 2u syntax

    member this.AsFields : TypedPatFields =
        {
            Pat = this.Pat
            ColonToken = this.ColonToken
            Target = this.Target
        }

module TypedPat =
    let (|Pat|) (value : TypedPat) : SyntaxResult<Pat> = value.Pat
    let (|ColonToken|) (value : TypedPat) : SyntaxResult<SyntaxToken> = value.ColonToken
    let (|Target|) (value : TypedPat) : SyntaxResult<Ty> = value.Target

type UnaryExprFields =
    {
        Op : SyntaxResult<QName>
        Operand : SyntaxResult<Expr>
    }

type UnaryExpr =
    private
    | UnaryExpr of SyntaxNode

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.UnaryExpr

    static member Cast (node : SyntaxNode) : UnaryExpr option =
        if AstNode.canCast<UnaryExpr> node.RawKind then
            Some (UnaryExpr node)
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (UnaryExpr node) = this
            node

    member this.Op : SyntaxResult<QName> =
        let (UnaryExpr syntax) = this
        Support.requiredNode<QName> 0u syntax

    member this.Operand : SyntaxResult<Expr> =
        let (UnaryExpr syntax) = this
        Support.requiredNode<Expr> 1u syntax

    member this.AsFields : UnaryExprFields =
        {
            Op = this.Op
            Operand = this.Operand
        }

module UnaryExpr =
    let (|Op|) (value : UnaryExpr) : SyntaxResult<QName> = value.Op
    let (|Operand|) (value : UnaryExpr) : SyntaxResult<Expr> = value.Operand

type UnitLiteralExprFields =
    {
        LParenToken : SyntaxResult<SyntaxToken>
        RParenToken : SyntaxResult<SyntaxToken>
    }

type UnitLiteralExpr =
    private
    | UnitLiteralExpr of SyntaxNode

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.UnitLiteralExpr

    static member Cast (node : SyntaxNode) : UnitLiteralExpr option =
        if AstNode.canCast<UnitLiteralExpr> node.RawKind then
            Some (UnitLiteralExpr node)
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (UnitLiteralExpr node) = this
            node

    member this.LParenToken : SyntaxResult<SyntaxToken> =
        let (UnitLiteralExpr syntax) = this
        Support.requiredToken 0u syntax

    member this.RParenToken : SyntaxResult<SyntaxToken> =
        let (UnitLiteralExpr syntax) = this
        Support.requiredToken 1u syntax

    member this.AsFields : UnitLiteralExprFields =
        {
            LParenToken = this.LParenToken
            RParenToken = this.RParenToken
        }

module UnitLiteralExpr =
    let (|LParenToken|) (value : UnitLiteralExpr) : SyntaxResult<SyntaxToken> = value.LParenToken
    let (|RParenToken|) (value : UnitLiteralExpr) : SyntaxResult<SyntaxToken> = value.RParenToken

type VarExprFields =
    {
        Name : SyntaxResult<Name>
    }

type VarExpr =
    private
    | VarExpr of SyntaxNode

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.VarExpr

    static member Cast (node : SyntaxNode) : VarExpr option =
        if AstNode.canCast<VarExpr> node.RawKind then
            Some (VarExpr node)
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (VarExpr node) = this
            node

    member this.Name : SyntaxResult<Name> =
        let (VarExpr syntax) = this
        Support.requiredNode<Name> 0u syntax

    member this.AsFields : VarExprFields =
        {
            Name = this.Name
        }

module VarExpr =
    let (|Name|) (value : VarExpr) : SyntaxResult<Name> = value.Name

type WildPatFields =
    {
        UnderscoreToken : SyntaxResult<SyntaxToken>
    }

type WildPat =
    private
    | WildPat of SyntaxNode

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.WildPat

    static member Cast (node : SyntaxNode) : WildPat option =
        if AstNode.canCast<WildPat> node.RawKind then
            Some (WildPat node)
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (WildPat node) = this
            node

    member this.UnderscoreToken : SyntaxResult<SyntaxToken> =
        let (WildPat syntax) = this
        Support.requiredToken 0u syntax

    member this.AsFields : WildPatFields =
        {
            UnderscoreToken = this.UnderscoreToken
        }

module WildPat =
    let (|UnderscoreToken|) (value : WildPat) : SyntaxResult<SyntaxToken> = value.UnderscoreToken

type ArgPat =
    | ArgPatNamePatField of NamePatField
    | ArgPatPat of Pat

    static member CanCast (kind : RawSyntaxKind) : bool =
        match SyntaxKind.fromRaw kind with
        | SyntaxKind.NamePatField -> true
        | _ when Pat.CanCast kind -> true
        | _ -> false

    static member Cast (node : SyntaxNode) : ArgPat option =
        match SyntaxKind.fromRaw node.RawKind with
        | SyntaxKind.NamePatField -> Some (ArgPatNamePatField (NamePatField node))
        | _ ->
            None
            |> Option.orElse (Pat.Cast node |> Option.map ArgPatPat)

    interface IAstNode with
        member this.Syntax : SyntaxNode =
            match this with
            | ArgPatNamePatField it -> (it :> IAstNode).Syntax
            | ArgPatPat it -> (it :> IAstNode).Syntax

type Expr =
    | ExprApp of AppExpr
    | ExprBin of BinExpr
    | ExprErr of ErrExpr
    | ExprFun of FunExpr
    | ExprIf of IfExpr
    | ExprLet of LetExpr
    | ExprLiteral of LiteralExpr
    | ExprMemberAccess of MemberAccessExpr
    | ExprParen of ParenExpr
    | ExprSeq of SeqExpr
    | ExprVar of VarExpr

    static member CanCast (kind : RawSyntaxKind) : bool =
        match SyntaxKind.fromRaw kind with
        | SyntaxKind.AppExpr
        | SyntaxKind.BinExpr
        | SyntaxKind.ErrExpr
        | SyntaxKind.FunExpr
        | SyntaxKind.IfExpr
        | SyntaxKind.LetExpr
        | SyntaxKind.MemberAccessExpr
        | SyntaxKind.ParenExpr
        | SyntaxKind.SeqExpr
        | SyntaxKind.VarExpr -> true
        | _ when LiteralExpr.CanCast kind -> true
        | _ -> false

    static member Cast (node : SyntaxNode) : Expr option =
        match SyntaxKind.fromRaw node.RawKind with
        | SyntaxKind.AppExpr -> Some (ExprApp (AppExpr node))
        | SyntaxKind.BinExpr -> Some (ExprBin (BinExpr node))
        | SyntaxKind.ErrExpr -> Some (ExprErr (ErrExpr node))
        | SyntaxKind.FunExpr -> Some (ExprFun (FunExpr node))
        | SyntaxKind.IfExpr -> Some (ExprIf (IfExpr node))
        | SyntaxKind.LetExpr -> Some (ExprLet (LetExpr node))
        | SyntaxKind.MemberAccessExpr -> Some (ExprMemberAccess (MemberAccessExpr node))
        | SyntaxKind.ParenExpr -> Some (ExprParen (ParenExpr node))
        | SyntaxKind.SeqExpr -> Some (ExprSeq (SeqExpr node))
        | SyntaxKind.VarExpr -> Some (ExprVar (VarExpr node))
        | _ ->
            None
            |> Option.orElse (LiteralExpr.Cast node |> Option.map ExprLiteral)

    interface IAstNode with
        member this.Syntax : SyntaxNode =
            match this with
            | ExprApp it -> (it :> IAstNode).Syntax
            | ExprBin it -> (it :> IAstNode).Syntax
            | ExprErr it -> (it :> IAstNode).Syntax
            | ExprFun it -> (it :> IAstNode).Syntax
            | ExprIf it -> (it :> IAstNode).Syntax
            | ExprLet it -> (it :> IAstNode).Syntax
            | ExprLiteral it -> (it :> IAstNode).Syntax
            | ExprMemberAccess it -> (it :> IAstNode).Syntax
            | ExprParen it -> (it :> IAstNode).Syntax
            | ExprSeq it -> (it :> IAstNode).Syntax
            | ExprVar it -> (it :> IAstNode).Syntax

type LiteralExpr =
    | LiteralExprBool of BoolLiteralExpr
    | LiteralExprChar of CharLiteralExpr
    | LiteralExprInt of IntLiteralExpr
    | LiteralExprString of StringLiteralExpr
    | LiteralExprUnit of UnitLiteralExpr

    static member CanCast (kind : RawSyntaxKind) : bool =
        match SyntaxKind.fromRaw kind with
        | SyntaxKind.BoolLiteralExpr
        | SyntaxKind.CharLiteralExpr
        | SyntaxKind.IntLiteralExpr
        | SyntaxKind.StringLiteralExpr
        | SyntaxKind.UnitLiteralExpr -> true
        | _ -> false

    static member Cast (node : SyntaxNode) : LiteralExpr option =
        match SyntaxKind.fromRaw node.RawKind with
        | SyntaxKind.BoolLiteralExpr -> Some (LiteralExprBool (BoolLiteralExpr node))
        | SyntaxKind.CharLiteralExpr -> Some (LiteralExprChar (CharLiteralExpr node))
        | SyntaxKind.IntLiteralExpr -> Some (LiteralExprInt (IntLiteralExpr node))
        | SyntaxKind.StringLiteralExpr -> Some (LiteralExprString (StringLiteralExpr node))
        | SyntaxKind.UnitLiteralExpr -> Some (LiteralExprUnit (UnitLiteralExpr node))
        | _ -> None

    interface IAstNode with
        member this.Syntax : SyntaxNode =
            match this with
            | LiteralExprBool it -> (it :> IAstNode).Syntax
            | LiteralExprChar it -> (it :> IAstNode).Syntax
            | LiteralExprInt it -> (it :> IAstNode).Syntax
            | LiteralExprString it -> (it :> IAstNode).Syntax
            | LiteralExprUnit it -> (it :> IAstNode).Syntax

type ModuleDecl =
    | ModuleDeclErrDecl of ErrDecl
    | ModuleDeclExpr of Expr
    | ModuleDeclInner of InnerModuleDecl
    | ModuleDeclLetDecl of LetDecl
    | ModuleDeclOpenDecl of OpenDecl

    static member CanCast (kind : RawSyntaxKind) : bool =
        match SyntaxKind.fromRaw kind with
        | SyntaxKind.ErrDecl
        | SyntaxKind.InnerModuleDecl
        | SyntaxKind.LetDecl
        | SyntaxKind.OpenDecl -> true
        | _ when Expr.CanCast kind -> true
        | _ -> false

    static member Cast (node : SyntaxNode) : ModuleDecl option =
        match SyntaxKind.fromRaw node.RawKind with
        | SyntaxKind.ErrDecl -> Some (ModuleDeclErrDecl (ErrDecl node))
        | SyntaxKind.InnerModuleDecl -> Some (ModuleDeclInner (InnerModuleDecl node))
        | SyntaxKind.LetDecl -> Some (ModuleDeclLetDecl (LetDecl node))
        | SyntaxKind.OpenDecl -> Some (ModuleDeclOpenDecl (OpenDecl node))
        | _ ->
            None
            |> Option.orElse (Expr.Cast node |> Option.map ModuleDeclExpr)

    interface IAstNode with
        member this.Syntax : SyntaxNode =
            match this with
            | ModuleDeclErrDecl it -> (it :> IAstNode).Syntax
            | ModuleDeclExpr it -> (it :> IAstNode).Syntax
            | ModuleDeclInner it -> (it :> IAstNode).Syntax
            | ModuleDeclLetDecl it -> (it :> IAstNode).Syntax
            | ModuleDeclOpenDecl it -> (it :> IAstNode).Syntax

type Pat =
    | PatAnd of AndPat
    | PatAs of AsPat
    | PatCons of ConsPat
    | PatErr of ErrPat
    | PatFunc of FuncPat
    | PatList of ListPat
    | PatLiteralExpr of LiteralExpr
    | PatNamed of NamedPat
    | PatOr of OrPat
    | PatParen of ParenPat
    | PatRecord of RecordPat
    | PatTuple of TuplePat
    | PatTyped of TypedPat
    | PatWild of WildPat

    static member CanCast (kind : RawSyntaxKind) : bool =
        match SyntaxKind.fromRaw kind with
        | SyntaxKind.AndPat
        | SyntaxKind.AsPat
        | SyntaxKind.ConsPat
        | SyntaxKind.ErrPat
        | SyntaxKind.FuncPat
        | SyntaxKind.ListPat
        | SyntaxKind.NamedPat
        | SyntaxKind.OrPat
        | SyntaxKind.ParenPat
        | SyntaxKind.RecordPat
        | SyntaxKind.TuplePat
        | SyntaxKind.TypedPat
        | SyntaxKind.WildPat -> true
        | _ when LiteralExpr.CanCast kind -> true
        | _ -> false

    static member Cast (node : SyntaxNode) : Pat option =
        match SyntaxKind.fromRaw node.RawKind with
        | SyntaxKind.AndPat -> Some (PatAnd (AndPat node))
        | SyntaxKind.AsPat -> Some (PatAs (AsPat node))
        | SyntaxKind.ConsPat -> Some (PatCons (ConsPat node))
        | SyntaxKind.ErrPat -> Some (PatErr (ErrPat node))
        | SyntaxKind.FuncPat -> Some (PatFunc (FuncPat node))
        | SyntaxKind.ListPat -> Some (PatList (ListPat node))
        | SyntaxKind.NamedPat -> Some (PatNamed (NamedPat node))
        | SyntaxKind.OrPat -> Some (PatOr (OrPat node))
        | SyntaxKind.ParenPat -> Some (PatParen (ParenPat node))
        | SyntaxKind.RecordPat -> Some (PatRecord (RecordPat node))
        | SyntaxKind.TuplePat -> Some (PatTuple (TuplePat (SyntaxList node)))
        | SyntaxKind.TypedPat -> Some (PatTyped (TypedPat node))
        | SyntaxKind.WildPat -> Some (PatWild (WildPat node))
        | _ ->
            None
            |> Option.orElse (LiteralExpr.Cast node |> Option.map PatLiteralExpr)

    interface IAstNode with
        member this.Syntax : SyntaxNode =
            match this with
            | PatAnd it -> (it :> IAstNode).Syntax
            | PatAs it -> (it :> IAstNode).Syntax
            | PatCons it -> (it :> IAstNode).Syntax
            | PatErr it -> (it :> IAstNode).Syntax
            | PatFunc it -> (it :> IAstNode).Syntax
            | PatList it -> (it :> IAstNode).Syntax
            | PatLiteralExpr it -> (it :> IAstNode).Syntax
            | PatNamed it -> (it :> IAstNode).Syntax
            | PatOr it -> (it :> IAstNode).Syntax
            | PatParen it -> (it :> IAstNode).Syntax
            | PatRecord it -> (it :> IAstNode).Syntax
            | PatTuple it -> (it :> IAstNode).Syntax
            | PatTyped it -> (it :> IAstNode).Syntax
            | PatWild it -> (it :> IAstNode).Syntax

type Ty =
    | TyErr of ErrTy
    | TyFn of FnTy
    | TyInfer of InferTy
    | TyParen of ParenTy
    | TyQ of QTy
    | TyTuple of TupleTy

    static member CanCast (kind : RawSyntaxKind) : bool =
        match SyntaxKind.fromRaw kind with
        | SyntaxKind.ErrTy
        | SyntaxKind.FnTy
        | SyntaxKind.InferTy
        | SyntaxKind.ParenTy
        | SyntaxKind.QTy
        | SyntaxKind.TupleTy -> true
        | _ -> false

    static member Cast (node : SyntaxNode) : Ty option =
        match SyntaxKind.fromRaw node.RawKind with
        | SyntaxKind.ErrTy -> Some (TyErr (ErrTy node))
        | SyntaxKind.FnTy -> Some (TyFn (FnTy node))
        | SyntaxKind.InferTy -> Some (TyInfer (InferTy node))
        | SyntaxKind.ParenTy -> Some (TyParen (ParenTy node))
        | SyntaxKind.QTy -> Some (TyQ (QTy node))
        | SyntaxKind.TupleTy -> Some (TyTuple (TupleTy (SyntaxList node)))
        | _ -> None

    interface IAstNode with
        member this.Syntax : SyntaxNode =
            match this with
            | TyErr it -> (it :> IAstNode).Syntax
            | TyFn it -> (it :> IAstNode).Syntax
            | TyInfer it -> (it :> IAstNode).Syntax
            | TyParen it -> (it :> IAstNode).Syntax
            | TyQ it -> (it :> IAstNode).Syntax
            | TyTuple it -> (it :> IAstNode).Syntax

type ArgPats =
    private
    | ArgPats of SyntaxList

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.ArgPats

    static member Cast (node : SyntaxNode) : ArgPats option =
        if AstNode.canCast<ArgPats> node.RawKind then
            Some (ArgPats (SyntaxList node))
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (ArgPats syntaxList) = this
            syntaxList.Node

    interface IAstNodeList<ArgPat> with
        member this.SyntaxList =
            let (ArgPats syntaxList) = this
            syntaxList

type ListExprElements =
    private
    | ListExprElements of SyntaxList

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.ListExprElements

    static member Cast (node : SyntaxNode) : ListExprElements option =
        if AstNode.canCast<ListExprElements> node.RawKind then
            Some (ListExprElements (SyntaxList node))
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (ListExprElements syntaxList) = this
            syntaxList.Node

    interface IAstSeparatedList<Expr> with
        member this.SyntaxList =
            let (ListExprElements syntaxList) = this
            syntaxList

type ListPatElements =
    private
    | ListPatElements of SyntaxList

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.ListPatElements

    static member Cast (node : SyntaxNode) : ListPatElements option =
        if AstNode.canCast<ListPatElements> node.RawKind then
            Some (ListPatElements (SyntaxList node))
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (ListPatElements syntaxList) = this
            syntaxList.Node

    interface IAstSeparatedList<Pat> with
        member this.SyntaxList =
            let (ListPatElements syntaxList) = this
            syntaxList

type MatchCaseList =
    private
    | MatchCaseList of SyntaxList

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.MatchCaseList

    static member Cast (node : SyntaxNode) : MatchCaseList option =
        if AstNode.canCast<MatchCaseList> node.RawKind then
            Some (MatchCaseList (SyntaxList node))
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (MatchCaseList syntaxList) = this
            syntaxList.Node

    interface IAstSeparatedList<MatchCase> with
        member this.SyntaxList =
            let (MatchCaseList syntaxList) = this
            syntaxList

type ModuleDeclList =
    private
    | ModuleDeclList of SyntaxList

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.ModuleDeclList

    static member Cast (node : SyntaxNode) : ModuleDeclList option =
        if AstNode.canCast<ModuleDeclList> node.RawKind then
            Some (ModuleDeclList (SyntaxList node))
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (ModuleDeclList syntaxList) = this
            syntaxList.Node

    interface IAstNodeList<ModuleDecl> with
        member this.SyntaxList =
            let (ModuleDeclList syntaxList) = this
            syntaxList

type RecordFields =
    private
    | RecordFields of SyntaxList

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.RecordFields

    static member Cast (node : SyntaxNode) : RecordFields option =
        if AstNode.canCast<RecordFields> node.RawKind then
            Some (RecordFields (SyntaxList node))
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (RecordFields syntaxList) = this
            syntaxList.Node

    interface IAstSeparatedList<NamePatField> with
        member this.SyntaxList =
            let (RecordFields syntaxList) = this
            syntaxList

type TupleExpr =
    private
    | TupleExpr of SyntaxList

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.TupleExpr

    static member Cast (node : SyntaxNode) : TupleExpr option =
        if AstNode.canCast<TupleExpr> node.RawKind then
            Some (TupleExpr (SyntaxList node))
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (TupleExpr syntaxList) = this
            syntaxList.Node

    interface IAstSeparatedList<Expr> with
        member this.SyntaxList =
            let (TupleExpr syntaxList) = this
            syntaxList

type TuplePat =
    private
    | TuplePat of SyntaxList

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.TuplePat

    static member Cast (node : SyntaxNode) : TuplePat option =
        if AstNode.canCast<TuplePat> node.RawKind then
            Some (TuplePat (SyntaxList node))
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (TuplePat syntaxList) = this
            syntaxList.Node

    interface IAstSeparatedList<Pat> with
        member this.SyntaxList =
            let (TuplePat syntaxList) = this
            syntaxList

type TupleTy =
    private
    | TupleTy of SyntaxList

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.TupleTy

    static member Cast (node : SyntaxNode) : TupleTy option =
        if AstNode.canCast<TupleTy> node.RawKind then
            Some (TupleTy (SyntaxList node))
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (TupleTy syntaxList) = this
            syntaxList.Node

    interface IAstSeparatedList<Ty> with
        member this.SyntaxList =
            let (TupleTy syntaxList) = this
            syntaxList

type ErrDecl =
    private
    | ErrDecl of SyntaxNode

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.ErrDecl

    static member Cast (node : SyntaxNode) : ErrDecl option =
        if AstNode.canCast<ErrDecl> node.RawKind then
            Some (ErrDecl node)
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (ErrDecl node) = this
            node

    member this.Items : SyntaxElementChildren =
        Support.elements (this :> IAstNode).Syntax

type ErrExpr =
    private
    | ErrExpr of SyntaxNode

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.ErrExpr

    static member Cast (node : SyntaxNode) : ErrExpr option =
        if AstNode.canCast<ErrExpr> node.RawKind then
            Some (ErrExpr node)
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (ErrExpr node) = this
            node

    member this.Items : SyntaxElementChildren =
        Support.elements (this :> IAstNode).Syntax

type ErrNode =
    private
    | ErrNode of SyntaxNode

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.ErrNode

    static member Cast (node : SyntaxNode) : ErrNode option =
        if AstNode.canCast<ErrNode> node.RawKind then
            Some (ErrNode node)
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (ErrNode node) = this
            node

    member this.Items : SyntaxElementChildren =
        Support.elements (this :> IAstNode).Syntax

type ErrPat =
    private
    | ErrPat of SyntaxNode

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.ErrPat

    static member Cast (node : SyntaxNode) : ErrPat option =
        if AstNode.canCast<ErrPat> node.RawKind then
            Some (ErrPat node)
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (ErrPat node) = this
            node

    member this.Items : SyntaxElementChildren =
        Support.elements (this :> IAstNode).Syntax

type ErrTy =
    private
    | ErrTy of SyntaxNode

    static member CanCast (kind : RawSyntaxKind) : bool =
         SyntaxKind.fromRaw kind = SyntaxKind.ErrTy

    static member Cast (node : SyntaxNode) : ErrTy option =
        if AstNode.canCast<ErrTy> node.RawKind then
            Some (ErrTy node)
        else
            None

    interface IAstNode with
        member this.Syntax =
            let (ErrTy node) = this
            node

    member this.Items : SyntaxElementChildren =
        Support.elements (this :> IAstNode).Syntax

