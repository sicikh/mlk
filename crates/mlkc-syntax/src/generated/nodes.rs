//! Generated file, do not edit by hand, see `xtask/codegen`

#![allow(dead_code)]
#![allow(unused)]
use std::fmt::{Debug, Formatter};

use mlkc_rowan::{
    AstNode, AstNodeList, AstNodeListIterator, AstNodeSlotMap, AstSeparatedList,
    AstSeparatedListNodesIterator, RawSyntaxKind, SyntaxKindSet, SyntaxResult, support,
};
use serde::{
    Serialize, Serializer,
    ser::{SerializeMap, SerializeSeq},
};

use crate::{
    MlkLanguage as Language, SyntaxElement, SyntaxElementChildren,
    SyntaxKind::{self as SyntaxKind, *},
    SyntaxList, SyntaxNode, SyntaxToken,
    macros::map_syntax_node,
};
#[doc = r" Sentinel value indicating a missing element in a dynamic node, where"]
#[doc = r" the slots are not statically known."]
pub(crate) const SLOT_MAP_EMPTY_VALUE: u8 = u8::MAX;
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Attribute {
    pub(crate) syntax: SyntaxNode,
}
impl Attribute {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> AttributeFields {
        AttributeFields {
            at_token: self.at_token(),
            name: self.name(),
        }
    }
    pub fn at_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 0usize)
    }
    pub fn name(&self) -> SyntaxResult<Name> {
        support::required_node(&self.syntax, 1usize)
    }
}
impl Serialize for Attribute {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(Some(2))?;
        state.serialize_entry("kind", "Attribute")?;
        state.serialize_entry("fields", &self.as_fields())?;
        state.end()
    }
}
#[derive(Serialize)]
pub struct AttributeFields {
    pub at_token: SyntaxResult<SyntaxToken>,
    pub name: SyntaxResult<Name>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct BinExpr {
    pub(crate) syntax: SyntaxNode,
}
impl BinExpr {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> BinExprFields {
        BinExprFields {
            lhs: self.lhs(),
            operator_token: self.operator_token(),
            rhs: self.rhs(),
        }
    }
    pub fn lhs(&self) -> SyntaxResult<Expr> {
        support::required_node(&self.syntax, 0usize)
    }
    pub fn operator_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 1usize)
    }
    pub fn rhs(&self) -> SyntaxResult<Expr> {
        support::required_node(&self.syntax, 2usize)
    }
}
impl Serialize for BinExpr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(Some(2))?;
        state.serialize_entry("kind", "BinExpr")?;
        state.serialize_entry("fields", &self.as_fields())?;
        state.end()
    }
}
#[derive(Serialize)]
pub struct BinExprFields {
    pub lhs: SyntaxResult<Expr>,
    pub operator_token: SyntaxResult<SyntaxToken>,
    pub rhs: SyntaxResult<Expr>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct CallExpr {
    pub(crate) syntax: SyntaxNode,
}
impl CallExpr {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> CallExprFields {
        CallExprFields {
            function: self.function(),
            l_paren_token: self.l_paren_token(),
            arguments: self.arguments(),
            r_paren_token: self.r_paren_token(),
        }
    }
    pub fn function(&self) -> SyntaxResult<Expr> {
        support::required_node(&self.syntax, 0usize)
    }
    pub fn l_paren_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 1usize)
    }
    pub fn arguments(&self) -> ArgumentList {
        support::list(&self.syntax, 2usize)
    }
    pub fn r_paren_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 3usize)
    }
}
impl Serialize for CallExpr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(Some(2))?;
        state.serialize_entry("kind", "CallExpr")?;
        state.serialize_entry("fields", &self.as_fields())?;
        state.end()
    }
}
#[derive(Serialize)]
pub struct CallExprFields {
    pub function: SyntaxResult<Expr>,
    pub l_paren_token: SyntaxResult<SyntaxToken>,
    pub arguments: ArgumentList,
    pub r_paren_token: SyntaxResult<SyntaxToken>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct FunBody {
    pub(crate) syntax: SyntaxNode,
}
impl FunBody {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> FunBodyFields {
        FunBodyFields {
            eq_token: self.eq_token(),
            expr: self.expr(),
        }
    }
    pub fn eq_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 0usize)
    }
    pub fn expr(&self) -> SyntaxResult<Expr> {
        support::required_node(&self.syntax, 1usize)
    }
}
impl Serialize for FunBody {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(Some(2))?;
        state.serialize_entry("kind", "FunBody")?;
        state.serialize_entry("fields", &self.as_fields())?;
        state.end()
    }
}
#[derive(Serialize)]
pub struct FunBodyFields {
    pub eq_token: SyntaxResult<SyntaxToken>,
    pub expr: SyntaxResult<Expr>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct FunDecl {
    pub(crate) syntax: SyntaxNode,
}
impl FunDecl {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> FunDeclFields {
        FunDeclFields {
            attributes: self.attributes(),
            visibility_token: self.visibility_token(),
            fun_token: self.fun_token(),
            name: self.name(),
            parameters: self.parameters(),
            return_type_annotation: self.return_type_annotation(),
            body: self.body(),
        }
    }
    pub fn attributes(&self) -> AttributeList {
        support::list(&self.syntax, 0usize)
    }
    pub fn visibility_token(&self) -> Option<SyntaxToken> {
        support::token(&self.syntax, 1usize)
    }
    pub fn fun_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 2usize)
    }
    pub fn name(&self) -> SyntaxResult<Name> {
        support::required_node(&self.syntax, 3usize)
    }
    pub fn parameters(&self) -> SyntaxResult<Parameters> {
        support::required_node(&self.syntax, 4usize)
    }
    pub fn return_type_annotation(&self) -> Option<FunReturnTypeAnnotation> {
        support::node(&self.syntax, 5usize)
    }
    pub fn body(&self) -> Option<FunBody> {
        support::node(&self.syntax, 6usize)
    }
}
impl Serialize for FunDecl {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(Some(2))?;
        state.serialize_entry("kind", "FunDecl")?;
        state.serialize_entry("fields", &self.as_fields())?;
        state.end()
    }
}
#[derive(Serialize)]
pub struct FunDeclFields {
    pub attributes: AttributeList,
    pub visibility_token: Option<SyntaxToken>,
    pub fun_token: SyntaxResult<SyntaxToken>,
    pub name: SyntaxResult<Name>,
    pub parameters: SyntaxResult<Parameters>,
    pub return_type_annotation: Option<FunReturnTypeAnnotation>,
    pub body: Option<FunBody>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct FunReturnTypeAnnotation {
    pub(crate) syntax: SyntaxNode,
}
impl FunReturnTypeAnnotation {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> FunReturnTypeAnnotationFields {
        FunReturnTypeAnnotationFields {
            colon_token: self.colon_token(),
            return_type: self.return_type(),
        }
    }
    pub fn colon_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 0usize)
    }
    pub fn return_type(&self) -> SyntaxResult<Type> {
        support::required_node(&self.syntax, 1usize)
    }
}
impl Serialize for FunReturnTypeAnnotation {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(Some(2))?;
        state.serialize_entry("kind", "FunReturnTypeAnnotation")?;
        state.serialize_entry("fields", &self.as_fields())?;
        state.end()
    }
}
#[derive(Serialize)]
pub struct FunReturnTypeAnnotationFields {
    pub colon_token: SyntaxResult<SyntaxToken>,
    pub return_type: SyntaxResult<Type>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct IdentPat {
    pub(crate) syntax: SyntaxNode,
}
impl IdentPat {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> IdentPatFields {
        IdentPatFields { name: self.name() }
    }
    pub fn name(&self) -> SyntaxResult<Name> {
        support::required_node(&self.syntax, 0usize)
    }
}
impl Serialize for IdentPat {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(Some(2))?;
        state.serialize_entry("kind", "IdentPat")?;
        state.serialize_entry("fields", &self.as_fields())?;
        state.end()
    }
}
#[derive(Serialize)]
pub struct IdentPatFields {
    pub name: SyntaxResult<Name>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct InferType {
    pub(crate) syntax: SyntaxNode,
}
impl InferType {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> InferTypeFields {
        InferTypeFields {
            underscore_token: self.underscore_token(),
        }
    }
    pub fn underscore_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 0usize)
    }
}
impl Serialize for InferType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(Some(2))?;
        state.serialize_entry("kind", "InferType")?;
        state.serialize_entry("fields", &self.as_fields())?;
        state.end()
    }
}
#[derive(Serialize)]
pub struct InferTypeFields {
    pub underscore_token: SyntaxResult<SyntaxToken>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct IntLiteral {
    pub(crate) syntax: SyntaxNode,
}
impl IntLiteral {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> IntLiteralFields {
        IntLiteralFields {
            value_token: self.value_token(),
        }
    }
    pub fn value_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 0usize)
    }
}
impl Serialize for IntLiteral {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(Some(2))?;
        state.serialize_entry("kind", "IntLiteral")?;
        state.serialize_entry("fields", &self.as_fields())?;
        state.end()
    }
}
#[derive(Serialize)]
pub struct IntLiteralFields {
    pub value_token: SyntaxResult<SyntaxToken>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct LetExpr {
    pub(crate) syntax: SyntaxNode,
}
impl LetExpr {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> LetExprFields {
        LetExprFields {
            let_token: self.let_token(),
            pat: self.pat(),
            eq_token: self.eq_token(),
            expr: self.expr(),
            in_token: self.in_token(),
            body: self.body(),
        }
    }
    pub fn let_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 0usize)
    }
    pub fn pat(&self) -> SyntaxResult<Pat> {
        support::required_node(&self.syntax, 1usize)
    }
    pub fn eq_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 2usize)
    }
    pub fn expr(&self) -> SyntaxResult<Expr> {
        support::required_node(&self.syntax, 3usize)
    }
    pub fn in_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 4usize)
    }
    pub fn body(&self) -> SyntaxResult<Expr> {
        support::required_node(&self.syntax, 5usize)
    }
}
impl Serialize for LetExpr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(Some(2))?;
        state.serialize_entry("kind", "LetExpr")?;
        state.serialize_entry("fields", &self.as_fields())?;
        state.end()
    }
}
#[derive(Serialize)]
pub struct LetExprFields {
    pub let_token: SyntaxResult<SyntaxToken>,
    pub pat: SyntaxResult<Pat>,
    pub eq_token: SyntaxResult<SyntaxToken>,
    pub expr: SyntaxResult<Expr>,
    pub in_token: SyntaxResult<SyntaxToken>,
    pub body: SyntaxResult<Expr>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ModulePreamble {
    pub(crate) syntax: SyntaxNode,
}
impl ModulePreamble {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> ModulePreambleFields {
        ModulePreambleFields {
            attributes: self.attributes(),
            module_token: self.module_token(),
            name: self.name(),
        }
    }
    pub fn attributes(&self) -> AttributeList {
        support::list(&self.syntax, 0usize)
    }
    pub fn module_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 1usize)
    }
    pub fn name(&self) -> SyntaxResult<Path> {
        support::required_node(&self.syntax, 2usize)
    }
}
impl Serialize for ModulePreamble {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(Some(2))?;
        state.serialize_entry("kind", "ModulePreamble")?;
        state.serialize_entry("fields", &self.as_fields())?;
        state.end()
    }
}
#[derive(Serialize)]
pub struct ModulePreambleFields {
    pub attributes: AttributeList,
    pub module_token: SyntaxResult<SyntaxToken>,
    pub name: SyntaxResult<Path>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ModuleRoot {
    pub(crate) syntax: SyntaxNode,
}
impl ModuleRoot {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> ModuleRootFields {
        ModuleRootFields {
            bom_token: self.bom_token(),
            preamble: self.preamble(),
            items: self.items(),
            eof_token: self.eof_token(),
        }
    }
    pub fn bom_token(&self) -> Option<SyntaxToken> {
        support::token(&self.syntax, 0usize)
    }
    pub fn preamble(&self) -> Option<ModulePreamble> {
        support::node(&self.syntax, 1usize)
    }
    pub fn items(&self) -> ModuleItemList {
        support::list(&self.syntax, 2usize)
    }
    pub fn eof_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 3usize)
    }
}
impl Serialize for ModuleRoot {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(Some(2))?;
        state.serialize_entry("kind", "ModuleRoot")?;
        state.serialize_entry("fields", &self.as_fields())?;
        state.end()
    }
}
#[derive(Serialize)]
pub struct ModuleRootFields {
    pub bom_token: Option<SyntaxToken>,
    pub preamble: Option<ModulePreamble>,
    pub items: ModuleItemList,
    pub eof_token: SyntaxResult<SyntaxToken>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Name {
    pub(crate) syntax: SyntaxNode,
}
impl Name {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> NameFields {
        NameFields {
            value_token: self.value_token(),
        }
    }
    pub fn value_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 0usize)
    }
}
impl Serialize for Name {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(Some(2))?;
        state.serialize_entry("kind", "Name")?;
        state.serialize_entry("fields", &self.as_fields())?;
        state.end()
    }
}
#[derive(Serialize)]
pub struct NameFields {
    pub value_token: SyntaxResult<SyntaxToken>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Parameter {
    pub(crate) syntax: SyntaxNode,
}
impl Parameter {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> ParameterFields {
        ParameterFields {
            pat: self.pat(),
            type_annotation: self.type_annotation(),
        }
    }
    pub fn pat(&self) -> SyntaxResult<Pat> {
        support::required_node(&self.syntax, 0usize)
    }
    pub fn type_annotation(&self) -> Option<TypeAnnotation> {
        support::node(&self.syntax, 1usize)
    }
}
impl Serialize for Parameter {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(Some(2))?;
        state.serialize_entry("kind", "Parameter")?;
        state.serialize_entry("fields", &self.as_fields())?;
        state.end()
    }
}
#[derive(Serialize)]
pub struct ParameterFields {
    pub pat: SyntaxResult<Pat>,
    pub type_annotation: Option<TypeAnnotation>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Parameters {
    pub(crate) syntax: SyntaxNode,
}
impl Parameters {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> ParametersFields {
        ParametersFields {
            l_paren_token: self.l_paren_token(),
            items: self.items(),
            r_paren_token: self.r_paren_token(),
        }
    }
    pub fn l_paren_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 0usize)
    }
    pub fn items(&self) -> ParameterList {
        support::list(&self.syntax, 1usize)
    }
    pub fn r_paren_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 2usize)
    }
}
impl Serialize for Parameters {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(Some(2))?;
        state.serialize_entry("kind", "Parameters")?;
        state.serialize_entry("fields", &self.as_fields())?;
        state.end()
    }
}
#[derive(Serialize)]
pub struct ParametersFields {
    pub l_paren_token: SyntaxResult<SyntaxToken>,
    pub items: ParameterList,
    pub r_paren_token: SyntaxResult<SyntaxToken>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ParenExpr {
    pub(crate) syntax: SyntaxNode,
}
impl ParenExpr {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> ParenExprFields {
        ParenExprFields {
            l_paren_token: self.l_paren_token(),
            expr: self.expr(),
            r_paren_token: self.r_paren_token(),
        }
    }
    pub fn l_paren_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 0usize)
    }
    pub fn expr(&self) -> SyntaxResult<Expr> {
        support::required_node(&self.syntax, 1usize)
    }
    pub fn r_paren_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 2usize)
    }
}
impl Serialize for ParenExpr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(Some(2))?;
        state.serialize_entry("kind", "ParenExpr")?;
        state.serialize_entry("fields", &self.as_fields())?;
        state.end()
    }
}
#[derive(Serialize)]
pub struct ParenExprFields {
    pub l_paren_token: SyntaxResult<SyntaxToken>,
    pub expr: SyntaxResult<Expr>,
    pub r_paren_token: SyntaxResult<SyntaxToken>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Path {
    pub(crate) syntax: SyntaxNode,
}
impl Path {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> PathFields {
        PathFields {
            qualifier: self.qualifier(),
            segment: self.segment(),
        }
    }
    pub fn qualifier(&self) -> Option<PathQualifier> {
        support::node(&self.syntax, 0usize)
    }
    pub fn segment(&self) -> SyntaxResult<PathSegment> {
        support::required_node(&self.syntax, 1usize)
    }
}
impl Serialize for Path {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(Some(2))?;
        state.serialize_entry("kind", "Path")?;
        state.serialize_entry("fields", &self.as_fields())?;
        state.end()
    }
}
#[derive(Serialize)]
pub struct PathFields {
    pub qualifier: Option<PathQualifier>,
    pub segment: SyntaxResult<PathSegment>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct PathExpr {
    pub(crate) syntax: SyntaxNode,
}
impl PathExpr {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> PathExprFields {
        PathExprFields { path: self.path() }
    }
    pub fn path(&self) -> SyntaxResult<Path> {
        support::required_node(&self.syntax, 0usize)
    }
}
impl Serialize for PathExpr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(Some(2))?;
        state.serialize_entry("kind", "PathExpr")?;
        state.serialize_entry("fields", &self.as_fields())?;
        state.end()
    }
}
#[derive(Serialize)]
pub struct PathExprFields {
    pub path: SyntaxResult<Path>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct PathQualifier {
    pub(crate) syntax: SyntaxNode,
}
impl PathQualifier {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> PathQualifierFields {
        PathQualifierFields {
            path: self.path(),
            dot_token: self.dot_token(),
        }
    }
    pub fn path(&self) -> SyntaxResult<Path> {
        support::required_node(&self.syntax, 0usize)
    }
    pub fn dot_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 1usize)
    }
}
impl Serialize for PathQualifier {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(Some(2))?;
        state.serialize_entry("kind", "PathQualifier")?;
        state.serialize_entry("fields", &self.as_fields())?;
        state.end()
    }
}
#[derive(Serialize)]
pub struct PathQualifierFields {
    pub path: SyntaxResult<Path>,
    pub dot_token: SyntaxResult<SyntaxToken>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct PathSegment {
    pub(crate) syntax: SyntaxNode,
}
impl PathSegment {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> PathSegmentFields {
        PathSegmentFields {
            root: self.root(),
            type_args: self.type_args(),
        }
    }
    pub fn root(&self) -> SyntaxResult<PathRoot> {
        support::required_node(&self.syntax, 0usize)
    }
    pub fn type_args(&self) -> Option<TypeArgs> {
        support::node(&self.syntax, 1usize)
    }
}
impl Serialize for PathSegment {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(Some(2))?;
        state.serialize_entry("kind", "PathSegment")?;
        state.serialize_entry("fields", &self.as_fields())?;
        state.end()
    }
}
#[derive(Serialize)]
pub struct PathSegmentFields {
    pub root: SyntaxResult<PathRoot>,
    pub type_args: Option<TypeArgs>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct PathType {
    pub(crate) syntax: SyntaxNode,
}
impl PathType {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> PathTypeFields {
        PathTypeFields { path: self.path() }
    }
    pub fn path(&self) -> SyntaxResult<Path> {
        support::required_node(&self.syntax, 0usize)
    }
}
impl Serialize for PathType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(Some(2))?;
        state.serialize_entry("kind", "PathType")?;
        state.serialize_entry("fields", &self.as_fields())?;
        state.end()
    }
}
#[derive(Serialize)]
pub struct PathTypeFields {
    pub path: SyntaxResult<Path>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Project {
    pub(crate) syntax: SyntaxNode,
}
impl Project {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> ProjectFields {
        ProjectFields {
            project_token: self.project_token(),
        }
    }
    pub fn project_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 0usize)
    }
}
impl Serialize for Project {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(Some(2))?;
        state.serialize_entry("kind", "Project")?;
        state.serialize_entry("fields", &self.as_fields())?;
        state.end()
    }
}
#[derive(Serialize)]
pub struct ProjectFields {
    pub project_token: SyntaxResult<SyntaxToken>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct StringLiteral {
    pub(crate) syntax: SyntaxNode,
}
impl StringLiteral {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> StringLiteralFields {
        StringLiteralFields {
            value_token: self.value_token(),
        }
    }
    pub fn value_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 0usize)
    }
}
impl Serialize for StringLiteral {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(Some(2))?;
        state.serialize_entry("kind", "StringLiteral")?;
        state.serialize_entry("fields", &self.as_fields())?;
        state.end()
    }
}
#[derive(Serialize)]
pub struct StringLiteralFields {
    pub value_token: SyntaxResult<SyntaxToken>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct TypeAnnotation {
    pub(crate) syntax: SyntaxNode,
}
impl TypeAnnotation {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> TypeAnnotationFields {
        TypeAnnotationFields {
            colon_token: self.colon_token(),
            ty: self.ty(),
        }
    }
    pub fn colon_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 0usize)
    }
    pub fn ty(&self) -> SyntaxResult<Type> {
        support::required_node(&self.syntax, 1usize)
    }
}
impl Serialize for TypeAnnotation {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(Some(2))?;
        state.serialize_entry("kind", "TypeAnnotation")?;
        state.serialize_entry("fields", &self.as_fields())?;
        state.end()
    }
}
#[derive(Serialize)]
pub struct TypeAnnotationFields {
    pub colon_token: SyntaxResult<SyntaxToken>,
    pub ty: SyntaxResult<Type>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct TypeArgs {
    pub(crate) syntax: SyntaxNode,
}
impl TypeArgs {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> TypeArgsFields {
        TypeArgsFields {
            l_brack_token: self.l_brack_token(),
            type_arg_list: self.type_arg_list(),
            r_brack_token: self.r_brack_token(),
        }
    }
    pub fn l_brack_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 0usize)
    }
    pub fn type_arg_list(&self) -> TypeArgList {
        support::list(&self.syntax, 1usize)
    }
    pub fn r_brack_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 2usize)
    }
}
impl Serialize for TypeArgs {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(Some(2))?;
        state.serialize_entry("kind", "TypeArgs")?;
        state.serialize_entry("fields", &self.as_fields())?;
        state.end()
    }
}
#[derive(Serialize)]
pub struct TypeArgsFields {
    pub l_brack_token: SyntaxResult<SyntaxToken>,
    pub type_arg_list: TypeArgList,
    pub r_brack_token: SyntaxResult<SyntaxToken>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct TypeDecl {
    pub(crate) syntax: SyntaxNode,
}
impl TypeDecl {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> TypeDeclFields {
        TypeDeclFields {
            attributes: self.attributes(),
            visibility_token: self.visibility_token(),
            type_token: self.type_token(),
            name: self.name(),
        }
    }
    pub fn attributes(&self) -> AttributeList {
        support::list(&self.syntax, 0usize)
    }
    pub fn visibility_token(&self) -> Option<SyntaxToken> {
        support::token(&self.syntax, 1usize)
    }
    pub fn type_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 2usize)
    }
    pub fn name(&self) -> SyntaxResult<Name> {
        support::required_node(&self.syntax, 3usize)
    }
}
impl Serialize for TypeDecl {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(Some(2))?;
        state.serialize_entry("kind", "TypeDecl")?;
        state.serialize_entry("fields", &self.as_fields())?;
        state.end()
    }
}
#[derive(Serialize)]
pub struct TypeDeclFields {
    pub attributes: AttributeList,
    pub visibility_token: Option<SyntaxToken>,
    pub type_token: SyntaxResult<SyntaxToken>,
    pub name: SyntaxResult<Name>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct UnaryExpr {
    pub(crate) syntax: SyntaxNode,
}
impl UnaryExpr {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> UnaryExprFields {
        UnaryExprFields {
            operator_token: self.operator_token(),
            operand: self.operand(),
        }
    }
    pub fn operator_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 0usize)
    }
    pub fn operand(&self) -> SyntaxResult<Expr> {
        support::required_node(&self.syntax, 1usize)
    }
}
impl Serialize for UnaryExpr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(Some(2))?;
        state.serialize_entry("kind", "UnaryExpr")?;
        state.serialize_entry("fields", &self.as_fields())?;
        state.end()
    }
}
#[derive(Serialize)]
pub struct UnaryExprFields {
    pub operator_token: SyntaxResult<SyntaxToken>,
    pub operand: SyntaxResult<Expr>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct UseAlias {
    pub(crate) syntax: SyntaxNode,
}
impl UseAlias {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> UseAliasFields {
        UseAliasFields {
            as_token: self.as_token(),
            name: self.name(),
        }
    }
    pub fn as_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 0usize)
    }
    pub fn name(&self) -> SyntaxResult<Name> {
        support::required_node(&self.syntax, 1usize)
    }
}
impl Serialize for UseAlias {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(Some(2))?;
        state.serialize_entry("kind", "UseAlias")?;
        state.serialize_entry("fields", &self.as_fields())?;
        state.end()
    }
}
#[derive(Serialize)]
pub struct UseAliasFields {
    pub as_token: SyntaxResult<SyntaxToken>,
    pub name: SyntaxResult<Name>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct UseDecl {
    pub(crate) syntax: SyntaxNode,
}
impl UseDecl {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> UseDeclFields {
        UseDeclFields {
            visibility_token: self.visibility_token(),
            use_token: self.use_token(),
            path: self.path(),
            alias: self.alias(),
        }
    }
    pub fn visibility_token(&self) -> Option<SyntaxToken> {
        support::token(&self.syntax, 0usize)
    }
    pub fn use_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 1usize)
    }
    pub fn path(&self) -> SyntaxResult<Path> {
        support::required_node(&self.syntax, 2usize)
    }
    pub fn alias(&self) -> Option<UseAlias> {
        support::node(&self.syntax, 3usize)
    }
}
impl Serialize for UseDecl {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(Some(2))?;
        state.serialize_entry("kind", "UseDecl")?;
        state.serialize_entry("fields", &self.as_fields())?;
        state.end()
    }
}
#[derive(Serialize)]
pub struct UseDeclFields {
    pub visibility_token: Option<SyntaxToken>,
    pub use_token: SyntaxResult<SyntaxToken>,
    pub path: SyntaxResult<Path>,
    pub alias: Option<UseAlias>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct WildcardPat {
    pub(crate) syntax: SyntaxNode,
}
impl WildcardPat {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> WildcardPatFields {
        WildcardPatFields {
            underscore_token: self.underscore_token(),
        }
    }
    pub fn underscore_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 0usize)
    }
}
impl Serialize for WildcardPat {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(Some(2))?;
        state.serialize_entry("kind", "WildcardPat")?;
        state.serialize_entry("fields", &self.as_fields())?;
        state.end()
    }
}
#[derive(Serialize)]
pub struct WildcardPatFields {
    pub underscore_token: SyntaxResult<SyntaxToken>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub enum AnyParameter {
    BogusParameter(BogusParameter),
    Parameter(Parameter),
}
impl Serialize for AnyParameter {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::BogusParameter(it) => it.serialize(serializer),
            Self::Parameter(it) => it.serialize(serializer),
        }
    }
}
impl AnyParameter {
    pub fn as_bogus_parameter(&self) -> Option<&BogusParameter> {
        match &self {
            Self::BogusParameter(item) => Some(item),
            _ => None,
        }
    }
    pub fn as_parameter(&self) -> Option<&Parameter> {
        match &self {
            Self::Parameter(item) => Some(item),
            _ => None,
        }
    }
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Expr {
    BinExpr(BinExpr),
    BogusExpr(BogusExpr),
    CallExpr(CallExpr),
    LetExpr(LetExpr),
    Literal(Literal),
    ParenExpr(ParenExpr),
    PathExpr(PathExpr),
    UnaryExpr(UnaryExpr),
}
impl Serialize for Expr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::BinExpr(it) => it.serialize(serializer),
            Self::BogusExpr(it) => it.serialize(serializer),
            Self::CallExpr(it) => it.serialize(serializer),
            Self::LetExpr(it) => it.serialize(serializer),
            Self::Literal(it) => it.serialize(serializer),
            Self::ParenExpr(it) => it.serialize(serializer),
            Self::PathExpr(it) => it.serialize(serializer),
            Self::UnaryExpr(it) => it.serialize(serializer),
        }
    }
}
impl Expr {
    pub fn as_bin_expr(&self) -> Option<&BinExpr> {
        match &self {
            Self::BinExpr(item) => Some(item),
            _ => None,
        }
    }
    pub fn as_bogus_expr(&self) -> Option<&BogusExpr> {
        match &self {
            Self::BogusExpr(item) => Some(item),
            _ => None,
        }
    }
    pub fn as_call_expr(&self) -> Option<&CallExpr> {
        match &self {
            Self::CallExpr(item) => Some(item),
            _ => None,
        }
    }
    pub fn as_let_expr(&self) -> Option<&LetExpr> {
        match &self {
            Self::LetExpr(item) => Some(item),
            _ => None,
        }
    }
    pub fn as_literal(&self) -> Option<&Literal> {
        match &self {
            Self::Literal(item) => Some(item),
            _ => None,
        }
    }
    pub fn as_paren_expr(&self) -> Option<&ParenExpr> {
        match &self {
            Self::ParenExpr(item) => Some(item),
            _ => None,
        }
    }
    pub fn as_path_expr(&self) -> Option<&PathExpr> {
        match &self {
            Self::PathExpr(item) => Some(item),
            _ => None,
        }
    }
    pub fn as_unary_expr(&self) -> Option<&UnaryExpr> {
        match &self {
            Self::UnaryExpr(item) => Some(item),
            _ => None,
        }
    }
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Literal {
    IntLiteral(IntLiteral),
    StringLiteral(StringLiteral),
}
impl Serialize for Literal {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::IntLiteral(it) => it.serialize(serializer),
            Self::StringLiteral(it) => it.serialize(serializer),
        }
    }
}
impl Literal {
    pub fn as_int_literal(&self) -> Option<&IntLiteral> {
        match &self {
            Self::IntLiteral(item) => Some(item),
            _ => None,
        }
    }
    pub fn as_string_literal(&self) -> Option<&StringLiteral> {
        match &self {
            Self::StringLiteral(item) => Some(item),
            _ => None,
        }
    }
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub enum ModuleItem {
    BogusDecl(BogusDecl),
    FunDecl(FunDecl),
    TypeDecl(TypeDecl),
    UseDecl(UseDecl),
}
impl Serialize for ModuleItem {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::BogusDecl(it) => it.serialize(serializer),
            Self::FunDecl(it) => it.serialize(serializer),
            Self::TypeDecl(it) => it.serialize(serializer),
            Self::UseDecl(it) => it.serialize(serializer),
        }
    }
}
impl ModuleItem {
    pub fn as_bogus_decl(&self) -> Option<&BogusDecl> {
        match &self {
            Self::BogusDecl(item) => Some(item),
            _ => None,
        }
    }
    pub fn as_fun_decl(&self) -> Option<&FunDecl> {
        match &self {
            Self::FunDecl(item) => Some(item),
            _ => None,
        }
    }
    pub fn as_type_decl(&self) -> Option<&TypeDecl> {
        match &self {
            Self::TypeDecl(item) => Some(item),
            _ => None,
        }
    }
    pub fn as_use_decl(&self) -> Option<&UseDecl> {
        match &self {
            Self::UseDecl(item) => Some(item),
            _ => None,
        }
    }
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Pat {
    BogusPat(BogusPat),
    IdentPat(IdentPat),
    WildcardPat(WildcardPat),
}
impl Serialize for Pat {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::BogusPat(it) => it.serialize(serializer),
            Self::IdentPat(it) => it.serialize(serializer),
            Self::WildcardPat(it) => it.serialize(serializer),
        }
    }
}
impl Pat {
    pub fn as_bogus_pat(&self) -> Option<&BogusPat> {
        match &self {
            Self::BogusPat(item) => Some(item),
            _ => None,
        }
    }
    pub fn as_ident_pat(&self) -> Option<&IdentPat> {
        match &self {
            Self::IdentPat(item) => Some(item),
            _ => None,
        }
    }
    pub fn as_wildcard_pat(&self) -> Option<&WildcardPat> {
        match &self {
            Self::WildcardPat(item) => Some(item),
            _ => None,
        }
    }
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub enum PathRoot {
    Name(Name),
    Project(Project),
}
impl Serialize for PathRoot {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Name(it) => it.serialize(serializer),
            Self::Project(it) => it.serialize(serializer),
        }
    }
}
impl PathRoot {
    pub fn as_name(&self) -> Option<&Name> {
        match &self {
            Self::Name(item) => Some(item),
            _ => None,
        }
    }
    pub fn as_project(&self) -> Option<&Project> {
        match &self {
            Self::Project(item) => Some(item),
            _ => None,
        }
    }
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Type {
    BogusType(BogusType),
    InferType(InferType),
    PathType(PathType),
}
impl Serialize for Type {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::BogusType(it) => it.serialize(serializer),
            Self::InferType(it) => it.serialize(serializer),
            Self::PathType(it) => it.serialize(serializer),
        }
    }
}
impl Type {
    pub fn as_bogus_type(&self) -> Option<&BogusType> {
        match &self {
            Self::BogusType(item) => Some(item),
            _ => None,
        }
    }
    pub fn as_infer_type(&self) -> Option<&InferType> {
        match &self {
            Self::InferType(item) => Some(item),
            _ => None,
        }
    }
    pub fn as_path_type(&self) -> Option<&PathType> {
        match &self {
            Self::PathType(item) => Some(item),
            _ => None,
        }
    }
}
impl AstNode for Attribute {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(ATTRIBUTE as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == ATTRIBUTE
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(Self { syntax })
        } else {
            None
        }
    }
    fn syntax(&self) -> &SyntaxNode {
        &self.syntax
    }
    fn into_syntax(self) -> SyntaxNode {
        self.syntax
    }
}
impl std::fmt::Debug for Attribute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("Attribute")
                .field("at_token", &support::DebugSyntaxResult(self.at_token()))
                .field("name", &support::DebugSyntaxResult(self.name()))
                .finish()
        } else {
            f.debug_struct("Attribute").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<Attribute> for SyntaxNode {
    fn from(n: Attribute) -> Self {
        n.syntax
    }
}
impl From<Attribute> for SyntaxElement {
    fn from(n: Attribute) -> Self {
        n.syntax.into()
    }
}
impl AstNode for BinExpr {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(BIN_EXPR as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == BIN_EXPR
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(Self { syntax })
        } else {
            None
        }
    }
    fn syntax(&self) -> &SyntaxNode {
        &self.syntax
    }
    fn into_syntax(self) -> SyntaxNode {
        self.syntax
    }
}
impl std::fmt::Debug for BinExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("BinExpr")
                .field("lhs", &support::DebugSyntaxResult(self.lhs()))
                .field(
                    "operator_token",
                    &support::DebugSyntaxResult(self.operator_token()),
                )
                .field("rhs", &support::DebugSyntaxResult(self.rhs()))
                .finish()
        } else {
            f.debug_struct("BinExpr").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<BinExpr> for SyntaxNode {
    fn from(n: BinExpr) -> Self {
        n.syntax
    }
}
impl From<BinExpr> for SyntaxElement {
    fn from(n: BinExpr) -> Self {
        n.syntax.into()
    }
}
impl AstNode for CallExpr {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(CALL_EXPR as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == CALL_EXPR
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(Self { syntax })
        } else {
            None
        }
    }
    fn syntax(&self) -> &SyntaxNode {
        &self.syntax
    }
    fn into_syntax(self) -> SyntaxNode {
        self.syntax
    }
}
impl std::fmt::Debug for CallExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("CallExpr")
                .field("function", &support::DebugSyntaxResult(self.function()))
                .field(
                    "l_paren_token",
                    &support::DebugSyntaxResult(self.l_paren_token()),
                )
                .field("arguments", &self.arguments())
                .field(
                    "r_paren_token",
                    &support::DebugSyntaxResult(self.r_paren_token()),
                )
                .finish()
        } else {
            f.debug_struct("CallExpr").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<CallExpr> for SyntaxNode {
    fn from(n: CallExpr) -> Self {
        n.syntax
    }
}
impl From<CallExpr> for SyntaxElement {
    fn from(n: CallExpr) -> Self {
        n.syntax.into()
    }
}
impl AstNode for FunBody {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(FUN_BODY as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == FUN_BODY
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(Self { syntax })
        } else {
            None
        }
    }
    fn syntax(&self) -> &SyntaxNode {
        &self.syntax
    }
    fn into_syntax(self) -> SyntaxNode {
        self.syntax
    }
}
impl std::fmt::Debug for FunBody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("FunBody")
                .field("eq_token", &support::DebugSyntaxResult(self.eq_token()))
                .field("expr", &support::DebugSyntaxResult(self.expr()))
                .finish()
        } else {
            f.debug_struct("FunBody").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<FunBody> for SyntaxNode {
    fn from(n: FunBody) -> Self {
        n.syntax
    }
}
impl From<FunBody> for SyntaxElement {
    fn from(n: FunBody) -> Self {
        n.syntax.into()
    }
}
impl AstNode for FunDecl {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(FUN_DECL as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == FUN_DECL
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(Self { syntax })
        } else {
            None
        }
    }
    fn syntax(&self) -> &SyntaxNode {
        &self.syntax
    }
    fn into_syntax(self) -> SyntaxNode {
        self.syntax
    }
}
impl std::fmt::Debug for FunDecl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("FunDecl")
                .field("attributes", &self.attributes())
                .field(
                    "visibility_token",
                    &support::DebugOptionalElement(self.visibility_token()),
                )
                .field("fun_token", &support::DebugSyntaxResult(self.fun_token()))
                .field("name", &support::DebugSyntaxResult(self.name()))
                .field("parameters", &support::DebugSyntaxResult(self.parameters()))
                .field(
                    "return_type_annotation",
                    &support::DebugOptionalElement(self.return_type_annotation()),
                )
                .field("body", &support::DebugOptionalElement(self.body()))
                .finish()
        } else {
            f.debug_struct("FunDecl").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<FunDecl> for SyntaxNode {
    fn from(n: FunDecl) -> Self {
        n.syntax
    }
}
impl From<FunDecl> for SyntaxElement {
    fn from(n: FunDecl) -> Self {
        n.syntax.into()
    }
}
impl AstNode for FunReturnTypeAnnotation {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(FUN_RETURN_TYPE_ANNOTATION as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == FUN_RETURN_TYPE_ANNOTATION
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(Self { syntax })
        } else {
            None
        }
    }
    fn syntax(&self) -> &SyntaxNode {
        &self.syntax
    }
    fn into_syntax(self) -> SyntaxNode {
        self.syntax
    }
}
impl std::fmt::Debug for FunReturnTypeAnnotation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("FunReturnTypeAnnotation")
                .field(
                    "colon_token",
                    &support::DebugSyntaxResult(self.colon_token()),
                )
                .field(
                    "return_type",
                    &support::DebugSyntaxResult(self.return_type()),
                )
                .finish()
        } else {
            f.debug_struct("FunReturnTypeAnnotation").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<FunReturnTypeAnnotation> for SyntaxNode {
    fn from(n: FunReturnTypeAnnotation) -> Self {
        n.syntax
    }
}
impl From<FunReturnTypeAnnotation> for SyntaxElement {
    fn from(n: FunReturnTypeAnnotation) -> Self {
        n.syntax.into()
    }
}
impl AstNode for IdentPat {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(IDENT_PAT as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == IDENT_PAT
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(Self { syntax })
        } else {
            None
        }
    }
    fn syntax(&self) -> &SyntaxNode {
        &self.syntax
    }
    fn into_syntax(self) -> SyntaxNode {
        self.syntax
    }
}
impl std::fmt::Debug for IdentPat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("IdentPat")
                .field("name", &support::DebugSyntaxResult(self.name()))
                .finish()
        } else {
            f.debug_struct("IdentPat").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<IdentPat> for SyntaxNode {
    fn from(n: IdentPat) -> Self {
        n.syntax
    }
}
impl From<IdentPat> for SyntaxElement {
    fn from(n: IdentPat) -> Self {
        n.syntax.into()
    }
}
impl AstNode for InferType {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(INFER_TYPE as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == INFER_TYPE
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(Self { syntax })
        } else {
            None
        }
    }
    fn syntax(&self) -> &SyntaxNode {
        &self.syntax
    }
    fn into_syntax(self) -> SyntaxNode {
        self.syntax
    }
}
impl std::fmt::Debug for InferType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("InferType")
                .field(
                    "underscore_token",
                    &support::DebugSyntaxResult(self.underscore_token()),
                )
                .finish()
        } else {
            f.debug_struct("InferType").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<InferType> for SyntaxNode {
    fn from(n: InferType) -> Self {
        n.syntax
    }
}
impl From<InferType> for SyntaxElement {
    fn from(n: InferType) -> Self {
        n.syntax.into()
    }
}
impl AstNode for IntLiteral {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(INT_LITERAL as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == INT_LITERAL
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(Self { syntax })
        } else {
            None
        }
    }
    fn syntax(&self) -> &SyntaxNode {
        &self.syntax
    }
    fn into_syntax(self) -> SyntaxNode {
        self.syntax
    }
}
impl std::fmt::Debug for IntLiteral {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("IntLiteral")
                .field(
                    "value_token",
                    &support::DebugSyntaxResult(self.value_token()),
                )
                .finish()
        } else {
            f.debug_struct("IntLiteral").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<IntLiteral> for SyntaxNode {
    fn from(n: IntLiteral) -> Self {
        n.syntax
    }
}
impl From<IntLiteral> for SyntaxElement {
    fn from(n: IntLiteral) -> Self {
        n.syntax.into()
    }
}
impl AstNode for LetExpr {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(LET_EXPR as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == LET_EXPR
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(Self { syntax })
        } else {
            None
        }
    }
    fn syntax(&self) -> &SyntaxNode {
        &self.syntax
    }
    fn into_syntax(self) -> SyntaxNode {
        self.syntax
    }
}
impl std::fmt::Debug for LetExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("LetExpr")
                .field("let_token", &support::DebugSyntaxResult(self.let_token()))
                .field("pat", &support::DebugSyntaxResult(self.pat()))
                .field("eq_token", &support::DebugSyntaxResult(self.eq_token()))
                .field("expr", &support::DebugSyntaxResult(self.expr()))
                .field("in_token", &support::DebugSyntaxResult(self.in_token()))
                .field("body", &support::DebugSyntaxResult(self.body()))
                .finish()
        } else {
            f.debug_struct("LetExpr").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<LetExpr> for SyntaxNode {
    fn from(n: LetExpr) -> Self {
        n.syntax
    }
}
impl From<LetExpr> for SyntaxElement {
    fn from(n: LetExpr) -> Self {
        n.syntax.into()
    }
}
impl AstNode for ModulePreamble {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(MODULE_PREAMBLE as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == MODULE_PREAMBLE
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(Self { syntax })
        } else {
            None
        }
    }
    fn syntax(&self) -> &SyntaxNode {
        &self.syntax
    }
    fn into_syntax(self) -> SyntaxNode {
        self.syntax
    }
}
impl std::fmt::Debug for ModulePreamble {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("ModulePreamble")
                .field("attributes", &self.attributes())
                .field(
                    "module_token",
                    &support::DebugSyntaxResult(self.module_token()),
                )
                .field("name", &support::DebugSyntaxResult(self.name()))
                .finish()
        } else {
            f.debug_struct("ModulePreamble").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<ModulePreamble> for SyntaxNode {
    fn from(n: ModulePreamble) -> Self {
        n.syntax
    }
}
impl From<ModulePreamble> for SyntaxElement {
    fn from(n: ModulePreamble) -> Self {
        n.syntax.into()
    }
}
impl AstNode for ModuleRoot {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(MODULE_ROOT as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == MODULE_ROOT
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(Self { syntax })
        } else {
            None
        }
    }
    fn syntax(&self) -> &SyntaxNode {
        &self.syntax
    }
    fn into_syntax(self) -> SyntaxNode {
        self.syntax
    }
}
impl std::fmt::Debug for ModuleRoot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("ModuleRoot")
                .field(
                    "bom_token",
                    &support::DebugOptionalElement(self.bom_token()),
                )
                .field("preamble", &support::DebugOptionalElement(self.preamble()))
                .field("items", &self.items())
                .field("eof_token", &support::DebugSyntaxResult(self.eof_token()))
                .finish()
        } else {
            f.debug_struct("ModuleRoot").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<ModuleRoot> for SyntaxNode {
    fn from(n: ModuleRoot) -> Self {
        n.syntax
    }
}
impl From<ModuleRoot> for SyntaxElement {
    fn from(n: ModuleRoot) -> Self {
        n.syntax.into()
    }
}
impl AstNode for Name {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> = SyntaxKindSet::from_raw(RawSyntaxKind(NAME as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == NAME
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(Self { syntax })
        } else {
            None
        }
    }
    fn syntax(&self) -> &SyntaxNode {
        &self.syntax
    }
    fn into_syntax(self) -> SyntaxNode {
        self.syntax
    }
}
impl std::fmt::Debug for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("Name")
                .field(
                    "value_token",
                    &support::DebugSyntaxResult(self.value_token()),
                )
                .finish()
        } else {
            f.debug_struct("Name").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<Name> for SyntaxNode {
    fn from(n: Name) -> Self {
        n.syntax
    }
}
impl From<Name> for SyntaxElement {
    fn from(n: Name) -> Self {
        n.syntax.into()
    }
}
impl AstNode for Parameter {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(PARAMETER as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == PARAMETER
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(Self { syntax })
        } else {
            None
        }
    }
    fn syntax(&self) -> &SyntaxNode {
        &self.syntax
    }
    fn into_syntax(self) -> SyntaxNode {
        self.syntax
    }
}
impl std::fmt::Debug for Parameter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("Parameter")
                .field("pat", &support::DebugSyntaxResult(self.pat()))
                .field(
                    "type_annotation",
                    &support::DebugOptionalElement(self.type_annotation()),
                )
                .finish()
        } else {
            f.debug_struct("Parameter").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<Parameter> for SyntaxNode {
    fn from(n: Parameter) -> Self {
        n.syntax
    }
}
impl From<Parameter> for SyntaxElement {
    fn from(n: Parameter) -> Self {
        n.syntax.into()
    }
}
impl AstNode for Parameters {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(PARAMETERS as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == PARAMETERS
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(Self { syntax })
        } else {
            None
        }
    }
    fn syntax(&self) -> &SyntaxNode {
        &self.syntax
    }
    fn into_syntax(self) -> SyntaxNode {
        self.syntax
    }
}
impl std::fmt::Debug for Parameters {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("Parameters")
                .field(
                    "l_paren_token",
                    &support::DebugSyntaxResult(self.l_paren_token()),
                )
                .field("items", &self.items())
                .field(
                    "r_paren_token",
                    &support::DebugSyntaxResult(self.r_paren_token()),
                )
                .finish()
        } else {
            f.debug_struct("Parameters").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<Parameters> for SyntaxNode {
    fn from(n: Parameters) -> Self {
        n.syntax
    }
}
impl From<Parameters> for SyntaxElement {
    fn from(n: Parameters) -> Self {
        n.syntax.into()
    }
}
impl AstNode for ParenExpr {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(PAREN_EXPR as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == PAREN_EXPR
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(Self { syntax })
        } else {
            None
        }
    }
    fn syntax(&self) -> &SyntaxNode {
        &self.syntax
    }
    fn into_syntax(self) -> SyntaxNode {
        self.syntax
    }
}
impl std::fmt::Debug for ParenExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("ParenExpr")
                .field(
                    "l_paren_token",
                    &support::DebugSyntaxResult(self.l_paren_token()),
                )
                .field("expr", &support::DebugSyntaxResult(self.expr()))
                .field(
                    "r_paren_token",
                    &support::DebugSyntaxResult(self.r_paren_token()),
                )
                .finish()
        } else {
            f.debug_struct("ParenExpr").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<ParenExpr> for SyntaxNode {
    fn from(n: ParenExpr) -> Self {
        n.syntax
    }
}
impl From<ParenExpr> for SyntaxElement {
    fn from(n: ParenExpr) -> Self {
        n.syntax.into()
    }
}
impl AstNode for Path {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> = SyntaxKindSet::from_raw(RawSyntaxKind(PATH as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == PATH
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(Self { syntax })
        } else {
            None
        }
    }
    fn syntax(&self) -> &SyntaxNode {
        &self.syntax
    }
    fn into_syntax(self) -> SyntaxNode {
        self.syntax
    }
}
impl std::fmt::Debug for Path {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("Path")
                .field(
                    "qualifier",
                    &support::DebugOptionalElement(self.qualifier()),
                )
                .field("segment", &support::DebugSyntaxResult(self.segment()))
                .finish()
        } else {
            f.debug_struct("Path").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<Path> for SyntaxNode {
    fn from(n: Path) -> Self {
        n.syntax
    }
}
impl From<Path> for SyntaxElement {
    fn from(n: Path) -> Self {
        n.syntax.into()
    }
}
impl AstNode for PathExpr {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(PATH_EXPR as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == PATH_EXPR
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(Self { syntax })
        } else {
            None
        }
    }
    fn syntax(&self) -> &SyntaxNode {
        &self.syntax
    }
    fn into_syntax(self) -> SyntaxNode {
        self.syntax
    }
}
impl std::fmt::Debug for PathExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("PathExpr")
                .field("path", &support::DebugSyntaxResult(self.path()))
                .finish()
        } else {
            f.debug_struct("PathExpr").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<PathExpr> for SyntaxNode {
    fn from(n: PathExpr) -> Self {
        n.syntax
    }
}
impl From<PathExpr> for SyntaxElement {
    fn from(n: PathExpr) -> Self {
        n.syntax.into()
    }
}
impl AstNode for PathQualifier {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(PATH_QUALIFIER as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == PATH_QUALIFIER
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(Self { syntax })
        } else {
            None
        }
    }
    fn syntax(&self) -> &SyntaxNode {
        &self.syntax
    }
    fn into_syntax(self) -> SyntaxNode {
        self.syntax
    }
}
impl std::fmt::Debug for PathQualifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("PathQualifier")
                .field("path", &support::DebugSyntaxResult(self.path()))
                .field("dot_token", &support::DebugSyntaxResult(self.dot_token()))
                .finish()
        } else {
            f.debug_struct("PathQualifier").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<PathQualifier> for SyntaxNode {
    fn from(n: PathQualifier) -> Self {
        n.syntax
    }
}
impl From<PathQualifier> for SyntaxElement {
    fn from(n: PathQualifier) -> Self {
        n.syntax.into()
    }
}
impl AstNode for PathSegment {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(PATH_SEGMENT as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == PATH_SEGMENT
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(Self { syntax })
        } else {
            None
        }
    }
    fn syntax(&self) -> &SyntaxNode {
        &self.syntax
    }
    fn into_syntax(self) -> SyntaxNode {
        self.syntax
    }
}
impl std::fmt::Debug for PathSegment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("PathSegment")
                .field("root", &support::DebugSyntaxResult(self.root()))
                .field(
                    "type_args",
                    &support::DebugOptionalElement(self.type_args()),
                )
                .finish()
        } else {
            f.debug_struct("PathSegment").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<PathSegment> for SyntaxNode {
    fn from(n: PathSegment) -> Self {
        n.syntax
    }
}
impl From<PathSegment> for SyntaxElement {
    fn from(n: PathSegment) -> Self {
        n.syntax.into()
    }
}
impl AstNode for PathType {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(PATH_TYPE as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == PATH_TYPE
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(Self { syntax })
        } else {
            None
        }
    }
    fn syntax(&self) -> &SyntaxNode {
        &self.syntax
    }
    fn into_syntax(self) -> SyntaxNode {
        self.syntax
    }
}
impl std::fmt::Debug for PathType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("PathType")
                .field("path", &support::DebugSyntaxResult(self.path()))
                .finish()
        } else {
            f.debug_struct("PathType").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<PathType> for SyntaxNode {
    fn from(n: PathType) -> Self {
        n.syntax
    }
}
impl From<PathType> for SyntaxElement {
    fn from(n: PathType) -> Self {
        n.syntax.into()
    }
}
impl AstNode for Project {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(PROJECT as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == PROJECT
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(Self { syntax })
        } else {
            None
        }
    }
    fn syntax(&self) -> &SyntaxNode {
        &self.syntax
    }
    fn into_syntax(self) -> SyntaxNode {
        self.syntax
    }
}
impl std::fmt::Debug for Project {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("Project")
                .field(
                    "project_token",
                    &support::DebugSyntaxResult(self.project_token()),
                )
                .finish()
        } else {
            f.debug_struct("Project").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<Project> for SyntaxNode {
    fn from(n: Project) -> Self {
        n.syntax
    }
}
impl From<Project> for SyntaxElement {
    fn from(n: Project) -> Self {
        n.syntax.into()
    }
}
impl AstNode for StringLiteral {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(STRING_LITERAL as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == STRING_LITERAL
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(Self { syntax })
        } else {
            None
        }
    }
    fn syntax(&self) -> &SyntaxNode {
        &self.syntax
    }
    fn into_syntax(self) -> SyntaxNode {
        self.syntax
    }
}
impl std::fmt::Debug for StringLiteral {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("StringLiteral")
                .field(
                    "value_token",
                    &support::DebugSyntaxResult(self.value_token()),
                )
                .finish()
        } else {
            f.debug_struct("StringLiteral").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<StringLiteral> for SyntaxNode {
    fn from(n: StringLiteral) -> Self {
        n.syntax
    }
}
impl From<StringLiteral> for SyntaxElement {
    fn from(n: StringLiteral) -> Self {
        n.syntax.into()
    }
}
impl AstNode for TypeAnnotation {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(TYPE_ANNOTATION as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == TYPE_ANNOTATION
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(Self { syntax })
        } else {
            None
        }
    }
    fn syntax(&self) -> &SyntaxNode {
        &self.syntax
    }
    fn into_syntax(self) -> SyntaxNode {
        self.syntax
    }
}
impl std::fmt::Debug for TypeAnnotation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("TypeAnnotation")
                .field(
                    "colon_token",
                    &support::DebugSyntaxResult(self.colon_token()),
                )
                .field("ty", &support::DebugSyntaxResult(self.ty()))
                .finish()
        } else {
            f.debug_struct("TypeAnnotation").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<TypeAnnotation> for SyntaxNode {
    fn from(n: TypeAnnotation) -> Self {
        n.syntax
    }
}
impl From<TypeAnnotation> for SyntaxElement {
    fn from(n: TypeAnnotation) -> Self {
        n.syntax.into()
    }
}
impl AstNode for TypeArgs {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(TYPE_ARGS as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == TYPE_ARGS
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(Self { syntax })
        } else {
            None
        }
    }
    fn syntax(&self) -> &SyntaxNode {
        &self.syntax
    }
    fn into_syntax(self) -> SyntaxNode {
        self.syntax
    }
}
impl std::fmt::Debug for TypeArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("TypeArgs")
                .field(
                    "l_brack_token",
                    &support::DebugSyntaxResult(self.l_brack_token()),
                )
                .field("type_arg_list", &self.type_arg_list())
                .field(
                    "r_brack_token",
                    &support::DebugSyntaxResult(self.r_brack_token()),
                )
                .finish()
        } else {
            f.debug_struct("TypeArgs").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<TypeArgs> for SyntaxNode {
    fn from(n: TypeArgs) -> Self {
        n.syntax
    }
}
impl From<TypeArgs> for SyntaxElement {
    fn from(n: TypeArgs) -> Self {
        n.syntax.into()
    }
}
impl AstNode for TypeDecl {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(TYPE_DECL as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == TYPE_DECL
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(Self { syntax })
        } else {
            None
        }
    }
    fn syntax(&self) -> &SyntaxNode {
        &self.syntax
    }
    fn into_syntax(self) -> SyntaxNode {
        self.syntax
    }
}
impl std::fmt::Debug for TypeDecl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("TypeDecl")
                .field("attributes", &self.attributes())
                .field(
                    "visibility_token",
                    &support::DebugOptionalElement(self.visibility_token()),
                )
                .field("type_token", &support::DebugSyntaxResult(self.type_token()))
                .field("name", &support::DebugSyntaxResult(self.name()))
                .finish()
        } else {
            f.debug_struct("TypeDecl").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<TypeDecl> for SyntaxNode {
    fn from(n: TypeDecl) -> Self {
        n.syntax
    }
}
impl From<TypeDecl> for SyntaxElement {
    fn from(n: TypeDecl) -> Self {
        n.syntax.into()
    }
}
impl AstNode for UnaryExpr {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(UNARY_EXPR as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == UNARY_EXPR
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(Self { syntax })
        } else {
            None
        }
    }
    fn syntax(&self) -> &SyntaxNode {
        &self.syntax
    }
    fn into_syntax(self) -> SyntaxNode {
        self.syntax
    }
}
impl std::fmt::Debug for UnaryExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("UnaryExpr")
                .field(
                    "operator_token",
                    &support::DebugSyntaxResult(self.operator_token()),
                )
                .field("operand", &support::DebugSyntaxResult(self.operand()))
                .finish()
        } else {
            f.debug_struct("UnaryExpr").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<UnaryExpr> for SyntaxNode {
    fn from(n: UnaryExpr) -> Self {
        n.syntax
    }
}
impl From<UnaryExpr> for SyntaxElement {
    fn from(n: UnaryExpr) -> Self {
        n.syntax.into()
    }
}
impl AstNode for UseAlias {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(USE_ALIAS as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == USE_ALIAS
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(Self { syntax })
        } else {
            None
        }
    }
    fn syntax(&self) -> &SyntaxNode {
        &self.syntax
    }
    fn into_syntax(self) -> SyntaxNode {
        self.syntax
    }
}
impl std::fmt::Debug for UseAlias {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("UseAlias")
                .field("as_token", &support::DebugSyntaxResult(self.as_token()))
                .field("name", &support::DebugSyntaxResult(self.name()))
                .finish()
        } else {
            f.debug_struct("UseAlias").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<UseAlias> for SyntaxNode {
    fn from(n: UseAlias) -> Self {
        n.syntax
    }
}
impl From<UseAlias> for SyntaxElement {
    fn from(n: UseAlias) -> Self {
        n.syntax.into()
    }
}
impl AstNode for UseDecl {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(USE_DECL as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == USE_DECL
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(Self { syntax })
        } else {
            None
        }
    }
    fn syntax(&self) -> &SyntaxNode {
        &self.syntax
    }
    fn into_syntax(self) -> SyntaxNode {
        self.syntax
    }
}
impl std::fmt::Debug for UseDecl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("UseDecl")
                .field(
                    "visibility_token",
                    &support::DebugOptionalElement(self.visibility_token()),
                )
                .field("use_token", &support::DebugSyntaxResult(self.use_token()))
                .field("path", &support::DebugSyntaxResult(self.path()))
                .field("alias", &support::DebugOptionalElement(self.alias()))
                .finish()
        } else {
            f.debug_struct("UseDecl").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<UseDecl> for SyntaxNode {
    fn from(n: UseDecl) -> Self {
        n.syntax
    }
}
impl From<UseDecl> for SyntaxElement {
    fn from(n: UseDecl) -> Self {
        n.syntax.into()
    }
}
impl AstNode for WildcardPat {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(WILDCARD_PAT as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == WILDCARD_PAT
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(Self { syntax })
        } else {
            None
        }
    }
    fn syntax(&self) -> &SyntaxNode {
        &self.syntax
    }
    fn into_syntax(self) -> SyntaxNode {
        self.syntax
    }
}
impl std::fmt::Debug for WildcardPat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("WildcardPat")
                .field(
                    "underscore_token",
                    &support::DebugSyntaxResult(self.underscore_token()),
                )
                .finish()
        } else {
            f.debug_struct("WildcardPat").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<WildcardPat> for SyntaxNode {
    fn from(n: WildcardPat) -> Self {
        n.syntax
    }
}
impl From<WildcardPat> for SyntaxElement {
    fn from(n: WildcardPat) -> Self {
        n.syntax.into()
    }
}
impl From<BogusParameter> for AnyParameter {
    fn from(node: BogusParameter) -> Self {
        Self::BogusParameter(node)
    }
}
impl From<Parameter> for AnyParameter {
    fn from(node: Parameter) -> Self {
        Self::Parameter(node)
    }
}
impl AstNode for AnyParameter {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> = BogusParameter::KIND_SET.union(Parameter::KIND_SET);
    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(kind, BOGUS_PARAMETER | PARAMETER)
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        let res = match syntax.kind() {
            BOGUS_PARAMETER => Self::BogusParameter(BogusParameter { syntax }),
            PARAMETER => Self::Parameter(Parameter { syntax }),
            _ => return None,
        };
        Some(res)
    }
    fn syntax(&self) -> &SyntaxNode {
        match self {
            Self::BogusParameter(it) => it.syntax(),
            Self::Parameter(it) => it.syntax(),
        }
    }
    fn into_syntax(self) -> SyntaxNode {
        match self {
            Self::BogusParameter(it) => it.into_syntax(),
            Self::Parameter(it) => it.into_syntax(),
        }
    }
}
impl std::fmt::Debug for AnyParameter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BogusParameter(it) => std::fmt::Debug::fmt(it, f),
            Self::Parameter(it) => std::fmt::Debug::fmt(it, f),
        }
    }
}
impl From<AnyParameter> for SyntaxNode {
    fn from(n: AnyParameter) -> Self {
        match n {
            AnyParameter::BogusParameter(it) => it.into_syntax(),
            AnyParameter::Parameter(it) => it.into_syntax(),
        }
    }
}
impl From<AnyParameter> for SyntaxElement {
    fn from(n: AnyParameter) -> Self {
        let node: SyntaxNode = n.into();
        node.into()
    }
}
impl From<BinExpr> for Expr {
    fn from(node: BinExpr) -> Self {
        Self::BinExpr(node)
    }
}
impl From<BogusExpr> for Expr {
    fn from(node: BogusExpr) -> Self {
        Self::BogusExpr(node)
    }
}
impl From<CallExpr> for Expr {
    fn from(node: CallExpr) -> Self {
        Self::CallExpr(node)
    }
}
impl From<LetExpr> for Expr {
    fn from(node: LetExpr) -> Self {
        Self::LetExpr(node)
    }
}
impl From<ParenExpr> for Expr {
    fn from(node: ParenExpr) -> Self {
        Self::ParenExpr(node)
    }
}
impl From<PathExpr> for Expr {
    fn from(node: PathExpr) -> Self {
        Self::PathExpr(node)
    }
}
impl From<UnaryExpr> for Expr {
    fn from(node: UnaryExpr) -> Self {
        Self::UnaryExpr(node)
    }
}
impl AstNode for Expr {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> = BinExpr::KIND_SET
        .union(BogusExpr::KIND_SET)
        .union(CallExpr::KIND_SET)
        .union(LetExpr::KIND_SET)
        .union(Literal::KIND_SET)
        .union(ParenExpr::KIND_SET)
        .union(PathExpr::KIND_SET)
        .union(UnaryExpr::KIND_SET);
    fn can_cast(kind: SyntaxKind) -> bool {
        match kind {
            BIN_EXPR | BOGUS_EXPR | CALL_EXPR | LET_EXPR | PAREN_EXPR | PATH_EXPR | UNARY_EXPR => {
                true
            },
            k if Literal::can_cast(k) => true,
            _ => false,
        }
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        let res = match syntax.kind() {
            BIN_EXPR => Self::BinExpr(BinExpr { syntax }),
            BOGUS_EXPR => Self::BogusExpr(BogusExpr { syntax }),
            CALL_EXPR => Self::CallExpr(CallExpr { syntax }),
            LET_EXPR => Self::LetExpr(LetExpr { syntax }),
            PAREN_EXPR => Self::ParenExpr(ParenExpr { syntax }),
            PATH_EXPR => Self::PathExpr(PathExpr { syntax }),
            UNARY_EXPR => Self::UnaryExpr(UnaryExpr { syntax }),
            _ => {
                if let Some(literal) = Literal::cast(syntax) {
                    return Some(Self::Literal(literal));
                }
                return None;
            },
        };
        Some(res)
    }
    fn syntax(&self) -> &SyntaxNode {
        match self {
            Self::BinExpr(it) => it.syntax(),
            Self::BogusExpr(it) => it.syntax(),
            Self::CallExpr(it) => it.syntax(),
            Self::LetExpr(it) => it.syntax(),
            Self::ParenExpr(it) => it.syntax(),
            Self::PathExpr(it) => it.syntax(),
            Self::UnaryExpr(it) => it.syntax(),
            Self::Literal(it) => it.syntax(),
        }
    }
    fn into_syntax(self) -> SyntaxNode {
        match self {
            Self::BinExpr(it) => it.into_syntax(),
            Self::BogusExpr(it) => it.into_syntax(),
            Self::CallExpr(it) => it.into_syntax(),
            Self::LetExpr(it) => it.into_syntax(),
            Self::ParenExpr(it) => it.into_syntax(),
            Self::PathExpr(it) => it.into_syntax(),
            Self::UnaryExpr(it) => it.into_syntax(),
            Self::Literal(it) => it.into_syntax(),
        }
    }
}
impl std::fmt::Debug for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BinExpr(it) => std::fmt::Debug::fmt(it, f),
            Self::BogusExpr(it) => std::fmt::Debug::fmt(it, f),
            Self::CallExpr(it) => std::fmt::Debug::fmt(it, f),
            Self::LetExpr(it) => std::fmt::Debug::fmt(it, f),
            Self::Literal(it) => std::fmt::Debug::fmt(it, f),
            Self::ParenExpr(it) => std::fmt::Debug::fmt(it, f),
            Self::PathExpr(it) => std::fmt::Debug::fmt(it, f),
            Self::UnaryExpr(it) => std::fmt::Debug::fmt(it, f),
        }
    }
}
impl From<Expr> for SyntaxNode {
    fn from(n: Expr) -> Self {
        match n {
            Expr::BinExpr(it) => it.into_syntax(),
            Expr::BogusExpr(it) => it.into_syntax(),
            Expr::CallExpr(it) => it.into_syntax(),
            Expr::LetExpr(it) => it.into_syntax(),
            Expr::Literal(it) => it.into_syntax(),
            Expr::ParenExpr(it) => it.into_syntax(),
            Expr::PathExpr(it) => it.into_syntax(),
            Expr::UnaryExpr(it) => it.into_syntax(),
        }
    }
}
impl From<Expr> for SyntaxElement {
    fn from(n: Expr) -> Self {
        let node: SyntaxNode = n.into();
        node.into()
    }
}
impl From<IntLiteral> for Literal {
    fn from(node: IntLiteral) -> Self {
        Self::IntLiteral(node)
    }
}
impl From<StringLiteral> for Literal {
    fn from(node: StringLiteral) -> Self {
        Self::StringLiteral(node)
    }
}
impl AstNode for Literal {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> = IntLiteral::KIND_SET.union(StringLiteral::KIND_SET);
    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(kind, INT_LITERAL | STRING_LITERAL)
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        let res = match syntax.kind() {
            INT_LITERAL => Self::IntLiteral(IntLiteral { syntax }),
            STRING_LITERAL => Self::StringLiteral(StringLiteral { syntax }),
            _ => return None,
        };
        Some(res)
    }
    fn syntax(&self) -> &SyntaxNode {
        match self {
            Self::IntLiteral(it) => it.syntax(),
            Self::StringLiteral(it) => it.syntax(),
        }
    }
    fn into_syntax(self) -> SyntaxNode {
        match self {
            Self::IntLiteral(it) => it.into_syntax(),
            Self::StringLiteral(it) => it.into_syntax(),
        }
    }
}
impl std::fmt::Debug for Literal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IntLiteral(it) => std::fmt::Debug::fmt(it, f),
            Self::StringLiteral(it) => std::fmt::Debug::fmt(it, f),
        }
    }
}
impl From<Literal> for SyntaxNode {
    fn from(n: Literal) -> Self {
        match n {
            Literal::IntLiteral(it) => it.into_syntax(),
            Literal::StringLiteral(it) => it.into_syntax(),
        }
    }
}
impl From<Literal> for SyntaxElement {
    fn from(n: Literal) -> Self {
        let node: SyntaxNode = n.into();
        node.into()
    }
}
impl From<BogusDecl> for ModuleItem {
    fn from(node: BogusDecl) -> Self {
        Self::BogusDecl(node)
    }
}
impl From<FunDecl> for ModuleItem {
    fn from(node: FunDecl) -> Self {
        Self::FunDecl(node)
    }
}
impl From<TypeDecl> for ModuleItem {
    fn from(node: TypeDecl) -> Self {
        Self::TypeDecl(node)
    }
}
impl From<UseDecl> for ModuleItem {
    fn from(node: UseDecl) -> Self {
        Self::UseDecl(node)
    }
}
impl AstNode for ModuleItem {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> = BogusDecl::KIND_SET
        .union(FunDecl::KIND_SET)
        .union(TypeDecl::KIND_SET)
        .union(UseDecl::KIND_SET);
    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(kind, BOGUS_DECL | FUN_DECL | TYPE_DECL | USE_DECL)
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        let res = match syntax.kind() {
            BOGUS_DECL => Self::BogusDecl(BogusDecl { syntax }),
            FUN_DECL => Self::FunDecl(FunDecl { syntax }),
            TYPE_DECL => Self::TypeDecl(TypeDecl { syntax }),
            USE_DECL => Self::UseDecl(UseDecl { syntax }),
            _ => return None,
        };
        Some(res)
    }
    fn syntax(&self) -> &SyntaxNode {
        match self {
            Self::BogusDecl(it) => it.syntax(),
            Self::FunDecl(it) => it.syntax(),
            Self::TypeDecl(it) => it.syntax(),
            Self::UseDecl(it) => it.syntax(),
        }
    }
    fn into_syntax(self) -> SyntaxNode {
        match self {
            Self::BogusDecl(it) => it.into_syntax(),
            Self::FunDecl(it) => it.into_syntax(),
            Self::TypeDecl(it) => it.into_syntax(),
            Self::UseDecl(it) => it.into_syntax(),
        }
    }
}
impl std::fmt::Debug for ModuleItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BogusDecl(it) => std::fmt::Debug::fmt(it, f),
            Self::FunDecl(it) => std::fmt::Debug::fmt(it, f),
            Self::TypeDecl(it) => std::fmt::Debug::fmt(it, f),
            Self::UseDecl(it) => std::fmt::Debug::fmt(it, f),
        }
    }
}
impl From<ModuleItem> for SyntaxNode {
    fn from(n: ModuleItem) -> Self {
        match n {
            ModuleItem::BogusDecl(it) => it.into_syntax(),
            ModuleItem::FunDecl(it) => it.into_syntax(),
            ModuleItem::TypeDecl(it) => it.into_syntax(),
            ModuleItem::UseDecl(it) => it.into_syntax(),
        }
    }
}
impl From<ModuleItem> for SyntaxElement {
    fn from(n: ModuleItem) -> Self {
        let node: SyntaxNode = n.into();
        node.into()
    }
}
impl From<BogusPat> for Pat {
    fn from(node: BogusPat) -> Self {
        Self::BogusPat(node)
    }
}
impl From<IdentPat> for Pat {
    fn from(node: IdentPat) -> Self {
        Self::IdentPat(node)
    }
}
impl From<WildcardPat> for Pat {
    fn from(node: WildcardPat) -> Self {
        Self::WildcardPat(node)
    }
}
impl AstNode for Pat {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> = BogusPat::KIND_SET
        .union(IdentPat::KIND_SET)
        .union(WildcardPat::KIND_SET);
    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(kind, BOGUS_PAT | IDENT_PAT | WILDCARD_PAT)
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        let res = match syntax.kind() {
            BOGUS_PAT => Self::BogusPat(BogusPat { syntax }),
            IDENT_PAT => Self::IdentPat(IdentPat { syntax }),
            WILDCARD_PAT => Self::WildcardPat(WildcardPat { syntax }),
            _ => return None,
        };
        Some(res)
    }
    fn syntax(&self) -> &SyntaxNode {
        match self {
            Self::BogusPat(it) => it.syntax(),
            Self::IdentPat(it) => it.syntax(),
            Self::WildcardPat(it) => it.syntax(),
        }
    }
    fn into_syntax(self) -> SyntaxNode {
        match self {
            Self::BogusPat(it) => it.into_syntax(),
            Self::IdentPat(it) => it.into_syntax(),
            Self::WildcardPat(it) => it.into_syntax(),
        }
    }
}
impl std::fmt::Debug for Pat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BogusPat(it) => std::fmt::Debug::fmt(it, f),
            Self::IdentPat(it) => std::fmt::Debug::fmt(it, f),
            Self::WildcardPat(it) => std::fmt::Debug::fmt(it, f),
        }
    }
}
impl From<Pat> for SyntaxNode {
    fn from(n: Pat) -> Self {
        match n {
            Pat::BogusPat(it) => it.into_syntax(),
            Pat::IdentPat(it) => it.into_syntax(),
            Pat::WildcardPat(it) => it.into_syntax(),
        }
    }
}
impl From<Pat> for SyntaxElement {
    fn from(n: Pat) -> Self {
        let node: SyntaxNode = n.into();
        node.into()
    }
}
impl From<Name> for PathRoot {
    fn from(node: Name) -> Self {
        Self::Name(node)
    }
}
impl From<Project> for PathRoot {
    fn from(node: Project) -> Self {
        Self::Project(node)
    }
}
impl AstNode for PathRoot {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> = Name::KIND_SET.union(Project::KIND_SET);
    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(kind, NAME | PROJECT)
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        let res = match syntax.kind() {
            NAME => Self::Name(Name { syntax }),
            PROJECT => Self::Project(Project { syntax }),
            _ => return None,
        };
        Some(res)
    }
    fn syntax(&self) -> &SyntaxNode {
        match self {
            Self::Name(it) => it.syntax(),
            Self::Project(it) => it.syntax(),
        }
    }
    fn into_syntax(self) -> SyntaxNode {
        match self {
            Self::Name(it) => it.into_syntax(),
            Self::Project(it) => it.into_syntax(),
        }
    }
}
impl std::fmt::Debug for PathRoot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Name(it) => std::fmt::Debug::fmt(it, f),
            Self::Project(it) => std::fmt::Debug::fmt(it, f),
        }
    }
}
impl From<PathRoot> for SyntaxNode {
    fn from(n: PathRoot) -> Self {
        match n {
            PathRoot::Name(it) => it.into_syntax(),
            PathRoot::Project(it) => it.into_syntax(),
        }
    }
}
impl From<PathRoot> for SyntaxElement {
    fn from(n: PathRoot) -> Self {
        let node: SyntaxNode = n.into();
        node.into()
    }
}
impl From<BogusType> for Type {
    fn from(node: BogusType) -> Self {
        Self::BogusType(node)
    }
}
impl From<InferType> for Type {
    fn from(node: InferType) -> Self {
        Self::InferType(node)
    }
}
impl From<PathType> for Type {
    fn from(node: PathType) -> Self {
        Self::PathType(node)
    }
}
impl AstNode for Type {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> = BogusType::KIND_SET
        .union(InferType::KIND_SET)
        .union(PathType::KIND_SET);
    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(kind, BOGUS_TYPE | INFER_TYPE | PATH_TYPE)
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        let res = match syntax.kind() {
            BOGUS_TYPE => Self::BogusType(BogusType { syntax }),
            INFER_TYPE => Self::InferType(InferType { syntax }),
            PATH_TYPE => Self::PathType(PathType { syntax }),
            _ => return None,
        };
        Some(res)
    }
    fn syntax(&self) -> &SyntaxNode {
        match self {
            Self::BogusType(it) => it.syntax(),
            Self::InferType(it) => it.syntax(),
            Self::PathType(it) => it.syntax(),
        }
    }
    fn into_syntax(self) -> SyntaxNode {
        match self {
            Self::BogusType(it) => it.into_syntax(),
            Self::InferType(it) => it.into_syntax(),
            Self::PathType(it) => it.into_syntax(),
        }
    }
}
impl std::fmt::Debug for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BogusType(it) => std::fmt::Debug::fmt(it, f),
            Self::InferType(it) => std::fmt::Debug::fmt(it, f),
            Self::PathType(it) => std::fmt::Debug::fmt(it, f),
        }
    }
}
impl From<Type> for SyntaxNode {
    fn from(n: Type) -> Self {
        match n {
            Type::BogusType(it) => it.into_syntax(),
            Type::InferType(it) => it.into_syntax(),
            Type::PathType(it) => it.into_syntax(),
        }
    }
}
impl From<Type> for SyntaxElement {
    fn from(n: Type) -> Self {
        let node: SyntaxNode = n.into();
        node.into()
    }
}
impl std::fmt::Display for AnyParameter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for Literal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for ModuleItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for Pat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for PathRoot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for Attribute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for BinExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for CallExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for FunBody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for FunDecl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for FunReturnTypeAnnotation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for IdentPat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for InferType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for IntLiteral {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for LetExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for ModulePreamble {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for ModuleRoot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for Parameter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for Parameters {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for ParenExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for Path {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for PathExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for PathQualifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for PathSegment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for PathType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for Project {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for StringLiteral {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for TypeAnnotation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for TypeArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for TypeDecl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for UnaryExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for UseAlias {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for UseDecl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for WildcardPat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Bogus {
    syntax: SyntaxNode,
}
impl Serialize for Bogus {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(Some(2))?;
        state.serialize_entry("kind", "Bogus")?;
        state.serialize_entry("items", &self.items().collect::<Vec<_>>())?;
        state.end()
    }
}
impl Bogus {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn items(&self) -> SyntaxElementChildren {
        support::elements(&self.syntax)
    }
}
impl AstNode for Bogus {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> = SyntaxKindSet::from_raw(RawSyntaxKind(BOGUS as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == BOGUS
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(Self { syntax })
        } else {
            None
        }
    }
    fn syntax(&self) -> &SyntaxNode {
        &self.syntax
    }
    fn into_syntax(self) -> SyntaxNode {
        self.syntax
    }
}
impl std::fmt::Debug for Bogus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Bogus")
            .field("items", &DebugSyntaxElementChildren(self.items()))
            .finish()
    }
}
impl From<Bogus> for SyntaxNode {
    fn from(n: Bogus) -> Self {
        n.syntax
    }
}
impl From<Bogus> for SyntaxElement {
    fn from(n: Bogus) -> Self {
        n.syntax.into()
    }
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct BogusDecl {
    syntax: SyntaxNode,
}
impl Serialize for BogusDecl {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(Some(2))?;
        state.serialize_entry("kind", "BogusDecl")?;
        state.serialize_entry("items", &self.items().collect::<Vec<_>>())?;
        state.end()
    }
}
impl BogusDecl {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn items(&self) -> SyntaxElementChildren {
        support::elements(&self.syntax)
    }
}
impl AstNode for BogusDecl {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(BOGUS_DECL as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == BOGUS_DECL
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(Self { syntax })
        } else {
            None
        }
    }
    fn syntax(&self) -> &SyntaxNode {
        &self.syntax
    }
    fn into_syntax(self) -> SyntaxNode {
        self.syntax
    }
}
impl std::fmt::Debug for BogusDecl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BogusDecl")
            .field("items", &DebugSyntaxElementChildren(self.items()))
            .finish()
    }
}
impl From<BogusDecl> for SyntaxNode {
    fn from(n: BogusDecl) -> Self {
        n.syntax
    }
}
impl From<BogusDecl> for SyntaxElement {
    fn from(n: BogusDecl) -> Self {
        n.syntax.into()
    }
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct BogusExpr {
    syntax: SyntaxNode,
}
impl Serialize for BogusExpr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(Some(2))?;
        state.serialize_entry("kind", "BogusExpr")?;
        state.serialize_entry("items", &self.items().collect::<Vec<_>>())?;
        state.end()
    }
}
impl BogusExpr {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn items(&self) -> SyntaxElementChildren {
        support::elements(&self.syntax)
    }
}
impl AstNode for BogusExpr {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(BOGUS_EXPR as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == BOGUS_EXPR
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(Self { syntax })
        } else {
            None
        }
    }
    fn syntax(&self) -> &SyntaxNode {
        &self.syntax
    }
    fn into_syntax(self) -> SyntaxNode {
        self.syntax
    }
}
impl std::fmt::Debug for BogusExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BogusExpr")
            .field("items", &DebugSyntaxElementChildren(self.items()))
            .finish()
    }
}
impl From<BogusExpr> for SyntaxNode {
    fn from(n: BogusExpr) -> Self {
        n.syntax
    }
}
impl From<BogusExpr> for SyntaxElement {
    fn from(n: BogusExpr) -> Self {
        n.syntax.into()
    }
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct BogusParameter {
    syntax: SyntaxNode,
}
impl Serialize for BogusParameter {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(Some(2))?;
        state.serialize_entry("kind", "BogusParameter")?;
        state.serialize_entry("items", &self.items().collect::<Vec<_>>())?;
        state.end()
    }
}
impl BogusParameter {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn items(&self) -> SyntaxElementChildren {
        support::elements(&self.syntax)
    }
}
impl AstNode for BogusParameter {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(BOGUS_PARAMETER as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == BOGUS_PARAMETER
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(Self { syntax })
        } else {
            None
        }
    }
    fn syntax(&self) -> &SyntaxNode {
        &self.syntax
    }
    fn into_syntax(self) -> SyntaxNode {
        self.syntax
    }
}
impl std::fmt::Debug for BogusParameter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BogusParameter")
            .field("items", &DebugSyntaxElementChildren(self.items()))
            .finish()
    }
}
impl From<BogusParameter> for SyntaxNode {
    fn from(n: BogusParameter) -> Self {
        n.syntax
    }
}
impl From<BogusParameter> for SyntaxElement {
    fn from(n: BogusParameter) -> Self {
        n.syntax.into()
    }
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct BogusPat {
    syntax: SyntaxNode,
}
impl Serialize for BogusPat {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(Some(2))?;
        state.serialize_entry("kind", "BogusPat")?;
        state.serialize_entry("items", &self.items().collect::<Vec<_>>())?;
        state.end()
    }
}
impl BogusPat {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn items(&self) -> SyntaxElementChildren {
        support::elements(&self.syntax)
    }
}
impl AstNode for BogusPat {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(BOGUS_PAT as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == BOGUS_PAT
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(Self { syntax })
        } else {
            None
        }
    }
    fn syntax(&self) -> &SyntaxNode {
        &self.syntax
    }
    fn into_syntax(self) -> SyntaxNode {
        self.syntax
    }
}
impl std::fmt::Debug for BogusPat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BogusPat")
            .field("items", &DebugSyntaxElementChildren(self.items()))
            .finish()
    }
}
impl From<BogusPat> for SyntaxNode {
    fn from(n: BogusPat) -> Self {
        n.syntax
    }
}
impl From<BogusPat> for SyntaxElement {
    fn from(n: BogusPat) -> Self {
        n.syntax.into()
    }
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct BogusType {
    syntax: SyntaxNode,
}
impl Serialize for BogusType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(Some(2))?;
        state.serialize_entry("kind", "BogusType")?;
        state.serialize_entry("items", &self.items().collect::<Vec<_>>())?;
        state.end()
    }
}
impl BogusType {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn items(&self) -> SyntaxElementChildren {
        support::elements(&self.syntax)
    }
}
impl AstNode for BogusType {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(BOGUS_TYPE as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == BOGUS_TYPE
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(Self { syntax })
        } else {
            None
        }
    }
    fn syntax(&self) -> &SyntaxNode {
        &self.syntax
    }
    fn into_syntax(self) -> SyntaxNode {
        self.syntax
    }
}
impl std::fmt::Debug for BogusType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BogusType")
            .field("items", &DebugSyntaxElementChildren(self.items()))
            .finish()
    }
}
impl From<BogusType> for SyntaxNode {
    fn from(n: BogusType) -> Self {
        n.syntax
    }
}
impl From<BogusType> for SyntaxElement {
    fn from(n: BogusType) -> Self {
        n.syntax.into()
    }
}
mlkc_rowan::declare_node_union! { pub AnyBogusNode = Bogus | BogusDecl | BogusExpr | BogusParameter | BogusPat | BogusType }
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct ArgumentList {
    syntax_list: SyntaxList,
}
impl ArgumentList {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self {
            syntax_list: syntax.into_list(),
        }
    }
}
impl AstNode for ArgumentList {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(ARGUMENT_LIST as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == ARGUMENT_LIST
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(Self {
                syntax_list: syntax.into_list(),
            })
        } else {
            None
        }
    }
    fn syntax(&self) -> &SyntaxNode {
        self.syntax_list.node()
    }
    fn into_syntax(self) -> SyntaxNode {
        self.syntax_list.into_node()
    }
}
impl Serialize for ArgumentList {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(Some(2))?;
        state.serialize_entry("kind", "ArgumentList")?;
        state.serialize_entry("items", &self.iter().collect::<Vec<_>>())?;
        state.end()
    }
}
impl AstSeparatedList for ArgumentList {
    type Language = Language;
    type Node = Expr;
    fn syntax_list(&self) -> &SyntaxList {
        &self.syntax_list
    }
    fn into_syntax_list(self) -> SyntaxList {
        self.syntax_list
    }
}
impl Debug for ArgumentList {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("ArgumentList ")?;
        f.debug_list().entries(self.elements()).finish()
    }
}
impl IntoIterator for ArgumentList {
    type Item = SyntaxResult<Expr>;
    type IntoIter = AstSeparatedListNodesIterator<Language, Expr>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl IntoIterator for &ArgumentList {
    type Item = SyntaxResult<Expr>;
    type IntoIter = AstSeparatedListNodesIterator<Language, Expr>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct AttributeList {
    syntax_list: SyntaxList,
}
impl AttributeList {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self {
            syntax_list: syntax.into_list(),
        }
    }
}
impl AstNode for AttributeList {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(ATTRIBUTE_LIST as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == ATTRIBUTE_LIST
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(Self {
                syntax_list: syntax.into_list(),
            })
        } else {
            None
        }
    }
    fn syntax(&self) -> &SyntaxNode {
        self.syntax_list.node()
    }
    fn into_syntax(self) -> SyntaxNode {
        self.syntax_list.into_node()
    }
}
impl Serialize for AttributeList {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(Some(2))?;
        state.serialize_entry("kind", "AttributeList")?;
        state.serialize_entry("items", &self.iter().collect::<Vec<_>>())?;
        state.end()
    }
}
impl AstNodeList for AttributeList {
    type Language = Language;
    type Node = Attribute;
    fn syntax_list(&self) -> &SyntaxList {
        &self.syntax_list
    }
    fn into_syntax_list(self) -> SyntaxList {
        self.syntax_list
    }
}
impl Debug for AttributeList {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("AttributeList ")?;
        f.debug_list().entries(self.iter()).finish()
    }
}
impl IntoIterator for &AttributeList {
    type Item = Attribute;
    type IntoIter = AstNodeListIterator<Language, Attribute>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl IntoIterator for AttributeList {
    type Item = Attribute;
    type IntoIter = AstNodeListIterator<Language, Attribute>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct ModuleItemList {
    syntax_list: SyntaxList,
}
impl ModuleItemList {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self {
            syntax_list: syntax.into_list(),
        }
    }
}
impl AstNode for ModuleItemList {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(MODULE_ITEM_LIST as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == MODULE_ITEM_LIST
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(Self {
                syntax_list: syntax.into_list(),
            })
        } else {
            None
        }
    }
    fn syntax(&self) -> &SyntaxNode {
        self.syntax_list.node()
    }
    fn into_syntax(self) -> SyntaxNode {
        self.syntax_list.into_node()
    }
}
impl Serialize for ModuleItemList {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(Some(2))?;
        state.serialize_entry("kind", "ModuleItemList")?;
        state.serialize_entry("items", &self.iter().collect::<Vec<_>>())?;
        state.end()
    }
}
impl AstNodeList for ModuleItemList {
    type Language = Language;
    type Node = ModuleItem;
    fn syntax_list(&self) -> &SyntaxList {
        &self.syntax_list
    }
    fn into_syntax_list(self) -> SyntaxList {
        self.syntax_list
    }
}
impl Debug for ModuleItemList {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("ModuleItemList ")?;
        f.debug_list().entries(self.iter()).finish()
    }
}
impl IntoIterator for &ModuleItemList {
    type Item = ModuleItem;
    type IntoIter = AstNodeListIterator<Language, ModuleItem>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl IntoIterator for ModuleItemList {
    type Item = ModuleItem;
    type IntoIter = AstNodeListIterator<Language, ModuleItem>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct ParameterList {
    syntax_list: SyntaxList,
}
impl ParameterList {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self {
            syntax_list: syntax.into_list(),
        }
    }
}
impl AstNode for ParameterList {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(PARAMETER_LIST as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == PARAMETER_LIST
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(Self {
                syntax_list: syntax.into_list(),
            })
        } else {
            None
        }
    }
    fn syntax(&self) -> &SyntaxNode {
        self.syntax_list.node()
    }
    fn into_syntax(self) -> SyntaxNode {
        self.syntax_list.into_node()
    }
}
impl Serialize for ParameterList {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(Some(2))?;
        state.serialize_entry("kind", "ParameterList")?;
        state.serialize_entry("items", &self.iter().collect::<Vec<_>>())?;
        state.end()
    }
}
impl AstSeparatedList for ParameterList {
    type Language = Language;
    type Node = AnyParameter;
    fn syntax_list(&self) -> &SyntaxList {
        &self.syntax_list
    }
    fn into_syntax_list(self) -> SyntaxList {
        self.syntax_list
    }
}
impl Debug for ParameterList {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("ParameterList ")?;
        f.debug_list().entries(self.elements()).finish()
    }
}
impl IntoIterator for ParameterList {
    type Item = SyntaxResult<AnyParameter>;
    type IntoIter = AstSeparatedListNodesIterator<Language, AnyParameter>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl IntoIterator for &ParameterList {
    type Item = SyntaxResult<AnyParameter>;
    type IntoIter = AstSeparatedListNodesIterator<Language, AnyParameter>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct TypeArgList {
    syntax_list: SyntaxList,
}
impl TypeArgList {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self {
            syntax_list: syntax.into_list(),
        }
    }
}
impl AstNode for TypeArgList {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(TYPE_ARG_LIST as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == TYPE_ARG_LIST
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(Self {
                syntax_list: syntax.into_list(),
            })
        } else {
            None
        }
    }
    fn syntax(&self) -> &SyntaxNode {
        self.syntax_list.node()
    }
    fn into_syntax(self) -> SyntaxNode {
        self.syntax_list.into_node()
    }
}
impl Serialize for TypeArgList {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(Some(2))?;
        state.serialize_entry("kind", "TypeArgList")?;
        state.serialize_entry("items", &self.iter().collect::<Vec<_>>())?;
        state.end()
    }
}
impl AstSeparatedList for TypeArgList {
    type Language = Language;
    type Node = Type;
    fn syntax_list(&self) -> &SyntaxList {
        &self.syntax_list
    }
    fn into_syntax_list(self) -> SyntaxList {
        self.syntax_list
    }
}
impl Debug for TypeArgList {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("TypeArgList ")?;
        f.debug_list().entries(self.elements()).finish()
    }
}
impl IntoIterator for TypeArgList {
    type Item = SyntaxResult<Type>;
    type IntoIter = AstSeparatedListNodesIterator<Language, Type>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl IntoIterator for &TypeArgList {
    type Item = SyntaxResult<Type>;
    type IntoIter = AstSeparatedListNodesIterator<Language, Type>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
#[derive(Clone)]
pub struct DebugSyntaxElementChildren(pub SyntaxElementChildren);
impl Debug for DebugSyntaxElementChildren {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_list()
            .entries(self.clone().0.map(DebugSyntaxElement))
            .finish()
    }
}
struct DebugSyntaxElement(SyntaxElement);
impl Debug for DebugSyntaxElement {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            SyntaxElement::Node(node) => {
                map_syntax_node ! (node . clone () , node => std :: fmt :: Debug :: fmt (& node , f))
            },
            SyntaxElement::Token(token) => Debug::fmt(token, f),
        }
    }
}
