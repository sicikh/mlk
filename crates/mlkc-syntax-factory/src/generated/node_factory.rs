//! Generated file, do not edit by hand, see `xtask/codegen`

#![allow(clippy::redundant_closure)]
use mlkc_rowan::AstNode;
use mlkc_syntax::{SyntaxElement, SyntaxNode, SyntaxToken, *};
pub fn attribute(at_token: SyntaxToken, name: Name) -> Attribute {
    Attribute::unwrap_cast(SyntaxNode::new_detached(SyntaxKind::ATTRIBUTE, [
        Some(SyntaxElement::Token(at_token)),
        Some(SyntaxElement::Node(name.into_syntax())),
    ]))
}
pub fn bin_expr(lhs: Expr, operator_token_token: SyntaxToken, rhs: Expr) -> BinExpr {
    BinExpr::unwrap_cast(SyntaxNode::new_detached(SyntaxKind::BIN_EXPR, [
        Some(SyntaxElement::Node(lhs.into_syntax())),
        Some(SyntaxElement::Token(operator_token_token)),
        Some(SyntaxElement::Node(rhs.into_syntax())),
    ]))
}
pub fn call_expr(
    function: Expr,
    l_paren_token: SyntaxToken,
    arguments: ArgumentList,
    r_paren_token: SyntaxToken,
) -> CallExpr {
    CallExpr::unwrap_cast(SyntaxNode::new_detached(SyntaxKind::CALL_EXPR, [
        Some(SyntaxElement::Node(function.into_syntax())),
        Some(SyntaxElement::Token(l_paren_token)),
        Some(SyntaxElement::Node(arguments.into_syntax())),
        Some(SyntaxElement::Token(r_paren_token)),
    ]))
}
pub fn fun_body(eq_token: SyntaxToken, expr: Expr) -> FunBody {
    FunBody::unwrap_cast(SyntaxNode::new_detached(SyntaxKind::FUN_BODY, [
        Some(SyntaxElement::Token(eq_token)),
        Some(SyntaxElement::Node(expr.into_syntax())),
    ]))
}
pub fn fun_decl(
    attributes: AttributeList,
    fun_token: SyntaxToken,
    name: Name,
    parameters: Parameters,
) -> FunDeclBuilder {
    FunDeclBuilder {
        attributes,
        fun_token,
        name,
        parameters,
        visibility_token: None,
        return_type_annotation: None,
        body: None,
    }
}
pub struct FunDeclBuilder {
    attributes: AttributeList,
    fun_token: SyntaxToken,
    name: Name,
    parameters: Parameters,
    visibility_token: Option<SyntaxToken>,
    return_type_annotation: Option<FunReturnTypeAnnotation>,
    body: Option<FunBody>,
}
impl FunDeclBuilder {
    pub fn with_visibility_token(mut self, visibility_token: SyntaxToken) -> Self {
        self.visibility_token = Some(visibility_token);
        self
    }
    pub fn with_return_type_annotation(
        mut self,
        return_type_annotation: FunReturnTypeAnnotation,
    ) -> Self {
        self.return_type_annotation = Some(return_type_annotation);
        self
    }
    pub fn with_body(mut self, body: FunBody) -> Self {
        self.body = Some(body);
        self
    }
    pub fn build(self) -> FunDecl {
        FunDecl::unwrap_cast(SyntaxNode::new_detached(SyntaxKind::FUN_DECL, [
            Some(SyntaxElement::Node(self.attributes.into_syntax())),
            self.visibility_token
                .map(|token| SyntaxElement::Token(token)),
            Some(SyntaxElement::Token(self.fun_token)),
            Some(SyntaxElement::Node(self.name.into_syntax())),
            Some(SyntaxElement::Node(self.parameters.into_syntax())),
            self.return_type_annotation
                .map(|token| SyntaxElement::Node(token.into_syntax())),
            self.body
                .map(|token| SyntaxElement::Node(token.into_syntax())),
        ]))
    }
}
pub fn fun_return_type_annotation(
    colon_token: SyntaxToken,
    return_type: Type,
) -> FunReturnTypeAnnotation {
    FunReturnTypeAnnotation::unwrap_cast(SyntaxNode::new_detached(
        SyntaxKind::FUN_RETURN_TYPE_ANNOTATION,
        [
            Some(SyntaxElement::Token(colon_token)),
            Some(SyntaxElement::Node(return_type.into_syntax())),
        ],
    ))
}
pub fn ident_pat(name: Name) -> IdentPat {
    IdentPat::unwrap_cast(SyntaxNode::new_detached(SyntaxKind::IDENT_PAT, [Some(
        SyntaxElement::Node(name.into_syntax()),
    )]))
}
pub fn infer_type(underscore_token: SyntaxToken) -> InferType {
    InferType::unwrap_cast(SyntaxNode::new_detached(SyntaxKind::INFER_TYPE, [Some(
        SyntaxElement::Token(underscore_token),
    )]))
}
pub fn int_literal(value_token: SyntaxToken) -> IntLiteral {
    IntLiteral::unwrap_cast(SyntaxNode::new_detached(SyntaxKind::INT_LITERAL, [Some(
        SyntaxElement::Token(value_token),
    )]))
}
pub fn let_expr(
    let_token: SyntaxToken,
    pat: Pat,
    eq_token: SyntaxToken,
    expr: Expr,
    in_token: SyntaxToken,
    body: Expr,
) -> LetExpr {
    LetExpr::unwrap_cast(SyntaxNode::new_detached(SyntaxKind::LET_EXPR, [
        Some(SyntaxElement::Token(let_token)),
        Some(SyntaxElement::Node(pat.into_syntax())),
        Some(SyntaxElement::Token(eq_token)),
        Some(SyntaxElement::Node(expr.into_syntax())),
        Some(SyntaxElement::Token(in_token)),
        Some(SyntaxElement::Node(body.into_syntax())),
    ]))
}
pub fn module_preamble(
    attributes: AttributeList,
    module_token: SyntaxToken,
    name: Path,
) -> ModulePreamble {
    ModulePreamble::unwrap_cast(SyntaxNode::new_detached(SyntaxKind::MODULE_PREAMBLE, [
        Some(SyntaxElement::Node(attributes.into_syntax())),
        Some(SyntaxElement::Token(module_token)),
        Some(SyntaxElement::Node(name.into_syntax())),
    ]))
}
pub fn module_root(items: ModuleItemList, eof_token: SyntaxToken) -> ModuleRootBuilder {
    ModuleRootBuilder {
        items,
        eof_token,
        bom_token: None,
        preamble: None,
    }
}
pub struct ModuleRootBuilder {
    items: ModuleItemList,
    eof_token: SyntaxToken,
    bom_token: Option<SyntaxToken>,
    preamble: Option<ModulePreamble>,
}
impl ModuleRootBuilder {
    pub fn with_bom_token(mut self, bom_token: SyntaxToken) -> Self {
        self.bom_token = Some(bom_token);
        self
    }
    pub fn with_preamble(mut self, preamble: ModulePreamble) -> Self {
        self.preamble = Some(preamble);
        self
    }
    pub fn build(self) -> ModuleRoot {
        ModuleRoot::unwrap_cast(SyntaxNode::new_detached(SyntaxKind::MODULE_ROOT, [
            self.bom_token.map(|token| SyntaxElement::Token(token)),
            self.preamble
                .map(|token| SyntaxElement::Node(token.into_syntax())),
            Some(SyntaxElement::Node(self.items.into_syntax())),
            Some(SyntaxElement::Token(self.eof_token)),
        ]))
    }
}
pub fn name(value_token: SyntaxToken) -> Name {
    Name::unwrap_cast(SyntaxNode::new_detached(SyntaxKind::NAME, [Some(
        SyntaxElement::Token(value_token),
    )]))
}
pub fn parameter(pat: Pat) -> ParameterBuilder {
    ParameterBuilder {
        pat,
        type_annotation: None,
    }
}
pub struct ParameterBuilder {
    pat: Pat,
    type_annotation: Option<TypeAnnotation>,
}
impl ParameterBuilder {
    pub fn with_type_annotation(mut self, type_annotation: TypeAnnotation) -> Self {
        self.type_annotation = Some(type_annotation);
        self
    }
    pub fn build(self) -> Parameter {
        Parameter::unwrap_cast(SyntaxNode::new_detached(SyntaxKind::PARAMETER, [
            Some(SyntaxElement::Node(self.pat.into_syntax())),
            self.type_annotation
                .map(|token| SyntaxElement::Node(token.into_syntax())),
        ]))
    }
}
pub fn parameters(
    l_paren_token: SyntaxToken,
    items: ParameterList,
    r_paren_token: SyntaxToken,
) -> Parameters {
    Parameters::unwrap_cast(SyntaxNode::new_detached(SyntaxKind::PARAMETERS, [
        Some(SyntaxElement::Token(l_paren_token)),
        Some(SyntaxElement::Node(items.into_syntax())),
        Some(SyntaxElement::Token(r_paren_token)),
    ]))
}
pub fn paren_expr(l_paren_token: SyntaxToken, expr: Expr, r_paren_token: SyntaxToken) -> ParenExpr {
    ParenExpr::unwrap_cast(SyntaxNode::new_detached(SyntaxKind::PAREN_EXPR, [
        Some(SyntaxElement::Token(l_paren_token)),
        Some(SyntaxElement::Node(expr.into_syntax())),
        Some(SyntaxElement::Token(r_paren_token)),
    ]))
}
pub fn path(segment: PathSegment) -> PathBuilder {
    PathBuilder {
        segment,
        qualifier: None,
    }
}
pub struct PathBuilder {
    segment: PathSegment,
    qualifier: Option<PathQualifier>,
}
impl PathBuilder {
    pub fn with_qualifier(mut self, qualifier: PathQualifier) -> Self {
        self.qualifier = Some(qualifier);
        self
    }
    pub fn build(self) -> Path {
        Path::unwrap_cast(SyntaxNode::new_detached(SyntaxKind::PATH, [
            self.qualifier
                .map(|token| SyntaxElement::Node(token.into_syntax())),
            Some(SyntaxElement::Node(self.segment.into_syntax())),
        ]))
    }
}
pub fn path_expr(path: Path) -> PathExpr {
    PathExpr::unwrap_cast(SyntaxNode::new_detached(SyntaxKind::PATH_EXPR, [Some(
        SyntaxElement::Node(path.into_syntax()),
    )]))
}
pub fn path_qualifier(path: Path, dot_token: SyntaxToken) -> PathQualifier {
    PathQualifier::unwrap_cast(SyntaxNode::new_detached(SyntaxKind::PATH_QUALIFIER, [
        Some(SyntaxElement::Node(path.into_syntax())),
        Some(SyntaxElement::Token(dot_token)),
    ]))
}
pub fn path_segment(root: PathRoot) -> PathSegmentBuilder {
    PathSegmentBuilder {
        root,
        type_args: None,
    }
}
pub struct PathSegmentBuilder {
    root: PathRoot,
    type_args: Option<TypeArgs>,
}
impl PathSegmentBuilder {
    pub fn with_type_args(mut self, type_args: TypeArgs) -> Self {
        self.type_args = Some(type_args);
        self
    }
    pub fn build(self) -> PathSegment {
        PathSegment::unwrap_cast(SyntaxNode::new_detached(SyntaxKind::PATH_SEGMENT, [
            Some(SyntaxElement::Node(self.root.into_syntax())),
            self.type_args
                .map(|token| SyntaxElement::Node(token.into_syntax())),
        ]))
    }
}
pub fn path_type(path: Path) -> PathType {
    PathType::unwrap_cast(SyntaxNode::new_detached(SyntaxKind::PATH_TYPE, [Some(
        SyntaxElement::Node(path.into_syntax()),
    )]))
}
pub fn project(project_token: SyntaxToken) -> Project {
    Project::unwrap_cast(SyntaxNode::new_detached(SyntaxKind::PROJECT, [Some(
        SyntaxElement::Token(project_token),
    )]))
}
pub fn string_literal(value_token: SyntaxToken) -> StringLiteral {
    StringLiteral::unwrap_cast(SyntaxNode::new_detached(SyntaxKind::STRING_LITERAL, [
        Some(SyntaxElement::Token(value_token)),
    ]))
}
pub fn type_annotation(colon_token: SyntaxToken, ty: Type) -> TypeAnnotation {
    TypeAnnotation::unwrap_cast(SyntaxNode::new_detached(SyntaxKind::TYPE_ANNOTATION, [
        Some(SyntaxElement::Token(colon_token)),
        Some(SyntaxElement::Node(ty.into_syntax())),
    ]))
}
pub fn type_args(
    l_brack_token: SyntaxToken,
    type_arg_list: TypeArgList,
    r_brack_token: SyntaxToken,
) -> TypeArgs {
    TypeArgs::unwrap_cast(SyntaxNode::new_detached(SyntaxKind::TYPE_ARGS, [
        Some(SyntaxElement::Token(l_brack_token)),
        Some(SyntaxElement::Node(type_arg_list.into_syntax())),
        Some(SyntaxElement::Token(r_brack_token)),
    ]))
}
pub fn type_decl(
    attributes: AttributeList,
    type_token: SyntaxToken,
    name: Name,
) -> TypeDeclBuilder {
    TypeDeclBuilder {
        attributes,
        type_token,
        name,
        visibility_token: None,
    }
}
pub struct TypeDeclBuilder {
    attributes: AttributeList,
    type_token: SyntaxToken,
    name: Name,
    visibility_token: Option<SyntaxToken>,
}
impl TypeDeclBuilder {
    pub fn with_visibility_token(mut self, visibility_token: SyntaxToken) -> Self {
        self.visibility_token = Some(visibility_token);
        self
    }
    pub fn build(self) -> TypeDecl {
        TypeDecl::unwrap_cast(SyntaxNode::new_detached(SyntaxKind::TYPE_DECL, [
            Some(SyntaxElement::Node(self.attributes.into_syntax())),
            self.visibility_token
                .map(|token| SyntaxElement::Token(token)),
            Some(SyntaxElement::Token(self.type_token)),
            Some(SyntaxElement::Node(self.name.into_syntax())),
        ]))
    }
}
pub fn unary_expr(operator_token_token: SyntaxToken, operand: Expr) -> UnaryExpr {
    UnaryExpr::unwrap_cast(SyntaxNode::new_detached(SyntaxKind::UNARY_EXPR, [
        Some(SyntaxElement::Token(operator_token_token)),
        Some(SyntaxElement::Node(operand.into_syntax())),
    ]))
}
pub fn use_alias(as_token: SyntaxToken, name: Name) -> UseAlias {
    UseAlias::unwrap_cast(SyntaxNode::new_detached(SyntaxKind::USE_ALIAS, [
        Some(SyntaxElement::Token(as_token)),
        Some(SyntaxElement::Node(name.into_syntax())),
    ]))
}
pub fn use_decl(use_token: SyntaxToken, path: Path) -> UseDeclBuilder {
    UseDeclBuilder {
        use_token,
        path,
        visibility_token: None,
        alias: None,
    }
}
pub struct UseDeclBuilder {
    use_token: SyntaxToken,
    path: Path,
    visibility_token: Option<SyntaxToken>,
    alias: Option<UseAlias>,
}
impl UseDeclBuilder {
    pub fn with_visibility_token(mut self, visibility_token: SyntaxToken) -> Self {
        self.visibility_token = Some(visibility_token);
        self
    }
    pub fn with_alias(mut self, alias: UseAlias) -> Self {
        self.alias = Some(alias);
        self
    }
    pub fn build(self) -> UseDecl {
        UseDecl::unwrap_cast(SyntaxNode::new_detached(SyntaxKind::USE_DECL, [
            self.visibility_token
                .map(|token| SyntaxElement::Token(token)),
            Some(SyntaxElement::Token(self.use_token)),
            Some(SyntaxElement::Node(self.path.into_syntax())),
            self.alias
                .map(|token| SyntaxElement::Node(token.into_syntax())),
        ]))
    }
}
pub fn wildcard_pat(underscore_token: SyntaxToken) -> WildcardPat {
    WildcardPat::unwrap_cast(SyntaxNode::new_detached(SyntaxKind::WILDCARD_PAT, [Some(
        SyntaxElement::Token(underscore_token),
    )]))
}
pub fn argument_list<I, S>(items: I, separators: S) -> ArgumentList
where
    I: IntoIterator<Item = Expr>,
    I::IntoIter: ExactSizeIterator,
    S: IntoIterator<Item = SyntaxToken>,
    S::IntoIter: ExactSizeIterator,
{
    let mut items = items.into_iter();
    let mut separators = separators.into_iter();
    let length = items.len() + separators.len();
    ArgumentList::unwrap_cast(SyntaxNode::new_detached(
        SyntaxKind::ARGUMENT_LIST,
        (0..length).map(|index| {
            if index % 2 == 0 {
                Some(items.next()?.into_syntax().into())
            } else {
                Some(separators.next()?.into())
            }
        }),
    ))
}
pub fn attribute_list<I>(items: I) -> AttributeList
where
    I: IntoIterator<Item = Attribute>,
    I::IntoIter: ExactSizeIterator,
{
    AttributeList::unwrap_cast(SyntaxNode::new_detached(
        SyntaxKind::ATTRIBUTE_LIST,
        items
            .into_iter()
            .map(|item| Some(item.into_syntax().into())),
    ))
}
pub fn module_item_list<I>(items: I) -> ModuleItemList
where
    I: IntoIterator<Item = ModuleItem>,
    I::IntoIter: ExactSizeIterator,
{
    ModuleItemList::unwrap_cast(SyntaxNode::new_detached(
        SyntaxKind::MODULE_ITEM_LIST,
        items
            .into_iter()
            .map(|item| Some(item.into_syntax().into())),
    ))
}
pub fn parameter_list<I, S>(items: I, separators: S) -> ParameterList
where
    I: IntoIterator<Item = AnyParameter>,
    I::IntoIter: ExactSizeIterator,
    S: IntoIterator<Item = SyntaxToken>,
    S::IntoIter: ExactSizeIterator,
{
    let mut items = items.into_iter();
    let mut separators = separators.into_iter();
    let length = items.len() + separators.len();
    ParameterList::unwrap_cast(SyntaxNode::new_detached(
        SyntaxKind::PARAMETER_LIST,
        (0..length).map(|index| {
            if index % 2 == 0 {
                Some(items.next()?.into_syntax().into())
            } else {
                Some(separators.next()?.into())
            }
        }),
    ))
}
pub fn type_arg_list<I, S>(items: I, separators: S) -> TypeArgList
where
    I: IntoIterator<Item = Type>,
    I::IntoIter: ExactSizeIterator,
    S: IntoIterator<Item = SyntaxToken>,
    S::IntoIter: ExactSizeIterator,
{
    let mut items = items.into_iter();
    let mut separators = separators.into_iter();
    let length = items.len() + separators.len();
    TypeArgList::unwrap_cast(SyntaxNode::new_detached(
        SyntaxKind::TYPE_ARG_LIST,
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
    Bogus::unwrap_cast(SyntaxNode::new_detached(SyntaxKind::BOGUS, slots))
}
pub fn bogus_decl<I>(slots: I) -> BogusDecl
where
    I: IntoIterator<Item = Option<SyntaxElement>>,
    I::IntoIter: ExactSizeIterator,
{
    BogusDecl::unwrap_cast(SyntaxNode::new_detached(SyntaxKind::BOGUS_DECL, slots))
}
pub fn bogus_expr<I>(slots: I) -> BogusExpr
where
    I: IntoIterator<Item = Option<SyntaxElement>>,
    I::IntoIter: ExactSizeIterator,
{
    BogusExpr::unwrap_cast(SyntaxNode::new_detached(SyntaxKind::BOGUS_EXPR, slots))
}
pub fn bogus_parameter<I>(slots: I) -> BogusParameter
where
    I: IntoIterator<Item = Option<SyntaxElement>>,
    I::IntoIter: ExactSizeIterator,
{
    BogusParameter::unwrap_cast(SyntaxNode::new_detached(SyntaxKind::BOGUS_PARAMETER, slots))
}
pub fn bogus_pat<I>(slots: I) -> BogusPat
where
    I: IntoIterator<Item = Option<SyntaxElement>>,
    I::IntoIter: ExactSizeIterator,
{
    BogusPat::unwrap_cast(SyntaxNode::new_detached(SyntaxKind::BOGUS_PAT, slots))
}
pub fn bogus_type<I>(slots: I) -> BogusType
where
    I: IntoIterator<Item = Option<SyntaxElement>>,
    I::IntoIter: ExactSizeIterator,
{
    BogusType::unwrap_cast(SyntaxNode::new_detached(SyntaxKind::BOGUS_TYPE, slots))
}
