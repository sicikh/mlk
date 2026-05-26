namespace MLK.Compiler.Syntax.Factory

open MLK.Compiler.Fusca
open MLK.Compiler.Syntax

module SyntaxFactory =
    let makeSyntax (kind : RawSyntaxKind) (children : ParsedChildren) : GreenNode =
        let kind = SyntaxKind.fromRaw kind
        match kind with
        | SyntaxKind.ErrDecl | SyntaxKind.ErrExpr | SyntaxKind.ErrNode | SyntaxKind.ErrPat | SyntaxKind.ErrTy -> GreenNode.mk (SyntaxKind.toRaw kind) (ParsedChildren.elements children |> Seq.map Some)

        | SyntaxKind.AndPat ->
            let slots = RawNodeSlots.empty
            let (ParsedChildren elements) = children
        
            let slots, elements =
                match elements with
                | element :: tail when Pat.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when SyntaxKind.fromRaw (GreenElement.kind element) = SyntaxKind.Amp -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when Pat.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
        
            match elements with
            | [] -> RawNodeSlots.intoNode (SyntaxKind.toRaw kind) children slots
            | _ -> GreenNode.mk (SyntaxKind.toErr kind |> SyntaxKind.toRaw) (ParsedChildren.elements children |> Seq.map Some)
        
        | SyntaxKind.AppExpr ->
            let slots = RawNodeSlots.empty
            let (ParsedChildren elements) = children
        
            let slots, elements =
                match elements with
                | element :: tail when Expr.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when Expr.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
        
            match elements with
            | [] -> RawNodeSlots.intoNode (SyntaxKind.toRaw kind) children slots
            | _ -> GreenNode.mk (SyntaxKind.toErr kind |> SyntaxKind.toRaw) (ParsedChildren.elements children |> Seq.map Some)
        
        | SyntaxKind.AsPat ->
            let slots = RawNodeSlots.empty
            let (ParsedChildren elements) = children
        
            let slots, elements =
                match elements with
                | element :: tail when Pat.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when SyntaxKind.fromRaw (GreenElement.kind element) = SyntaxKind.AsKw -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when Pat.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
        
            match elements with
            | [] -> RawNodeSlots.intoNode (SyntaxKind.toRaw kind) children slots
            | _ -> GreenNode.mk (SyntaxKind.toErr kind |> SyntaxKind.toRaw) (ParsedChildren.elements children |> Seq.map Some)
        
        | SyntaxKind.BinExpr ->
            let slots = RawNodeSlots.empty
            let (ParsedChildren elements) = children
        
            let slots, elements =
                match elements with
                | element :: tail when Expr.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when SyntaxKind.fromRaw (GreenElement.kind element) = SyntaxKind.Operator -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when Expr.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
        
            match elements with
            | [] -> RawNodeSlots.intoNode (SyntaxKind.toRaw kind) children slots
            | _ -> GreenNode.mk (SyntaxKind.toErr kind |> SyntaxKind.toRaw) (ParsedChildren.elements children |> Seq.map Some)
        
        | SyntaxKind.Binding ->
            let slots = RawNodeSlots.empty
            let (ParsedChildren elements) = children
        
            let slots, elements =
                match elements with
                | element :: tail when Pat.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when SyntaxKind.fromRaw (GreenElement.kind element) = SyntaxKind.Eq -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when Expr.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
        
            match elements with
            | [] -> RawNodeSlots.intoNode (SyntaxKind.toRaw kind) children slots
            | _ -> GreenNode.mk (SyntaxKind.toErr kind |> SyntaxKind.toRaw) (ParsedChildren.elements children |> Seq.map Some)
        
        | SyntaxKind.BoolLiteralExpr ->
            let slots = RawNodeSlots.empty
            let (ParsedChildren elements) = children
        
            let slots, elements =
                match elements with
                | element :: tail when let kind = SyntaxKind.fromRaw (GreenElement.kind element) in [SyntaxKind.FalseKw ; SyntaxKind.TrueKw] |> List.exists ((=) kind) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
        
            match elements with
            | [] -> RawNodeSlots.intoNode (SyntaxKind.toRaw kind) children slots
            | _ -> GreenNode.mk (SyntaxKind.toErr kind |> SyntaxKind.toRaw) (ParsedChildren.elements children |> Seq.map Some)
        
        | SyntaxKind.CharLiteralExpr ->
            let slots = RawNodeSlots.empty
            let (ParsedChildren elements) = children
        
            let slots, elements =
                match elements with
                | element :: tail when SyntaxKind.fromRaw (GreenElement.kind element) = SyntaxKind.CharLiteral -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
        
            match elements with
            | [] -> RawNodeSlots.intoNode (SyntaxKind.toRaw kind) children slots
            | _ -> GreenNode.mk (SyntaxKind.toErr kind |> SyntaxKind.toRaw) (ParsedChildren.elements children |> Seq.map Some)
        
        | SyntaxKind.ConsPat ->
            let slots = RawNodeSlots.empty
            let (ParsedChildren elements) = children
        
            let slots, elements =
                match elements with
                | element :: tail when Pat.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when SyntaxKind.fromRaw (GreenElement.kind element) = SyntaxKind.Colon2 -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when Pat.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
        
            match elements with
            | [] -> RawNodeSlots.intoNode (SyntaxKind.toRaw kind) children slots
            | _ -> GreenNode.mk (SyntaxKind.toErr kind |> SyntaxKind.toRaw) (ParsedChildren.elements children |> Seq.map Some)
        
        | SyntaxKind.FnTy ->
            let slots = RawNodeSlots.empty
            let (ParsedChildren elements) = children
        
            let slots, elements =
                match elements with
                | element :: tail when Ty.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when SyntaxKind.fromRaw (GreenElement.kind element) = SyntaxKind.Arrow -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when Ty.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
        
            match elements with
            | [] -> RawNodeSlots.intoNode (SyntaxKind.toRaw kind) children slots
            | _ -> GreenNode.mk (SyntaxKind.toErr kind |> SyntaxKind.toRaw) (ParsedChildren.elements children |> Seq.map Some)
        
        | SyntaxKind.FunExpr ->
            let slots = RawNodeSlots.empty
            let (ParsedChildren elements) = children
        
            let slots, elements =
                match elements with
                | element :: tail when SyntaxKind.fromRaw (GreenElement.kind element) = SyntaxKind.FunKw -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when Name.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when SyntaxKind.fromRaw (GreenElement.kind element) = SyntaxKind.Arrow -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when Expr.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
        
            match elements with
            | [] -> RawNodeSlots.intoNode (SyntaxKind.toRaw kind) children slots
            | _ -> GreenNode.mk (SyntaxKind.toErr kind |> SyntaxKind.toRaw) (ParsedChildren.elements children |> Seq.map Some)
        
        | SyntaxKind.FuncPat ->
            let slots = RawNodeSlots.empty
            let (ParsedChildren elements) = children
        
            let slots, elements =
                match elements with
                | element :: tail when Pat.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when ArgPats.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
        
            match elements with
            | [] -> RawNodeSlots.intoNode (SyntaxKind.toRaw kind) children slots
            | _ -> GreenNode.mk (SyntaxKind.toErr kind |> SyntaxKind.toRaw) (ParsedChildren.elements children |> Seq.map Some)
        
        | SyntaxKind.IfExpr ->
            let slots = RawNodeSlots.empty
            let (ParsedChildren elements) = children
        
            let slots, elements =
                match elements with
                | element :: tail when SyntaxKind.fromRaw (GreenElement.kind element) = SyntaxKind.IfKw -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when Expr.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when SyntaxKind.fromRaw (GreenElement.kind element) = SyntaxKind.ThenKw -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when Expr.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when SyntaxKind.fromRaw (GreenElement.kind element) = SyntaxKind.ElseKw -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when Expr.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
        
            match elements with
            | [] -> RawNodeSlots.intoNode (SyntaxKind.toRaw kind) children slots
            | _ -> GreenNode.mk (SyntaxKind.toErr kind |> SyntaxKind.toRaw) (ParsedChildren.elements children |> Seq.map Some)
        
        | SyntaxKind.InferTy ->
            let slots = RawNodeSlots.empty
            let (ParsedChildren elements) = children
        
            let slots, elements =
                match elements with
                | element :: tail when SyntaxKind.fromRaw (GreenElement.kind element) = SyntaxKind.Underscore -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
        
            match elements with
            | [] -> RawNodeSlots.intoNode (SyntaxKind.toRaw kind) children slots
            | _ -> GreenNode.mk (SyntaxKind.toErr kind |> SyntaxKind.toRaw) (ParsedChildren.elements children |> Seq.map Some)
        
        | SyntaxKind.InnerModuleDecl ->
            let slots = RawNodeSlots.empty
            let (ParsedChildren elements) = children
        
            let slots, elements =
                match elements with
                | element :: tail when ModulePreamble.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when SyntaxKind.fromRaw (GreenElement.kind element) = SyntaxKind.Eq -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when ModuleDeclList.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
        
            match elements with
            | [] -> RawNodeSlots.intoNode (SyntaxKind.toRaw kind) children slots
            | _ -> GreenNode.mk (SyntaxKind.toErr kind |> SyntaxKind.toRaw) (ParsedChildren.elements children |> Seq.map Some)
        
        | SyntaxKind.IntLiteralExpr ->
            let slots = RawNodeSlots.empty
            let (ParsedChildren elements) = children
        
            let slots, elements =
                match elements with
                | element :: tail when SyntaxKind.fromRaw (GreenElement.kind element) = SyntaxKind.IntLiteral -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
        
            match elements with
            | [] -> RawNodeSlots.intoNode (SyntaxKind.toRaw kind) children slots
            | _ -> GreenNode.mk (SyntaxKind.toErr kind |> SyntaxKind.toRaw) (ParsedChildren.elements children |> Seq.map Some)
        
        | SyntaxKind.LetDecl ->
            let slots = RawNodeSlots.empty
            let (ParsedChildren elements) = children
        
            let slots, elements =
                match elements with
                | element :: tail when SyntaxKind.fromRaw (GreenElement.kind element) = SyntaxKind.LetKw -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when SyntaxKind.fromRaw (GreenElement.kind element) = SyntaxKind.RecKw -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when Name.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when SyntaxKind.fromRaw (GreenElement.kind element) = SyntaxKind.Eq -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when Expr.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
        
            match elements with
            | [] -> RawNodeSlots.intoNode (SyntaxKind.toRaw kind) children slots
            | _ -> GreenNode.mk (SyntaxKind.toErr kind |> SyntaxKind.toRaw) (ParsedChildren.elements children |> Seq.map Some)
        
        | SyntaxKind.LetExpr ->
            let slots = RawNodeSlots.empty
            let (ParsedChildren elements) = children
        
            let slots, elements =
                match elements with
                | element :: tail when LetDecl.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when Expr.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
        
            match elements with
            | [] -> RawNodeSlots.intoNode (SyntaxKind.toRaw kind) children slots
            | _ -> GreenNode.mk (SyntaxKind.toErr kind |> SyntaxKind.toRaw) (ParsedChildren.elements children |> Seq.map Some)
        
        | SyntaxKind.ListExpr ->
            let slots = RawNodeSlots.empty
            let (ParsedChildren elements) = children
        
            let slots, elements =
                match elements with
                | element :: tail when SyntaxKind.fromRaw (GreenElement.kind element) = SyntaxKind.LBracket -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when ListExprElements.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when SyntaxKind.fromRaw (GreenElement.kind element) = SyntaxKind.RBracket -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
        
            match elements with
            | [] -> RawNodeSlots.intoNode (SyntaxKind.toRaw kind) children slots
            | _ -> GreenNode.mk (SyntaxKind.toErr kind |> SyntaxKind.toRaw) (ParsedChildren.elements children |> Seq.map Some)
        
        | SyntaxKind.ListPat ->
            let slots = RawNodeSlots.empty
            let (ParsedChildren elements) = children
        
            let slots, elements =
                match elements with
                | element :: tail when SyntaxKind.fromRaw (GreenElement.kind element) = SyntaxKind.LBracket -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when ListPatElements.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when SyntaxKind.fromRaw (GreenElement.kind element) = SyntaxKind.RBracket -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
        
            match elements with
            | [] -> RawNodeSlots.intoNode (SyntaxKind.toRaw kind) children slots
            | _ -> GreenNode.mk (SyntaxKind.toErr kind |> SyntaxKind.toRaw) (ParsedChildren.elements children |> Seq.map Some)
        
        | SyntaxKind.MatchCase ->
            let slots = RawNodeSlots.empty
            let (ParsedChildren elements) = children
        
            let slots, elements =
                match elements with
                | element :: tail when Pat.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when MatchGuard.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when SyntaxKind.fromRaw (GreenElement.kind element) = SyntaxKind.Arrow -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when Expr.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
        
            match elements with
            | [] -> RawNodeSlots.intoNode (SyntaxKind.toRaw kind) children slots
            | _ -> GreenNode.mk (SyntaxKind.toErr kind |> SyntaxKind.toRaw) (ParsedChildren.elements children |> Seq.map Some)
        
        | SyntaxKind.MatchExpr ->
            let slots = RawNodeSlots.empty
            let (ParsedChildren elements) = children
        
            let slots, elements =
                match elements with
                | element :: tail when SyntaxKind.fromRaw (GreenElement.kind element) = SyntaxKind.MatchKw -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when Expr.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when SyntaxKind.fromRaw (GreenElement.kind element) = SyntaxKind.WithKw -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when SyntaxKind.fromRaw (GreenElement.kind element) = SyntaxKind.Pipe -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when MatchCaseList.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
        
            match elements with
            | [] -> RawNodeSlots.intoNode (SyntaxKind.toRaw kind) children slots
            | _ -> GreenNode.mk (SyntaxKind.toErr kind |> SyntaxKind.toRaw) (ParsedChildren.elements children |> Seq.map Some)
        
        | SyntaxKind.MatchGuard ->
            let slots = RawNodeSlots.empty
            let (ParsedChildren elements) = children
        
            let slots, elements =
                match elements with
                | element :: tail when SyntaxKind.fromRaw (GreenElement.kind element) = SyntaxKind.WhenKw -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when Expr.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
        
            match elements with
            | [] -> RawNodeSlots.intoNode (SyntaxKind.toRaw kind) children slots
            | _ -> GreenNode.mk (SyntaxKind.toErr kind |> SyntaxKind.toRaw) (ParsedChildren.elements children |> Seq.map Some)
        
        | SyntaxKind.MemberAccessExpr ->
            let slots = RawNodeSlots.empty
            let (ParsedChildren elements) = children
        
            let slots, elements =
                match elements with
                | element :: tail when Expr.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when SyntaxKind.fromRaw (GreenElement.kind element) = SyntaxKind.Dot -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when Name.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
        
            match elements with
            | [] -> RawNodeSlots.intoNode (SyntaxKind.toRaw kind) children slots
            | _ -> GreenNode.mk (SyntaxKind.toErr kind |> SyntaxKind.toRaw) (ParsedChildren.elements children |> Seq.map Some)
        
        | SyntaxKind.ModulePreamble ->
            let slots = RawNodeSlots.empty
            let (ParsedChildren elements) = children
        
            let slots, elements =
                match elements with
                | element :: tail when SyntaxKind.fromRaw (GreenElement.kind element) = SyntaxKind.ModuleKw -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when QName.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
        
            match elements with
            | [] -> RawNodeSlots.intoNode (SyntaxKind.toRaw kind) children slots
            | _ -> GreenNode.mk (SyntaxKind.toErr kind |> SyntaxKind.toRaw) (ParsedChildren.elements children |> Seq.map Some)
        
        | SyntaxKind.ModuleRoot ->
            let slots = RawNodeSlots.empty
            let (ParsedChildren elements) = children
        
            let slots, elements =
                match elements with
                | element :: tail when ModulePreamble.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when ModuleDeclList.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when SyntaxKind.fromRaw (GreenElement.kind element) = SyntaxKind.Eof -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
        
            match elements with
            | [] -> RawNodeSlots.intoNode (SyntaxKind.toRaw kind) children slots
            | _ -> GreenNode.mk (SyntaxKind.toErr kind |> SyntaxKind.toRaw) (ParsedChildren.elements children |> Seq.map Some)
        
        | SyntaxKind.Name ->
            let slots = RawNodeSlots.empty
            let (ParsedChildren elements) = children
        
            let slots, elements =
                match elements with
                | element :: tail when SyntaxKind.fromRaw (GreenElement.kind element) = SyntaxKind.Ident -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
        
            match elements with
            | [] -> RawNodeSlots.intoNode (SyntaxKind.toRaw kind) children slots
            | _ -> GreenNode.mk (SyntaxKind.toErr kind |> SyntaxKind.toRaw) (ParsedChildren.elements children |> Seq.map Some)
        
        | SyntaxKind.NamePatField ->
            let slots = RawNodeSlots.empty
            let (ParsedChildren elements) = children
        
            let slots, elements =
                match elements with
                | element :: tail when QName.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when SyntaxKind.fromRaw (GreenElement.kind element) = SyntaxKind.Eq -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when Pat.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
        
            match elements with
            | [] -> RawNodeSlots.intoNode (SyntaxKind.toRaw kind) children slots
            | _ -> GreenNode.mk (SyntaxKind.toErr kind |> SyntaxKind.toRaw) (ParsedChildren.elements children |> Seq.map Some)
        
        | SyntaxKind.NamedPat ->
            let slots = RawNodeSlots.empty
            let (ParsedChildren elements) = children
        
            let slots, elements =
                match elements with
                | element :: tail when QName.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
        
            match elements with
            | [] -> RawNodeSlots.intoNode (SyntaxKind.toRaw kind) children slots
            | _ -> GreenNode.mk (SyntaxKind.toErr kind |> SyntaxKind.toRaw) (ParsedChildren.elements children |> Seq.map Some)
        
        | SyntaxKind.OpenDecl ->
            let slots = RawNodeSlots.empty
            let (ParsedChildren elements) = children
        
            let slots, elements =
                match elements with
                | element :: tail when SyntaxKind.fromRaw (GreenElement.kind element) = SyntaxKind.OpenKw -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when QName.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
        
            match elements with
            | [] -> RawNodeSlots.intoNode (SyntaxKind.toRaw kind) children slots
            | _ -> GreenNode.mk (SyntaxKind.toErr kind |> SyntaxKind.toRaw) (ParsedChildren.elements children |> Seq.map Some)
        
        | SyntaxKind.Operator ->
            let slots = RawNodeSlots.empty
            let (ParsedChildren elements) = children
        
            let slots, elements =
                match elements with
                | element :: tail when SyntaxKind.fromRaw (GreenElement.kind element) = SyntaxKind.Operator -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
        
            match elements with
            | [] -> RawNodeSlots.intoNode (SyntaxKind.toRaw kind) children slots
            | _ -> GreenNode.mk (SyntaxKind.toErr kind |> SyntaxKind.toRaw) (ParsedChildren.elements children |> Seq.map Some)
        
        | SyntaxKind.OrPat ->
            let slots = RawNodeSlots.empty
            let (ParsedChildren elements) = children
        
            let slots, elements =
                match elements with
                | element :: tail when Pat.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when SyntaxKind.fromRaw (GreenElement.kind element) = SyntaxKind.Pipe -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when Pat.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
        
            match elements with
            | [] -> RawNodeSlots.intoNode (SyntaxKind.toRaw kind) children slots
            | _ -> GreenNode.mk (SyntaxKind.toErr kind |> SyntaxKind.toRaw) (ParsedChildren.elements children |> Seq.map Some)
        
        | SyntaxKind.ParenExpr ->
            let slots = RawNodeSlots.empty
            let (ParsedChildren elements) = children
        
            let slots, elements =
                match elements with
                | element :: tail when SyntaxKind.fromRaw (GreenElement.kind element) = SyntaxKind.LParen -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when Expr.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when SyntaxKind.fromRaw (GreenElement.kind element) = SyntaxKind.RParen -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
        
            match elements with
            | [] -> RawNodeSlots.intoNode (SyntaxKind.toRaw kind) children slots
            | _ -> GreenNode.mk (SyntaxKind.toErr kind |> SyntaxKind.toRaw) (ParsedChildren.elements children |> Seq.map Some)
        
        | SyntaxKind.ParenPat ->
            let slots = RawNodeSlots.empty
            let (ParsedChildren elements) = children
        
            let slots, elements =
                match elements with
                | element :: tail when SyntaxKind.fromRaw (GreenElement.kind element) = SyntaxKind.LParen -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when Pat.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when SyntaxKind.fromRaw (GreenElement.kind element) = SyntaxKind.RParen -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
        
            match elements with
            | [] -> RawNodeSlots.intoNode (SyntaxKind.toRaw kind) children slots
            | _ -> GreenNode.mk (SyntaxKind.toErr kind |> SyntaxKind.toRaw) (ParsedChildren.elements children |> Seq.map Some)
        
        | SyntaxKind.ParenTy ->
            let slots = RawNodeSlots.empty
            let (ParsedChildren elements) = children
        
            let slots, elements =
                match elements with
                | element :: tail when SyntaxKind.fromRaw (GreenElement.kind element) = SyntaxKind.LParen -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when Ty.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when SyntaxKind.fromRaw (GreenElement.kind element) = SyntaxKind.RParen -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
        
            match elements with
            | [] -> RawNodeSlots.intoNode (SyntaxKind.toRaw kind) children slots
            | _ -> GreenNode.mk (SyntaxKind.toErr kind |> SyntaxKind.toRaw) (ParsedChildren.elements children |> Seq.map Some)
        
        | SyntaxKind.QName ->
            let slots = RawNodeSlots.empty
            let (ParsedChildren elements) = children
        
            let slots, elements =
                match elements with
                | element :: tail when QName.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when SyntaxKind.fromRaw (GreenElement.kind element) = SyntaxKind.Dot -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when QNameSegment.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
        
            match elements with
            | [] -> RawNodeSlots.intoNode (SyntaxKind.toRaw kind) children slots
            | _ -> GreenNode.mk (SyntaxKind.toErr kind |> SyntaxKind.toRaw) (ParsedChildren.elements children |> Seq.map Some)
        
        | SyntaxKind.QNameSegment ->
            let slots = RawNodeSlots.empty
            let (ParsedChildren elements) = children
        
            let slots, elements =
                match elements with
                | element :: tail when Name.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
        
            match elements with
            | [] -> RawNodeSlots.intoNode (SyntaxKind.toRaw kind) children slots
            | _ -> GreenNode.mk (SyntaxKind.toErr kind |> SyntaxKind.toRaw) (ParsedChildren.elements children |> Seq.map Some)
        
        | SyntaxKind.QTy ->
            let slots = RawNodeSlots.empty
            let (ParsedChildren elements) = children
        
            let slots, elements =
                match elements with
                | element :: tail when QName.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
        
            match elements with
            | [] -> RawNodeSlots.intoNode (SyntaxKind.toRaw kind) children slots
            | _ -> GreenNode.mk (SyntaxKind.toErr kind |> SyntaxKind.toRaw) (ParsedChildren.elements children |> Seq.map Some)
        
        | SyntaxKind.RecordPat ->
            let slots = RawNodeSlots.empty
            let (ParsedChildren elements) = children
        
            let slots, elements =
                match elements with
                | element :: tail when SyntaxKind.fromRaw (GreenElement.kind element) = SyntaxKind.LBrace -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when RecordFields.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when SyntaxKind.fromRaw (GreenElement.kind element) = SyntaxKind.RBrace -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
        
            match elements with
            | [] -> RawNodeSlots.intoNode (SyntaxKind.toRaw kind) children slots
            | _ -> GreenNode.mk (SyntaxKind.toErr kind |> SyntaxKind.toRaw) (ParsedChildren.elements children |> Seq.map Some)
        
        | SyntaxKind.SeqExpr ->
            let slots = RawNodeSlots.empty
            let (ParsedChildren elements) = children
        
            let slots, elements =
                match elements with
                | element :: tail when Expr.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when SyntaxKind.fromRaw (GreenElement.kind element) = SyntaxKind.Semicolon -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when Expr.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
        
            match elements with
            | [] -> RawNodeSlots.intoNode (SyntaxKind.toRaw kind) children slots
            | _ -> GreenNode.mk (SyntaxKind.toErr kind |> SyntaxKind.toRaw) (ParsedChildren.elements children |> Seq.map Some)
        
        | SyntaxKind.StringLiteralExpr ->
            let slots = RawNodeSlots.empty
            let (ParsedChildren elements) = children
        
            let slots, elements =
                match elements with
                | element :: tail when SyntaxKind.fromRaw (GreenElement.kind element) = SyntaxKind.StringLiteral -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
        
            match elements with
            | [] -> RawNodeSlots.intoNode (SyntaxKind.toRaw kind) children slots
            | _ -> GreenNode.mk (SyntaxKind.toErr kind |> SyntaxKind.toRaw) (ParsedChildren.elements children |> Seq.map Some)
        
        | SyntaxKind.TypedExpr ->
            let slots = RawNodeSlots.empty
            let (ParsedChildren elements) = children
        
            let slots, elements =
                match elements with
                | element :: tail when Expr.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when SyntaxKind.fromRaw (GreenElement.kind element) = SyntaxKind.Colon -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when Ty.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
        
            match elements with
            | [] -> RawNodeSlots.intoNode (SyntaxKind.toRaw kind) children slots
            | _ -> GreenNode.mk (SyntaxKind.toErr kind |> SyntaxKind.toRaw) (ParsedChildren.elements children |> Seq.map Some)
        
        | SyntaxKind.TypedPat ->
            let slots = RawNodeSlots.empty
            let (ParsedChildren elements) = children
        
            let slots, elements =
                match elements with
                | element :: tail when Pat.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when SyntaxKind.fromRaw (GreenElement.kind element) = SyntaxKind.Colon -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when Ty.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
        
            match elements with
            | [] -> RawNodeSlots.intoNode (SyntaxKind.toRaw kind) children slots
            | _ -> GreenNode.mk (SyntaxKind.toErr kind |> SyntaxKind.toRaw) (ParsedChildren.elements children |> Seq.map Some)
        
        | SyntaxKind.UnaryExpr ->
            let slots = RawNodeSlots.empty
            let (ParsedChildren elements) = children
        
            let slots, elements =
                match elements with
                | element :: tail when QName.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when Expr.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
        
            match elements with
            | [] -> RawNodeSlots.intoNode (SyntaxKind.toRaw kind) children slots
            | _ -> GreenNode.mk (SyntaxKind.toErr kind |> SyntaxKind.toRaw) (ParsedChildren.elements children |> Seq.map Some)
        
        | SyntaxKind.UnitLiteralExpr ->
            let slots = RawNodeSlots.empty
            let (ParsedChildren elements) = children
        
            let slots, elements =
                match elements with
                | element :: tail when SyntaxKind.fromRaw (GreenElement.kind element) = SyntaxKind.LParen -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
            
            let slots, elements =
                match elements with
                | element :: tail when SyntaxKind.fromRaw (GreenElement.kind element) = SyntaxKind.RParen -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
        
            match elements with
            | [] -> RawNodeSlots.intoNode (SyntaxKind.toRaw kind) children slots
            | _ -> GreenNode.mk (SyntaxKind.toErr kind |> SyntaxKind.toRaw) (ParsedChildren.elements children |> Seq.map Some)
        
        | SyntaxKind.VarExpr ->
            let slots = RawNodeSlots.empty
            let (ParsedChildren elements) = children
        
            let slots, elements =
                match elements with
                | element :: tail when Name.CanCast (GreenElement.kind element) -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
        
            match elements with
            | [] -> RawNodeSlots.intoNode (SyntaxKind.toRaw kind) children slots
            | _ -> GreenNode.mk (SyntaxKind.toErr kind |> SyntaxKind.toRaw) (ParsedChildren.elements children |> Seq.map Some)
        
        | SyntaxKind.WildPat ->
            let slots = RawNodeSlots.empty
            let (ParsedChildren elements) = children
        
            let slots, elements =
                match elements with
                | element :: tail when SyntaxKind.fromRaw (GreenElement.kind element) = SyntaxKind.Underscore -> RawNodeSlots.addPresent slots, tail
                | _ -> RawNodeSlots.addAbsent slots, elements
        
            match elements with
            | [] -> RawNodeSlots.intoNode (SyntaxKind.toRaw kind) children slots
            | _ -> GreenNode.mk (SyntaxKind.toErr kind |> SyntaxKind.toRaw) (ParsedChildren.elements children |> Seq.map Some)

        | _ -> failwith $"Is {kind} a token?"
    