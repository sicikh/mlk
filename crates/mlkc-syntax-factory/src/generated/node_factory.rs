//! Generated file, do not edit by hand, see `xtask/codegen`

#![allow(clippy::redundant_closure)]
use biome_rowan::AstNode;
use mlkc_syntax::{
    MlkSyntaxElement as SyntaxElement, MlkSyntaxNode as SyntaxNode, MlkSyntaxToken as SyntaxToken,
    *,
};
pub fn and_pat(Lhs: Pat, amp_token: SyntaxToken, Rhs: Pat) -> AndPat {
    AndPat::unwrap_cast(SyntaxNode::new_detached(MlkSyntaxKind::AND_PAT, [
        Some(SyntaxElement::Node(Lhs.into_syntax())),
        Some(SyntaxElement::Token(amp_token)),
        Some(SyntaxElement::Node(Rhs.into_syntax())),
    ]))
}
pub fn app_expr(Func: Expr, Arg: Expr) -> AppExpr {
    AppExpr::unwrap_cast(SyntaxNode::new_detached(MlkSyntaxKind::APP_EXPR, [
        Some(SyntaxElement::Node(Func.into_syntax())),
        Some(SyntaxElement::Node(Arg.into_syntax())),
    ]))
}
pub fn as_pat(Lhs: Pat, as_token: SyntaxToken, Rhs: Pat) -> AsPat {
    AsPat::unwrap_cast(SyntaxNode::new_detached(MlkSyntaxKind::AS_PAT, [
        Some(SyntaxElement::Node(Lhs.into_syntax())),
        Some(SyntaxElement::Token(as_token)),
        Some(SyntaxElement::Node(Rhs.into_syntax())),
    ]))
}
pub fn bin_expr(Left: Expr, Op_token: SyntaxToken, Right: Expr) -> BinExpr {
    BinExpr::unwrap_cast(SyntaxNode::new_detached(MlkSyntaxKind::BIN_EXPR, [
        Some(SyntaxElement::Node(Left.into_syntax())),
        Some(SyntaxElement::Token(Op_token)),
        Some(SyntaxElement::Node(Right.into_syntax())),
    ]))
}
pub fn binding(Pat: Pat, eq_token: SyntaxToken, Expr: Expr) -> Binding {
    Binding::unwrap_cast(SyntaxNode::new_detached(MlkSyntaxKind::BINDING, [
        Some(SyntaxElement::Node(Pat.into_syntax())),
        Some(SyntaxElement::Token(eq_token)),
        Some(SyntaxElement::Node(Expr.into_syntax())),
    ]))
}
pub fn bool_literal(ValueToken_token: SyntaxToken) -> BoolLiteral {
    BoolLiteral::unwrap_cast(SyntaxNode::new_detached(MlkSyntaxKind::BOOL_LITERAL, [
        Some(SyntaxElement::Token(ValueToken_token)),
    ]))
}
pub fn char_literal(Value_token: SyntaxToken) -> CharLiteral {
    CharLiteral::unwrap_cast(SyntaxNode::new_detached(MlkSyntaxKind::CHAR_LITERAL, [
        Some(SyntaxElement::Token(Value_token)),
    ]))
}
pub fn cons_pat(Head: Pat, double_colon_token: SyntaxToken, Tail: Pat) -> ConsPat {
    ConsPat::unwrap_cast(SyntaxNode::new_detached(MlkSyntaxKind::CONS_PAT, [
        Some(SyntaxElement::Node(Head.into_syntax())),
        Some(SyntaxElement::Token(double_colon_token)),
        Some(SyntaxElement::Node(Tail.into_syntax())),
    ]))
}
pub fn fn_ty(Arg: Ty, arrow_token: SyntaxToken, Ret: Ty) -> FnTy {
    FnTy::unwrap_cast(SyntaxNode::new_detached(MlkSyntaxKind::FN_TY, [
        Some(SyntaxElement::Node(Arg.into_syntax())),
        Some(SyntaxElement::Token(arrow_token)),
        Some(SyntaxElement::Node(Ret.into_syntax())),
    ]))
}
pub fn fun_expr(
    fun_token: SyntaxToken,
    Arg: Name,
    arrow_token: SyntaxToken,
    Body: Expr,
) -> FunExpr {
    FunExpr::unwrap_cast(SyntaxNode::new_detached(MlkSyntaxKind::FUN_EXPR, [
        Some(SyntaxElement::Token(fun_token)),
        Some(SyntaxElement::Node(Arg.into_syntax())),
        Some(SyntaxElement::Token(arrow_token)),
        Some(SyntaxElement::Node(Body.into_syntax())),
    ]))
}
pub fn func_pat(Func: Pat, Args: ArgPats) -> FuncPat {
    FuncPat::unwrap_cast(SyntaxNode::new_detached(MlkSyntaxKind::FUNC_PAT, [
        Some(SyntaxElement::Node(Func.into_syntax())),
        Some(SyntaxElement::Node(Args.into_syntax())),
    ]))
}
pub fn if_expr(
    if_token: SyntaxToken,
    Cond: Expr,
    then_token: SyntaxToken,
    ThenBranch: Expr,
) -> IfExprBuilder {
    IfExprBuilder {
        if_token,
        Cond,
        then_token,
        ThenBranch,
        else_token: None,
        ElseBranch: None,
    }
}
pub struct IfExprBuilder {
    if_token: SyntaxToken,
    Cond: Expr,
    then_token: SyntaxToken,
    ThenBranch: Expr,
    else_token: Option<SyntaxToken>,
    ElseBranch: Option<Expr>,
}
impl IfExprBuilder {
    pub fn with_else_token(mut self, else_token: SyntaxToken) -> Self {
        self.else_token = Some(else_token);
        self
    }
    pub fn with_ElseBranch(mut self, ElseBranch: Expr) -> Self {
        self.ElseBranch = Some(ElseBranch);
        self
    }
    pub fn build(self) -> IfExpr {
        IfExpr::unwrap_cast(SyntaxNode::new_detached(MlkSyntaxKind::IF_EXPR, [
            Some(SyntaxElement::Token(self.if_token)),
            Some(SyntaxElement::Node(self.Cond.into_syntax())),
            Some(SyntaxElement::Token(self.then_token)),
            Some(SyntaxElement::Node(self.ThenBranch.into_syntax())),
            self.else_token.map(|token| SyntaxElement::Token(token)),
            self.ElseBranch
                .map(|token| SyntaxElement::Node(token.into_syntax())),
        ]))
    }
}
pub fn infer_ty(__token: SyntaxToken) -> InferTy {
    InferTy::unwrap_cast(SyntaxNode::new_detached(MlkSyntaxKind::INFER_TY, [Some(
        SyntaxElement::Token(__token),
    )]))
}
pub fn inner_module_item(eq_token: SyntaxToken, Decls: ModuleItemList) -> InnerModuleItemBuilder {
    InnerModuleItemBuilder {
        eq_token,
        Decls,
        Preamble: None,
    }
}
pub struct InnerModuleItemBuilder {
    eq_token: SyntaxToken,
    Decls: ModuleItemList,
    Preamble: Option<ModulePreamble>,
}
impl InnerModuleItemBuilder {
    pub fn with_Preamble(mut self, Preamble: ModulePreamble) -> Self {
        self.Preamble = Some(Preamble);
        self
    }
    pub fn build(self) -> InnerModuleItem {
        InnerModuleItem::unwrap_cast(SyntaxNode::new_detached(
            MlkSyntaxKind::INNER_MODULE_ITEM,
            [
                self.Preamble
                    .map(|token| SyntaxElement::Node(token.into_syntax())),
                Some(SyntaxElement::Token(self.eq_token)),
                Some(SyntaxElement::Node(self.Decls.into_syntax())),
            ],
        ))
    }
}
pub fn int_literal(Value_token: SyntaxToken) -> IntLiteral {
    IntLiteral::unwrap_cast(SyntaxNode::new_detached(MlkSyntaxKind::INT_LITERAL, [
        Some(SyntaxElement::Token(Value_token)),
    ]))
}
pub fn let_decl(let_token: SyntaxToken, Name: Name, eq_token: SyntaxToken, Expr: Expr) -> LetDecl {
    LetDecl::unwrap_cast(SyntaxNode::new_detached(MlkSyntaxKind::LET_DECL, [
        Some(SyntaxElement::Token(let_token)),
        Some(SyntaxElement::Node(Name.into_syntax())),
        Some(SyntaxElement::Token(eq_token)),
        Some(SyntaxElement::Node(Expr.into_syntax())),
    ]))
}
pub fn let_expr(Decl: LetDecl, Body: Expr) -> LetExpr {
    LetExpr::unwrap_cast(SyntaxNode::new_detached(MlkSyntaxKind::LET_EXPR, [
        Some(SyntaxElement::Node(Decl.into_syntax())),
        Some(SyntaxElement::Node(Body.into_syntax())),
    ]))
}
pub fn list_expr(
    l_brack_token: SyntaxToken,
    Elements: ListExprElements,
    r_brack_token: SyntaxToken,
) -> ListExpr {
    ListExpr::unwrap_cast(SyntaxNode::new_detached(MlkSyntaxKind::LIST_EXPR, [
        Some(SyntaxElement::Token(l_brack_token)),
        Some(SyntaxElement::Node(Elements.into_syntax())),
        Some(SyntaxElement::Token(r_brack_token)),
    ]))
}
pub fn list_pat(
    l_brack_token: SyntaxToken,
    Elements: ListPatElements,
    r_brack_token: SyntaxToken,
) -> ListPat {
    ListPat::unwrap_cast(SyntaxNode::new_detached(MlkSyntaxKind::LIST_PAT, [
        Some(SyntaxElement::Token(l_brack_token)),
        Some(SyntaxElement::Node(Elements.into_syntax())),
        Some(SyntaxElement::Token(r_brack_token)),
    ]))
}
pub fn literal(Value_token: SyntaxToken) -> Literal {
    Literal::unwrap_cast(SyntaxNode::new_detached(MlkSyntaxKind::LITERAL, [Some(
        SyntaxElement::Token(Value_token),
    )]))
}
pub fn literal_pat(literal: Literal) -> LiteralPat {
    LiteralPat::unwrap_cast(SyntaxNode::new_detached(MlkSyntaxKind::LITERAL_PAT, [
        Some(SyntaxElement::Node(literal.into_syntax())),
    ]))
}
pub fn match_case(pat: Pat, arrow_token: SyntaxToken, Body: Expr) -> MatchCaseBuilder {
    MatchCaseBuilder {
        pat,
        arrow_token,
        Body,
        Guard: None,
    }
}
pub struct MatchCaseBuilder {
    pat: Pat,
    arrow_token: SyntaxToken,
    Body: Expr,
    Guard: Option<MatchGuard>,
}
impl MatchCaseBuilder {
    pub fn with_Guard(mut self, Guard: MatchGuard) -> Self {
        self.Guard = Some(Guard);
        self
    }
    pub fn build(self) -> MatchCase {
        MatchCase::unwrap_cast(SyntaxNode::new_detached(MlkSyntaxKind::MATCH_CASE, [
            Some(SyntaxElement::Node(self.pat.into_syntax())),
            self.Guard
                .map(|token| SyntaxElement::Node(token.into_syntax())),
            Some(SyntaxElement::Token(self.arrow_token)),
            Some(SyntaxElement::Node(self.Body.into_syntax())),
        ]))
    }
}
pub fn match_expr(
    match_token: SyntaxToken,
    Scrutinee: Expr,
    with_token: SyntaxToken,
    Cases: MatchCaseList,
) -> MatchExprBuilder {
    MatchExprBuilder {
        match_token,
        Scrutinee,
        with_token,
        Cases,
        LeadingPipeToken_token: None,
    }
}
pub struct MatchExprBuilder {
    match_token: SyntaxToken,
    Scrutinee: Expr,
    with_token: SyntaxToken,
    Cases: MatchCaseList,
    LeadingPipeToken_token: Option<SyntaxToken>,
}
impl MatchExprBuilder {
    pub fn with_LeadingPipeToken_token(mut self, LeadingPipeToken_token: SyntaxToken) -> Self {
        self.LeadingPipeToken_token = Some(LeadingPipeToken_token);
        self
    }
    pub fn build(self) -> MatchExpr {
        MatchExpr::unwrap_cast(SyntaxNode::new_detached(MlkSyntaxKind::MATCH_EXPR, [
            Some(SyntaxElement::Token(self.match_token)),
            Some(SyntaxElement::Node(self.Scrutinee.into_syntax())),
            Some(SyntaxElement::Token(self.with_token)),
            self.LeadingPipeToken_token
                .map(|token| SyntaxElement::Token(token)),
            Some(SyntaxElement::Node(self.Cases.into_syntax())),
        ]))
    }
}
pub fn match_guard(when_token: SyntaxToken, Cond: Expr) -> MatchGuard {
    MatchGuard::unwrap_cast(SyntaxNode::new_detached(MlkSyntaxKind::MATCH_GUARD, [
        Some(SyntaxElement::Token(when_token)),
        Some(SyntaxElement::Node(Cond.into_syntax())),
    ]))
}
pub fn member_access_expr(Target: Expr, dot_token: SyntaxToken, Member: Name) -> MemberAccessExpr {
    MemberAccessExpr::unwrap_cast(SyntaxNode::new_detached(
        MlkSyntaxKind::MEMBER_ACCESS_EXPR,
        [
            Some(SyntaxElement::Node(Target.into_syntax())),
            Some(SyntaxElement::Token(dot_token)),
            Some(SyntaxElement::Node(Member.into_syntax())),
        ],
    ))
}
pub fn module_preamble(module_token: SyntaxToken, Name: QName) -> ModulePreamble {
    ModulePreamble::unwrap_cast(SyntaxNode::new_detached(MlkSyntaxKind::MODULE_PREAMBLE, [
        Some(SyntaxElement::Token(module_token)),
        Some(SyntaxElement::Node(Name.into_syntax())),
    ]))
}
pub fn module_root(items: ModuleItemList, eof_token: SyntaxToken) -> ModuleRootBuilder {
    ModuleRootBuilder {
        items,
        eof_token,
        preamble: None,
    }
}
pub struct ModuleRootBuilder {
    items: ModuleItemList,
    eof_token: SyntaxToken,
    preamble: Option<ModulePreamble>,
}
impl ModuleRootBuilder {
    pub fn with_preamble(mut self, preamble: ModulePreamble) -> Self {
        self.preamble = Some(preamble);
        self
    }
    pub fn build(self) -> ModuleRoot {
        ModuleRoot::unwrap_cast(SyntaxNode::new_detached(MlkSyntaxKind::MODULE_ROOT, [
            self.preamble
                .map(|token| SyntaxElement::Node(token.into_syntax())),
            Some(SyntaxElement::Node(self.items.into_syntax())),
            Some(SyntaxElement::Token(self.eof_token)),
        ]))
    }
}
pub fn name(Value_token: SyntaxToken) -> Name {
    Name::unwrap_cast(SyntaxNode::new_detached(MlkSyntaxKind::NAME, [Some(
        SyntaxElement::Token(Value_token),
    )]))
}
pub fn name_pat_field(Name: QName, eq_token: SyntaxToken, pat: Pat) -> NamePatField {
    NamePatField::unwrap_cast(SyntaxNode::new_detached(MlkSyntaxKind::NAME_PAT_FIELD, [
        Some(SyntaxElement::Node(Name.into_syntax())),
        Some(SyntaxElement::Token(eq_token)),
        Some(SyntaxElement::Node(pat.into_syntax())),
    ]))
}
pub fn named_pat(Name: QName) -> NamedPat {
    NamedPat::unwrap_cast(SyntaxNode::new_detached(MlkSyntaxKind::NAMED_PAT, [Some(
        SyntaxElement::Node(Name.into_syntax()),
    )]))
}
pub fn open_decl(open_token: SyntaxToken, Module: QName) -> OpenDecl {
    OpenDecl::unwrap_cast(SyntaxNode::new_detached(MlkSyntaxKind::OPEN_DECL, [
        Some(SyntaxElement::Token(open_token)),
        Some(SyntaxElement::Node(Module.into_syntax())),
    ]))
}
pub fn operator(Value_token: SyntaxToken) -> Operator {
    Operator::unwrap_cast(SyntaxNode::new_detached(MlkSyntaxKind::OPERATOR, [Some(
        SyntaxElement::Token(Value_token),
    )]))
}
pub fn or_pat(Lhs: Pat, bitwise_or_token: SyntaxToken, Rhs: Pat) -> OrPat {
    OrPat::unwrap_cast(SyntaxNode::new_detached(MlkSyntaxKind::OR_PAT, [
        Some(SyntaxElement::Node(Lhs.into_syntax())),
        Some(SyntaxElement::Token(bitwise_or_token)),
        Some(SyntaxElement::Node(Rhs.into_syntax())),
    ]))
}
pub fn paren_expr(l_paren_token: SyntaxToken, expr: Expr, r_paren_token: SyntaxToken) -> ParenExpr {
    ParenExpr::unwrap_cast(SyntaxNode::new_detached(MlkSyntaxKind::PAREN_EXPR, [
        Some(SyntaxElement::Token(l_paren_token)),
        Some(SyntaxElement::Node(expr.into_syntax())),
        Some(SyntaxElement::Token(r_paren_token)),
    ]))
}
pub fn paren_pat(l_paren_token: SyntaxToken, pat: Pat, r_paren_token: SyntaxToken) -> ParenPat {
    ParenPat::unwrap_cast(SyntaxNode::new_detached(MlkSyntaxKind::PAREN_PAT, [
        Some(SyntaxElement::Token(l_paren_token)),
        Some(SyntaxElement::Node(pat.into_syntax())),
        Some(SyntaxElement::Token(r_paren_token)),
    ]))
}
pub fn paren_ty(l_paren_token: SyntaxToken, ty: Ty, r_paren_token: SyntaxToken) -> ParenTy {
    ParenTy::unwrap_cast(SyntaxNode::new_detached(MlkSyntaxKind::PAREN_TY, [
        Some(SyntaxElement::Token(l_paren_token)),
        Some(SyntaxElement::Node(ty.into_syntax())),
        Some(SyntaxElement::Token(r_paren_token)),
    ]))
}
pub fn q_name(Segment: QNameSegment) -> QNameBuilder {
    QNameBuilder {
        Segment,
        Qualifier: None,
        dot_token: None,
    }
}
pub struct QNameBuilder {
    Segment: QNameSegment,
    Qualifier: Option<QName>,
    dot_token: Option<SyntaxToken>,
}
impl QNameBuilder {
    pub fn with_Qualifier(mut self, Qualifier: QName) -> Self {
        self.Qualifier = Some(Qualifier);
        self
    }
    pub fn with_dot_token(mut self, dot_token: SyntaxToken) -> Self {
        self.dot_token = Some(dot_token);
        self
    }
    pub fn build(self) -> QName {
        QName::unwrap_cast(SyntaxNode::new_detached(MlkSyntaxKind::Q_NAME, [
            self.Qualifier
                .map(|token| SyntaxElement::Node(token.into_syntax())),
            self.dot_token.map(|token| SyntaxElement::Token(token)),
            Some(SyntaxElement::Node(self.Segment.into_syntax())),
        ]))
    }
}
pub fn q_name_segment(Name: Name) -> QNameSegment {
    QNameSegment::unwrap_cast(SyntaxNode::new_detached(MlkSyntaxKind::Q_NAME_SEGMENT, [
        Some(SyntaxElement::Node(Name.into_syntax())),
    ]))
}
pub fn q_ty(Name: QName) -> QTy {
    QTy::unwrap_cast(SyntaxNode::new_detached(MlkSyntaxKind::Q_TY, [Some(
        SyntaxElement::Node(Name.into_syntax()),
    )]))
}
pub fn record_pat(
    l_curly_token: SyntaxToken,
    Fields: RecordFields,
    r_curly_token: SyntaxToken,
) -> RecordPat {
    RecordPat::unwrap_cast(SyntaxNode::new_detached(MlkSyntaxKind::RECORD_PAT, [
        Some(SyntaxElement::Token(l_curly_token)),
        Some(SyntaxElement::Node(Fields.into_syntax())),
        Some(SyntaxElement::Token(r_curly_token)),
    ]))
}
pub fn seq_expr(First: Expr, Second: Expr) -> SeqExpr {
    SeqExpr::unwrap_cast(SyntaxNode::new_detached(MlkSyntaxKind::SEQ_EXPR, [
        Some(SyntaxElement::Node(First.into_syntax())),
        Some(SyntaxElement::Node(Second.into_syntax())),
    ]))
}
pub fn string_literal(Value_token: SyntaxToken) -> StringLiteral {
    StringLiteral::unwrap_cast(SyntaxNode::new_detached(MlkSyntaxKind::STRING_LITERAL, [
        Some(SyntaxElement::Token(Value_token)),
    ]))
}
pub fn typed_expr(Expr: Expr, colon_token: SyntaxToken, Target: Ty) -> TypedExpr {
    TypedExpr::unwrap_cast(SyntaxNode::new_detached(MlkSyntaxKind::TYPED_EXPR, [
        Some(SyntaxElement::Node(Expr.into_syntax())),
        Some(SyntaxElement::Token(colon_token)),
        Some(SyntaxElement::Node(Target.into_syntax())),
    ]))
}
pub fn typed_pat(pat: Pat, colon_token: SyntaxToken, Target: Ty) -> TypedPat {
    TypedPat::unwrap_cast(SyntaxNode::new_detached(MlkSyntaxKind::TYPED_PAT, [
        Some(SyntaxElement::Node(pat.into_syntax())),
        Some(SyntaxElement::Token(colon_token)),
        Some(SyntaxElement::Node(Target.into_syntax())),
    ]))
}
pub fn unary_expr(Op: QName, Operand: Expr) -> UnaryExpr {
    UnaryExpr::unwrap_cast(SyntaxNode::new_detached(MlkSyntaxKind::UNARY_EXPR, [
        Some(SyntaxElement::Node(Op.into_syntax())),
        Some(SyntaxElement::Node(Operand.into_syntax())),
    ]))
}
pub fn unit_literal(l_paren_token: SyntaxToken, r_paren_token: SyntaxToken) -> UnitLiteral {
    UnitLiteral::unwrap_cast(SyntaxNode::new_detached(MlkSyntaxKind::UNIT_LITERAL, [
        Some(SyntaxElement::Token(l_paren_token)),
        Some(SyntaxElement::Token(r_paren_token)),
    ]))
}
pub fn var_expr(Name: Name) -> VarExpr {
    VarExpr::unwrap_cast(SyntaxNode::new_detached(MlkSyntaxKind::VAR_EXPR, [Some(
        SyntaxElement::Node(Name.into_syntax()),
    )]))
}
pub fn wild_pat(__token: SyntaxToken) -> WildPat {
    WildPat::unwrap_cast(SyntaxNode::new_detached(MlkSyntaxKind::WILD_PAT, [Some(
        SyntaxElement::Token(__token),
    )]))
}
pub fn arg_pats<I>(items: I) -> ArgPats
where
    I: IntoIterator<Item = ArgPat>,
    I::IntoIter: ExactSizeIterator,
{
    ArgPats::unwrap_cast(SyntaxNode::new_detached(
        MlkSyntaxKind::ARG_PATS,
        items
            .into_iter()
            .map(|item| Some(item.into_syntax().into())),
    ))
}
pub fn list_expr_elements<I, S>(items: I, separators: S) -> ListExprElements
where
    I: IntoIterator<Item = Expr>,
    I::IntoIter: ExactSizeIterator,
    S: IntoIterator<Item = MlkSyntaxToken>,
    S::IntoIter: ExactSizeIterator,
{
    let mut items = items.into_iter();
    let mut separators = separators.into_iter();
    let length = items.len() + separators.len();
    ListExprElements::unwrap_cast(SyntaxNode::new_detached(
        MlkSyntaxKind::LIST_EXPR_ELEMENTS,
        (0..length).map(|index| {
            if index % 2 == 0 {
                Some(items.next()?.into_syntax().into())
            } else {
                Some(separators.next()?.into())
            }
        }),
    ))
}
pub fn list_pat_elements<I, S>(items: I, separators: S) -> ListPatElements
where
    I: IntoIterator<Item = Pat>,
    I::IntoIter: ExactSizeIterator,
    S: IntoIterator<Item = MlkSyntaxToken>,
    S::IntoIter: ExactSizeIterator,
{
    let mut items = items.into_iter();
    let mut separators = separators.into_iter();
    let length = items.len() + separators.len();
    ListPatElements::unwrap_cast(SyntaxNode::new_detached(
        MlkSyntaxKind::LIST_PAT_ELEMENTS,
        (0..length).map(|index| {
            if index % 2 == 0 {
                Some(items.next()?.into_syntax().into())
            } else {
                Some(separators.next()?.into())
            }
        }),
    ))
}
pub fn match_case_list<I, S>(items: I, separators: S) -> MatchCaseList
where
    I: IntoIterator<Item = MatchCase>,
    I::IntoIter: ExactSizeIterator,
    S: IntoIterator<Item = MlkSyntaxToken>,
    S::IntoIter: ExactSizeIterator,
{
    let mut items = items.into_iter();
    let mut separators = separators.into_iter();
    let length = items.len() + separators.len();
    MatchCaseList::unwrap_cast(SyntaxNode::new_detached(
        MlkSyntaxKind::MATCH_CASE_LIST,
        (0..length).map(|index| {
            if index % 2 == 0 {
                Some(items.next()?.into_syntax().into())
            } else {
                Some(separators.next()?.into())
            }
        }),
    ))
}
pub fn module_item_list<I>(items: I) -> ModuleItemList
where
    I: IntoIterator<Item = ModuleItem>,
    I::IntoIter: ExactSizeIterator,
{
    ModuleItemList::unwrap_cast(SyntaxNode::new_detached(
        MlkSyntaxKind::MODULE_ITEM_LIST,
        items
            .into_iter()
            .map(|item| Some(item.into_syntax().into())),
    ))
}
pub fn record_fields<I, S>(items: I, separators: S) -> RecordFields
where
    I: IntoIterator<Item = NamePatField>,
    I::IntoIter: ExactSizeIterator,
    S: IntoIterator<Item = MlkSyntaxToken>,
    S::IntoIter: ExactSizeIterator,
{
    let mut items = items.into_iter();
    let mut separators = separators.into_iter();
    let length = items.len() + separators.len();
    RecordFields::unwrap_cast(SyntaxNode::new_detached(
        MlkSyntaxKind::RECORD_FIELDS,
        (0..length).map(|index| {
            if index % 2 == 0 {
                Some(items.next()?.into_syntax().into())
            } else {
                Some(separators.next()?.into())
            }
        }),
    ))
}
pub fn tuple_expr<I, S>(items: I, separators: S) -> TupleExpr
where
    I: IntoIterator<Item = Expr>,
    I::IntoIter: ExactSizeIterator,
    S: IntoIterator<Item = MlkSyntaxToken>,
    S::IntoIter: ExactSizeIterator,
{
    let mut items = items.into_iter();
    let mut separators = separators.into_iter();
    let length = items.len() + separators.len();
    TupleExpr::unwrap_cast(SyntaxNode::new_detached(
        MlkSyntaxKind::TUPLE_EXPR,
        (0..length).map(|index| {
            if index % 2 == 0 {
                Some(items.next()?.into_syntax().into())
            } else {
                Some(separators.next()?.into())
            }
        }),
    ))
}
pub fn tuple_pat<I, S>(items: I, separators: S) -> TuplePat
where
    I: IntoIterator<Item = Pat>,
    I::IntoIter: ExactSizeIterator,
    S: IntoIterator<Item = MlkSyntaxToken>,
    S::IntoIter: ExactSizeIterator,
{
    let mut items = items.into_iter();
    let mut separators = separators.into_iter();
    let length = items.len() + separators.len();
    TuplePat::unwrap_cast(SyntaxNode::new_detached(
        MlkSyntaxKind::TUPLE_PAT,
        (0..length).map(|index| {
            if index % 2 == 0 {
                Some(items.next()?.into_syntax().into())
            } else {
                Some(separators.next()?.into())
            }
        }),
    ))
}
pub fn tuple_ty<I, S>(items: I, separators: S) -> TupleTy
where
    I: IntoIterator<Item = Ty>,
    I::IntoIter: ExactSizeIterator,
    S: IntoIterator<Item = MlkSyntaxToken>,
    S::IntoIter: ExactSizeIterator,
{
    let mut items = items.into_iter();
    let mut separators = separators.into_iter();
    let length = items.len() + separators.len();
    TupleTy::unwrap_cast(SyntaxNode::new_detached(
        MlkSyntaxKind::TUPLE_TY,
        (0..length).map(|index| {
            if index % 2 == 0 {
                Some(items.next()?.into_syntax().into())
            } else {
                Some(separators.next()?.into())
            }
        }),
    ))
}
pub fn bogus<I>(slots: I) -> Bogus
where
    I: IntoIterator<Item = Option<SyntaxElement>>,
    I::IntoIter: ExactSizeIterator,
{
    Bogus::unwrap_cast(SyntaxNode::new_detached(MlkSyntaxKind::BOGUS, slots))
}
pub fn bogus_decl<I>(slots: I) -> BogusDecl
where
    I: IntoIterator<Item = Option<SyntaxElement>>,
    I::IntoIter: ExactSizeIterator,
{
    BogusDecl::unwrap_cast(SyntaxNode::new_detached(MlkSyntaxKind::BOGUS_DECL, slots))
}
pub fn bogus_expr<I>(slots: I) -> BogusExpr
where
    I: IntoIterator<Item = Option<SyntaxElement>>,
    I::IntoIter: ExactSizeIterator,
{
    BogusExpr::unwrap_cast(SyntaxNode::new_detached(MlkSyntaxKind::BOGUS_EXPR, slots))
}
pub fn bogus_pat<I>(slots: I) -> BogusPat
where
    I: IntoIterator<Item = Option<SyntaxElement>>,
    I::IntoIter: ExactSizeIterator,
{
    BogusPat::unwrap_cast(SyntaxNode::new_detached(MlkSyntaxKind::BOGUS_PAT, slots))
}
pub fn bogus_ty<I>(slots: I) -> BogusTy
where
    I: IntoIterator<Item = Option<SyntaxElement>>,
    I::IntoIter: ExactSizeIterator,
{
    BogusTy::unwrap_cast(SyntaxNode::new_detached(MlkSyntaxKind::BOGUS_TY, slots))
}
