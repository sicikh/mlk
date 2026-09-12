//! Generated file, do not edit by hand, see `xtask/codegen`

#![allow(dead_code)]
#![allow(unused)]
use std::fmt::{Debug, Formatter};

use biome_rowan::{
    AstNode, AstNodeList, AstNodeListIterator, AstNodeSlotMap, AstSeparatedList,
    AstSeparatedListNodesIterator, RawSyntaxKind, SyntaxKindSet, SyntaxResult, support,
};
use serde::{Serialize, Serializer, ser::SerializeSeq};

use crate::{
    MlkLanguage as Language, MlkSyntaxElement as SyntaxElement,
    MlkSyntaxElementChildren as SyntaxElementChildren,
    MlkSyntaxKind::{self as SyntaxKind, *},
    MlkSyntaxList as SyntaxList, MlkSyntaxNode as SyntaxNode, MlkSyntaxToken as SyntaxToken,
    macros::map_syntax_node,
};
#[doc = r" Sentinel value indicating a missing element in a dynamic node, where"]
#[doc = r" the slots are not statically known."]
pub(crate) const SLOT_MAP_EMPTY_VALUE: u8 = u8::MAX;
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct AndPat {
    pub(crate) syntax: SyntaxNode,
}
impl AndPat {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> AndPatFields {
        AndPatFields {
            Lhs: self.Lhs(),
            amp_token: self.amp_token(),
            Rhs: self.Rhs(),
        }
    }
    pub fn Lhs(&self) -> SyntaxResult<Pat> {
        support::required_node(&self.syntax, 0usize)
    }
    pub fn amp_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 1usize)
    }
    pub fn Rhs(&self) -> SyntaxResult<Pat> {
        support::required_node(&self.syntax, 2usize)
    }
}
impl Serialize for AndPat {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_fields().serialize(serializer)
    }
}
#[derive(Serialize)]
pub struct AndPatFields {
    pub Lhs: SyntaxResult<Pat>,
    pub amp_token: SyntaxResult<SyntaxToken>,
    pub Rhs: SyntaxResult<Pat>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct AppExpr {
    pub(crate) syntax: SyntaxNode,
}
impl AppExpr {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> AppExprFields {
        AppExprFields {
            Func: self.Func(),
            Arg: self.Arg(),
        }
    }
    pub fn Func(&self) -> SyntaxResult<Expr> {
        support::required_node(&self.syntax, 0usize)
    }
    pub fn Arg(&self) -> SyntaxResult<Expr> {
        support::required_node(&self.syntax, 1usize)
    }
}
impl Serialize for AppExpr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_fields().serialize(serializer)
    }
}
#[derive(Serialize)]
pub struct AppExprFields {
    pub Func: SyntaxResult<Expr>,
    pub Arg: SyntaxResult<Expr>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct AsPat {
    pub(crate) syntax: SyntaxNode,
}
impl AsPat {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> AsPatFields {
        AsPatFields {
            Lhs: self.Lhs(),
            as_token: self.as_token(),
            Rhs: self.Rhs(),
        }
    }
    pub fn Lhs(&self) -> SyntaxResult<Pat> {
        support::required_node(&self.syntax, 0usize)
    }
    pub fn as_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 1usize)
    }
    pub fn Rhs(&self) -> SyntaxResult<Pat> {
        support::required_node(&self.syntax, 2usize)
    }
}
impl Serialize for AsPat {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_fields().serialize(serializer)
    }
}
#[derive(Serialize)]
pub struct AsPatFields {
    pub Lhs: SyntaxResult<Pat>,
    pub as_token: SyntaxResult<SyntaxToken>,
    pub Rhs: SyntaxResult<Pat>,
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
            Left: self.Left(),
            Op_token: self.Op_token(),
            Right: self.Right(),
        }
    }
    pub fn Left(&self) -> SyntaxResult<Expr> {
        support::required_node(&self.syntax, 0usize)
    }
    pub fn Op_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 1usize)
    }
    pub fn Right(&self) -> SyntaxResult<Expr> {
        support::required_node(&self.syntax, 2usize)
    }
}
impl Serialize for BinExpr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_fields().serialize(serializer)
    }
}
#[derive(Serialize)]
pub struct BinExprFields {
    pub Left: SyntaxResult<Expr>,
    pub Op_token: SyntaxResult<SyntaxToken>,
    pub Right: SyntaxResult<Expr>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Binding {
    pub(crate) syntax: SyntaxNode,
}
impl Binding {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> BindingFields {
        BindingFields {
            Pat: self.Pat(),
            eq_token: self.eq_token(),
            Expr: self.Expr(),
        }
    }
    pub fn Pat(&self) -> SyntaxResult<Pat> {
        support::required_node(&self.syntax, 0usize)
    }
    pub fn eq_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 1usize)
    }
    pub fn Expr(&self) -> SyntaxResult<Expr> {
        support::required_node(&self.syntax, 2usize)
    }
}
impl Serialize for Binding {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_fields().serialize(serializer)
    }
}
#[derive(Serialize)]
pub struct BindingFields {
    pub Pat: SyntaxResult<Pat>,
    pub eq_token: SyntaxResult<SyntaxToken>,
    pub Expr: SyntaxResult<Expr>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct BoolLiteral {
    pub(crate) syntax: SyntaxNode,
}
impl BoolLiteral {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> BoolLiteralFields {
        BoolLiteralFields {
            ValueToken: self.ValueToken(),
        }
    }
    pub fn ValueToken(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 0usize)
    }
}
impl Serialize for BoolLiteral {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_fields().serialize(serializer)
    }
}
#[derive(Serialize)]
pub struct BoolLiteralFields {
    pub ValueToken: SyntaxResult<SyntaxToken>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct CharLiteral {
    pub(crate) syntax: SyntaxNode,
}
impl CharLiteral {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> CharLiteralFields {
        CharLiteralFields {
            Value_token: self.Value_token(),
        }
    }
    pub fn Value_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 0usize)
    }
}
impl Serialize for CharLiteral {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_fields().serialize(serializer)
    }
}
#[derive(Serialize)]
pub struct CharLiteralFields {
    pub Value_token: SyntaxResult<SyntaxToken>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ConsPat {
    pub(crate) syntax: SyntaxNode,
}
impl ConsPat {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> ConsPatFields {
        ConsPatFields {
            Head: self.Head(),
            double_colon_token: self.double_colon_token(),
            Tail: self.Tail(),
        }
    }
    pub fn Head(&self) -> SyntaxResult<Pat> {
        support::required_node(&self.syntax, 0usize)
    }
    pub fn double_colon_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 1usize)
    }
    pub fn Tail(&self) -> SyntaxResult<Pat> {
        support::required_node(&self.syntax, 2usize)
    }
}
impl Serialize for ConsPat {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_fields().serialize(serializer)
    }
}
#[derive(Serialize)]
pub struct ConsPatFields {
    pub Head: SyntaxResult<Pat>,
    pub double_colon_token: SyntaxResult<SyntaxToken>,
    pub Tail: SyntaxResult<Pat>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct FnTy {
    pub(crate) syntax: SyntaxNode,
}
impl FnTy {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> FnTyFields {
        FnTyFields {
            Arg: self.Arg(),
            arrow_token: self.arrow_token(),
            Ret: self.Ret(),
        }
    }
    pub fn Arg(&self) -> SyntaxResult<Ty> {
        support::required_node(&self.syntax, 0usize)
    }
    pub fn arrow_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 1usize)
    }
    pub fn Ret(&self) -> SyntaxResult<Ty> {
        support::required_node(&self.syntax, 2usize)
    }
}
impl Serialize for FnTy {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_fields().serialize(serializer)
    }
}
#[derive(Serialize)]
pub struct FnTyFields {
    pub Arg: SyntaxResult<Ty>,
    pub arrow_token: SyntaxResult<SyntaxToken>,
    pub Ret: SyntaxResult<Ty>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct FunExpr {
    pub(crate) syntax: SyntaxNode,
}
impl FunExpr {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> FunExprFields {
        FunExprFields {
            fun_token: self.fun_token(),
            Arg: self.Arg(),
            arrow_token: self.arrow_token(),
            Body: self.Body(),
        }
    }
    pub fn fun_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 0usize)
    }
    pub fn Arg(&self) -> SyntaxResult<Name> {
        support::required_node(&self.syntax, 1usize)
    }
    pub fn arrow_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 2usize)
    }
    pub fn Body(&self) -> SyntaxResult<Expr> {
        support::required_node(&self.syntax, 3usize)
    }
}
impl Serialize for FunExpr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_fields().serialize(serializer)
    }
}
#[derive(Serialize)]
pub struct FunExprFields {
    pub fun_token: SyntaxResult<SyntaxToken>,
    pub Arg: SyntaxResult<Name>,
    pub arrow_token: SyntaxResult<SyntaxToken>,
    pub Body: SyntaxResult<Expr>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct FuncPat {
    pub(crate) syntax: SyntaxNode,
}
impl FuncPat {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> FuncPatFields {
        FuncPatFields {
            Func: self.Func(),
            Args: self.Args(),
        }
    }
    pub fn Func(&self) -> SyntaxResult<Pat> {
        support::required_node(&self.syntax, 0usize)
    }
    pub fn Args(&self) -> ArgPats {
        support::list(&self.syntax, 1usize)
    }
}
impl Serialize for FuncPat {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_fields().serialize(serializer)
    }
}
#[derive(Serialize)]
pub struct FuncPatFields {
    pub Func: SyntaxResult<Pat>,
    pub Args: ArgPats,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct IfExpr {
    pub(crate) syntax: SyntaxNode,
}
impl IfExpr {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> IfExprFields {
        IfExprFields {
            if_token: self.if_token(),
            Cond: self.Cond(),
            then_token: self.then_token(),
            ThenBranch: self.ThenBranch(),
            else_token: self.else_token(),
            ElseBranch: self.ElseBranch(),
        }
    }
    pub fn if_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 0usize)
    }
    pub fn Cond(&self) -> SyntaxResult<Expr> {
        support::required_node(&self.syntax, 1usize)
    }
    pub fn then_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 2usize)
    }
    pub fn ThenBranch(&self) -> SyntaxResult<Expr> {
        support::required_node(&self.syntax, 3usize)
    }
    pub fn else_token(&self) -> Option<SyntaxToken> {
        support::token(&self.syntax, 4usize)
    }
    pub fn ElseBranch(&self) -> Option<Expr> {
        support::node(&self.syntax, 5usize)
    }
}
impl Serialize for IfExpr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_fields().serialize(serializer)
    }
}
#[derive(Serialize)]
pub struct IfExprFields {
    pub if_token: SyntaxResult<SyntaxToken>,
    pub Cond: SyntaxResult<Expr>,
    pub then_token: SyntaxResult<SyntaxToken>,
    pub ThenBranch: SyntaxResult<Expr>,
    pub else_token: Option<SyntaxToken>,
    pub ElseBranch: Option<Expr>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct InferTy {
    pub(crate) syntax: SyntaxNode,
}
impl InferTy {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> InferTyFields {
        InferTyFields {
            __token: self.__token(),
        }
    }
    pub fn __token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 0usize)
    }
}
impl Serialize for InferTy {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_fields().serialize(serializer)
    }
}
#[derive(Serialize)]
pub struct InferTyFields {
    pub __token: SyntaxResult<SyntaxToken>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct InnerModuleItem {
    pub(crate) syntax: SyntaxNode,
}
impl InnerModuleItem {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> InnerModuleItemFields {
        InnerModuleItemFields {
            Preamble: self.Preamble(),
            eq_token: self.eq_token(),
            Decls: self.Decls(),
        }
    }
    pub fn Preamble(&self) -> Option<ModulePreamble> {
        support::node(&self.syntax, 0usize)
    }
    pub fn eq_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 1usize)
    }
    pub fn Decls(&self) -> ModuleItemList {
        support::list(&self.syntax, 2usize)
    }
}
impl Serialize for InnerModuleItem {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_fields().serialize(serializer)
    }
}
#[derive(Serialize)]
pub struct InnerModuleItemFields {
    pub Preamble: Option<ModulePreamble>,
    pub eq_token: SyntaxResult<SyntaxToken>,
    pub Decls: ModuleItemList,
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
            Value_token: self.Value_token(),
        }
    }
    pub fn Value_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 0usize)
    }
}
impl Serialize for IntLiteral {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_fields().serialize(serializer)
    }
}
#[derive(Serialize)]
pub struct IntLiteralFields {
    pub Value_token: SyntaxResult<SyntaxToken>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct LetDecl {
    pub(crate) syntax: SyntaxNode,
}
impl LetDecl {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> LetDeclFields {
        LetDeclFields {
            let_token: self.let_token(),
            Name: self.Name(),
            eq_token: self.eq_token(),
            Expr: self.Expr(),
        }
    }
    pub fn let_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 0usize)
    }
    pub fn Name(&self) -> SyntaxResult<Name> {
        support::required_node(&self.syntax, 1usize)
    }
    pub fn eq_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 2usize)
    }
    pub fn Expr(&self) -> SyntaxResult<Expr> {
        support::required_node(&self.syntax, 3usize)
    }
}
impl Serialize for LetDecl {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_fields().serialize(serializer)
    }
}
#[derive(Serialize)]
pub struct LetDeclFields {
    pub let_token: SyntaxResult<SyntaxToken>,
    pub Name: SyntaxResult<Name>,
    pub eq_token: SyntaxResult<SyntaxToken>,
    pub Expr: SyntaxResult<Expr>,
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
            Decl: self.Decl(),
            Body: self.Body(),
        }
    }
    pub fn Decl(&self) -> SyntaxResult<LetDecl> {
        support::required_node(&self.syntax, 0usize)
    }
    pub fn Body(&self) -> SyntaxResult<Expr> {
        support::required_node(&self.syntax, 1usize)
    }
}
impl Serialize for LetExpr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_fields().serialize(serializer)
    }
}
#[derive(Serialize)]
pub struct LetExprFields {
    pub Decl: SyntaxResult<LetDecl>,
    pub Body: SyntaxResult<Expr>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ListExpr {
    pub(crate) syntax: SyntaxNode,
}
impl ListExpr {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> ListExprFields {
        ListExprFields {
            l_brack_token: self.l_brack_token(),
            Elements: self.Elements(),
            r_brack_token: self.r_brack_token(),
        }
    }
    pub fn l_brack_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 0usize)
    }
    pub fn Elements(&self) -> ListExprElements {
        support::list(&self.syntax, 1usize)
    }
    pub fn r_brack_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 2usize)
    }
}
impl Serialize for ListExpr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_fields().serialize(serializer)
    }
}
#[derive(Serialize)]
pub struct ListExprFields {
    pub l_brack_token: SyntaxResult<SyntaxToken>,
    pub Elements: ListExprElements,
    pub r_brack_token: SyntaxResult<SyntaxToken>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ListPat {
    pub(crate) syntax: SyntaxNode,
}
impl ListPat {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> ListPatFields {
        ListPatFields {
            l_brack_token: self.l_brack_token(),
            Elements: self.Elements(),
            r_brack_token: self.r_brack_token(),
        }
    }
    pub fn l_brack_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 0usize)
    }
    pub fn Elements(&self) -> ListPatElements {
        support::list(&self.syntax, 1usize)
    }
    pub fn r_brack_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 2usize)
    }
}
impl Serialize for ListPat {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_fields().serialize(serializer)
    }
}
#[derive(Serialize)]
pub struct ListPatFields {
    pub l_brack_token: SyntaxResult<SyntaxToken>,
    pub Elements: ListPatElements,
    pub r_brack_token: SyntaxResult<SyntaxToken>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Literal {
    pub(crate) syntax: SyntaxNode,
}
impl Literal {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> LiteralFields {
        LiteralFields {
            Value_token: self.Value_token(),
        }
    }
    pub fn Value_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 0usize)
    }
}
impl Serialize for Literal {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_fields().serialize(serializer)
    }
}
#[derive(Serialize)]
pub struct LiteralFields {
    pub Value_token: SyntaxResult<SyntaxToken>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct LiteralPat {
    pub(crate) syntax: SyntaxNode,
}
impl LiteralPat {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> LiteralPatFields {
        LiteralPatFields {
            literal: self.literal(),
        }
    }
    pub fn literal(&self) -> SyntaxResult<Literal> {
        support::required_node(&self.syntax, 0usize)
    }
}
impl Serialize for LiteralPat {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_fields().serialize(serializer)
    }
}
#[derive(Serialize)]
pub struct LiteralPatFields {
    pub literal: SyntaxResult<Literal>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct MatchCase {
    pub(crate) syntax: SyntaxNode,
}
impl MatchCase {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> MatchCaseFields {
        MatchCaseFields {
            pat: self.pat(),
            Guard: self.Guard(),
            arrow_token: self.arrow_token(),
            Body: self.Body(),
        }
    }
    pub fn pat(&self) -> SyntaxResult<Pat> {
        support::required_node(&self.syntax, 0usize)
    }
    pub fn Guard(&self) -> Option<MatchGuard> {
        support::node(&self.syntax, 1usize)
    }
    pub fn arrow_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 2usize)
    }
    pub fn Body(&self) -> SyntaxResult<Expr> {
        support::required_node(&self.syntax, 3usize)
    }
}
impl Serialize for MatchCase {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_fields().serialize(serializer)
    }
}
#[derive(Serialize)]
pub struct MatchCaseFields {
    pub pat: SyntaxResult<Pat>,
    pub Guard: Option<MatchGuard>,
    pub arrow_token: SyntaxResult<SyntaxToken>,
    pub Body: SyntaxResult<Expr>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct MatchExpr {
    pub(crate) syntax: SyntaxNode,
}
impl MatchExpr {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> MatchExprFields {
        MatchExprFields {
            match_token: self.match_token(),
            Scrutinee: self.Scrutinee(),
            with_token: self.with_token(),
            LeadingPipeToken_token: self.LeadingPipeToken_token(),
            Cases: self.Cases(),
        }
    }
    pub fn match_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 0usize)
    }
    pub fn Scrutinee(&self) -> SyntaxResult<Expr> {
        support::required_node(&self.syntax, 1usize)
    }
    pub fn with_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 2usize)
    }
    pub fn LeadingPipeToken_token(&self) -> Option<SyntaxToken> {
        support::token(&self.syntax, 3usize)
    }
    pub fn Cases(&self) -> MatchCaseList {
        support::list(&self.syntax, 4usize)
    }
}
impl Serialize for MatchExpr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_fields().serialize(serializer)
    }
}
#[derive(Serialize)]
pub struct MatchExprFields {
    pub match_token: SyntaxResult<SyntaxToken>,
    pub Scrutinee: SyntaxResult<Expr>,
    pub with_token: SyntaxResult<SyntaxToken>,
    pub LeadingPipeToken_token: Option<SyntaxToken>,
    pub Cases: MatchCaseList,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct MatchGuard {
    pub(crate) syntax: SyntaxNode,
}
impl MatchGuard {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> MatchGuardFields {
        MatchGuardFields {
            when_token: self.when_token(),
            Cond: self.Cond(),
        }
    }
    pub fn when_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 0usize)
    }
    pub fn Cond(&self) -> SyntaxResult<Expr> {
        support::required_node(&self.syntax, 1usize)
    }
}
impl Serialize for MatchGuard {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_fields().serialize(serializer)
    }
}
#[derive(Serialize)]
pub struct MatchGuardFields {
    pub when_token: SyntaxResult<SyntaxToken>,
    pub Cond: SyntaxResult<Expr>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct MemberAccessExpr {
    pub(crate) syntax: SyntaxNode,
}
impl MemberAccessExpr {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> MemberAccessExprFields {
        MemberAccessExprFields {
            Target: self.Target(),
            dot_token: self.dot_token(),
            Member: self.Member(),
        }
    }
    pub fn Target(&self) -> SyntaxResult<Expr> {
        support::required_node(&self.syntax, 0usize)
    }
    pub fn dot_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 1usize)
    }
    pub fn Member(&self) -> SyntaxResult<Name> {
        support::required_node(&self.syntax, 2usize)
    }
}
impl Serialize for MemberAccessExpr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_fields().serialize(serializer)
    }
}
#[derive(Serialize)]
pub struct MemberAccessExprFields {
    pub Target: SyntaxResult<Expr>,
    pub dot_token: SyntaxResult<SyntaxToken>,
    pub Member: SyntaxResult<Name>,
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
            module_token: self.module_token(),
            Name: self.Name(),
        }
    }
    pub fn module_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 0usize)
    }
    pub fn Name(&self) -> SyntaxResult<QName> {
        support::required_node(&self.syntax, 1usize)
    }
}
impl Serialize for ModulePreamble {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_fields().serialize(serializer)
    }
}
#[derive(Serialize)]
pub struct ModulePreambleFields {
    pub module_token: SyntaxResult<SyntaxToken>,
    pub Name: SyntaxResult<QName>,
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
            preamble: self.preamble(),
            items: self.items(),
            eof_token: self.eof_token(),
        }
    }
    pub fn preamble(&self) -> Option<ModulePreamble> {
        support::node(&self.syntax, 0usize)
    }
    pub fn items(&self) -> ModuleItemList {
        support::list(&self.syntax, 1usize)
    }
    pub fn eof_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 2usize)
    }
}
impl Serialize for ModuleRoot {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_fields().serialize(serializer)
    }
}
#[derive(Serialize)]
pub struct ModuleRootFields {
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
            Value_token: self.Value_token(),
        }
    }
    pub fn Value_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 0usize)
    }
}
impl Serialize for Name {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_fields().serialize(serializer)
    }
}
#[derive(Serialize)]
pub struct NameFields {
    pub Value_token: SyntaxResult<SyntaxToken>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct NamePatField {
    pub(crate) syntax: SyntaxNode,
}
impl NamePatField {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> NamePatFieldFields {
        NamePatFieldFields {
            Name: self.Name(),
            eq_token: self.eq_token(),
            pat: self.pat(),
        }
    }
    pub fn Name(&self) -> SyntaxResult<QName> {
        support::required_node(&self.syntax, 0usize)
    }
    pub fn eq_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 1usize)
    }
    pub fn pat(&self) -> SyntaxResult<Pat> {
        support::required_node(&self.syntax, 2usize)
    }
}
impl Serialize for NamePatField {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_fields().serialize(serializer)
    }
}
#[derive(Serialize)]
pub struct NamePatFieldFields {
    pub Name: SyntaxResult<QName>,
    pub eq_token: SyntaxResult<SyntaxToken>,
    pub pat: SyntaxResult<Pat>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct NamedPat {
    pub(crate) syntax: SyntaxNode,
}
impl NamedPat {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> NamedPatFields {
        NamedPatFields { Name: self.Name() }
    }
    pub fn Name(&self) -> SyntaxResult<QName> {
        support::required_node(&self.syntax, 0usize)
    }
}
impl Serialize for NamedPat {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_fields().serialize(serializer)
    }
}
#[derive(Serialize)]
pub struct NamedPatFields {
    pub Name: SyntaxResult<QName>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct OpenDecl {
    pub(crate) syntax: SyntaxNode,
}
impl OpenDecl {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> OpenDeclFields {
        OpenDeclFields {
            open_token: self.open_token(),
            Module: self.Module(),
        }
    }
    pub fn open_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 0usize)
    }
    pub fn Module(&self) -> SyntaxResult<QName> {
        support::required_node(&self.syntax, 1usize)
    }
}
impl Serialize for OpenDecl {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_fields().serialize(serializer)
    }
}
#[derive(Serialize)]
pub struct OpenDeclFields {
    pub open_token: SyntaxResult<SyntaxToken>,
    pub Module: SyntaxResult<QName>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Operator {
    pub(crate) syntax: SyntaxNode,
}
impl Operator {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> OperatorFields {
        OperatorFields {
            Value_token: self.Value_token(),
        }
    }
    pub fn Value_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 0usize)
    }
}
impl Serialize for Operator {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_fields().serialize(serializer)
    }
}
#[derive(Serialize)]
pub struct OperatorFields {
    pub Value_token: SyntaxResult<SyntaxToken>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct OrPat {
    pub(crate) syntax: SyntaxNode,
}
impl OrPat {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> OrPatFields {
        OrPatFields {
            Lhs: self.Lhs(),
            bitwise_or_token: self.bitwise_or_token(),
            Rhs: self.Rhs(),
        }
    }
    pub fn Lhs(&self) -> SyntaxResult<Pat> {
        support::required_node(&self.syntax, 0usize)
    }
    pub fn bitwise_or_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 1usize)
    }
    pub fn Rhs(&self) -> SyntaxResult<Pat> {
        support::required_node(&self.syntax, 2usize)
    }
}
impl Serialize for OrPat {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_fields().serialize(serializer)
    }
}
#[derive(Serialize)]
pub struct OrPatFields {
    pub Lhs: SyntaxResult<Pat>,
    pub bitwise_or_token: SyntaxResult<SyntaxToken>,
    pub Rhs: SyntaxResult<Pat>,
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
        self.as_fields().serialize(serializer)
    }
}
#[derive(Serialize)]
pub struct ParenExprFields {
    pub l_paren_token: SyntaxResult<SyntaxToken>,
    pub expr: SyntaxResult<Expr>,
    pub r_paren_token: SyntaxResult<SyntaxToken>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ParenPat {
    pub(crate) syntax: SyntaxNode,
}
impl ParenPat {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> ParenPatFields {
        ParenPatFields {
            l_paren_token: self.l_paren_token(),
            pat: self.pat(),
            r_paren_token: self.r_paren_token(),
        }
    }
    pub fn l_paren_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 0usize)
    }
    pub fn pat(&self) -> SyntaxResult<Pat> {
        support::required_node(&self.syntax, 1usize)
    }
    pub fn r_paren_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 2usize)
    }
}
impl Serialize for ParenPat {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_fields().serialize(serializer)
    }
}
#[derive(Serialize)]
pub struct ParenPatFields {
    pub l_paren_token: SyntaxResult<SyntaxToken>,
    pub pat: SyntaxResult<Pat>,
    pub r_paren_token: SyntaxResult<SyntaxToken>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ParenTy {
    pub(crate) syntax: SyntaxNode,
}
impl ParenTy {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> ParenTyFields {
        ParenTyFields {
            l_paren_token: self.l_paren_token(),
            ty: self.ty(),
            r_paren_token: self.r_paren_token(),
        }
    }
    pub fn l_paren_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 0usize)
    }
    pub fn ty(&self) -> SyntaxResult<Ty> {
        support::required_node(&self.syntax, 1usize)
    }
    pub fn r_paren_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 2usize)
    }
}
impl Serialize for ParenTy {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_fields().serialize(serializer)
    }
}
#[derive(Serialize)]
pub struct ParenTyFields {
    pub l_paren_token: SyntaxResult<SyntaxToken>,
    pub ty: SyntaxResult<Ty>,
    pub r_paren_token: SyntaxResult<SyntaxToken>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct QName {
    pub(crate) syntax: SyntaxNode,
}
impl QName {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> QNameFields {
        QNameFields {
            Qualifier: self.Qualifier(),
            dot_token: self.dot_token(),
            Segment: self.Segment(),
        }
    }
    pub fn Qualifier(&self) -> Option<QName> {
        support::node(&self.syntax, 0usize)
    }
    pub fn dot_token(&self) -> Option<SyntaxToken> {
        support::token(&self.syntax, 1usize)
    }
    pub fn Segment(&self) -> SyntaxResult<QNameSegment> {
        support::required_node(&self.syntax, 2usize)
    }
}
impl Serialize for QName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_fields().serialize(serializer)
    }
}
#[derive(Serialize)]
pub struct QNameFields {
    pub Qualifier: Option<QName>,
    pub dot_token: Option<SyntaxToken>,
    pub Segment: SyntaxResult<QNameSegment>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct QNameSegment {
    pub(crate) syntax: SyntaxNode,
}
impl QNameSegment {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> QNameSegmentFields {
        QNameSegmentFields { Name: self.Name() }
    }
    pub fn Name(&self) -> SyntaxResult<Name> {
        support::required_node(&self.syntax, 0usize)
    }
}
impl Serialize for QNameSegment {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_fields().serialize(serializer)
    }
}
#[derive(Serialize)]
pub struct QNameSegmentFields {
    pub Name: SyntaxResult<Name>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct QTy {
    pub(crate) syntax: SyntaxNode,
}
impl QTy {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> QTyFields {
        QTyFields { Name: self.Name() }
    }
    pub fn Name(&self) -> SyntaxResult<QName> {
        support::required_node(&self.syntax, 0usize)
    }
}
impl Serialize for QTy {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_fields().serialize(serializer)
    }
}
#[derive(Serialize)]
pub struct QTyFields {
    pub Name: SyntaxResult<QName>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct RecordPat {
    pub(crate) syntax: SyntaxNode,
}
impl RecordPat {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> RecordPatFields {
        RecordPatFields {
            l_curly_token: self.l_curly_token(),
            Fields: self.Fields(),
            r_curly_token: self.r_curly_token(),
        }
    }
    pub fn l_curly_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 0usize)
    }
    pub fn Fields(&self) -> RecordFields {
        support::list(&self.syntax, 1usize)
    }
    pub fn r_curly_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 2usize)
    }
}
impl Serialize for RecordPat {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_fields().serialize(serializer)
    }
}
#[derive(Serialize)]
pub struct RecordPatFields {
    pub l_curly_token: SyntaxResult<SyntaxToken>,
    pub Fields: RecordFields,
    pub r_curly_token: SyntaxResult<SyntaxToken>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct SeqExpr {
    pub(crate) syntax: SyntaxNode,
}
impl SeqExpr {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> SeqExprFields {
        SeqExprFields {
            First: self.First(),
            Second: self.Second(),
        }
    }
    pub fn First(&self) -> SyntaxResult<Expr> {
        support::required_node(&self.syntax, 0usize)
    }
    pub fn Second(&self) -> SyntaxResult<Expr> {
        support::required_node(&self.syntax, 1usize)
    }
}
impl Serialize for SeqExpr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_fields().serialize(serializer)
    }
}
#[derive(Serialize)]
pub struct SeqExprFields {
    pub First: SyntaxResult<Expr>,
    pub Second: SyntaxResult<Expr>,
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
            Value_token: self.Value_token(),
        }
    }
    pub fn Value_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 0usize)
    }
}
impl Serialize for StringLiteral {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_fields().serialize(serializer)
    }
}
#[derive(Serialize)]
pub struct StringLiteralFields {
    pub Value_token: SyntaxResult<SyntaxToken>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct TypedExpr {
    pub(crate) syntax: SyntaxNode,
}
impl TypedExpr {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> TypedExprFields {
        TypedExprFields {
            Expr: self.Expr(),
            colon_token: self.colon_token(),
            Target: self.Target(),
        }
    }
    pub fn Expr(&self) -> SyntaxResult<Expr> {
        support::required_node(&self.syntax, 0usize)
    }
    pub fn colon_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 1usize)
    }
    pub fn Target(&self) -> SyntaxResult<Ty> {
        support::required_node(&self.syntax, 2usize)
    }
}
impl Serialize for TypedExpr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_fields().serialize(serializer)
    }
}
#[derive(Serialize)]
pub struct TypedExprFields {
    pub Expr: SyntaxResult<Expr>,
    pub colon_token: SyntaxResult<SyntaxToken>,
    pub Target: SyntaxResult<Ty>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct TypedPat {
    pub(crate) syntax: SyntaxNode,
}
impl TypedPat {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> TypedPatFields {
        TypedPatFields {
            pat: self.pat(),
            colon_token: self.colon_token(),
            Target: self.Target(),
        }
    }
    pub fn pat(&self) -> SyntaxResult<Pat> {
        support::required_node(&self.syntax, 0usize)
    }
    pub fn colon_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 1usize)
    }
    pub fn Target(&self) -> SyntaxResult<Ty> {
        support::required_node(&self.syntax, 2usize)
    }
}
impl Serialize for TypedPat {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_fields().serialize(serializer)
    }
}
#[derive(Serialize)]
pub struct TypedPatFields {
    pub pat: SyntaxResult<Pat>,
    pub colon_token: SyntaxResult<SyntaxToken>,
    pub Target: SyntaxResult<Ty>,
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
            Op: self.Op(),
            Operand: self.Operand(),
        }
    }
    pub fn Op(&self) -> SyntaxResult<QName> {
        support::required_node(&self.syntax, 0usize)
    }
    pub fn Operand(&self) -> SyntaxResult<Expr> {
        support::required_node(&self.syntax, 1usize)
    }
}
impl Serialize for UnaryExpr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_fields().serialize(serializer)
    }
}
#[derive(Serialize)]
pub struct UnaryExprFields {
    pub Op: SyntaxResult<QName>,
    pub Operand: SyntaxResult<Expr>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct UnitLiteral {
    pub(crate) syntax: SyntaxNode,
}
impl UnitLiteral {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> UnitLiteralFields {
        UnitLiteralFields {
            l_paren_token: self.l_paren_token(),
            r_paren_token: self.r_paren_token(),
        }
    }
    pub fn l_paren_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 0usize)
    }
    pub fn r_paren_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 1usize)
    }
}
impl Serialize for UnitLiteral {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_fields().serialize(serializer)
    }
}
#[derive(Serialize)]
pub struct UnitLiteralFields {
    pub l_paren_token: SyntaxResult<SyntaxToken>,
    pub r_paren_token: SyntaxResult<SyntaxToken>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct VarExpr {
    pub(crate) syntax: SyntaxNode,
}
impl VarExpr {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> VarExprFields {
        VarExprFields { Name: self.Name() }
    }
    pub fn Name(&self) -> SyntaxResult<Name> {
        support::required_node(&self.syntax, 0usize)
    }
}
impl Serialize for VarExpr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_fields().serialize(serializer)
    }
}
#[derive(Serialize)]
pub struct VarExprFields {
    pub Name: SyntaxResult<Name>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct WildPat {
    pub(crate) syntax: SyntaxNode,
}
impl WildPat {
    #[doc = r" Create an AstNode from a SyntaxNode without checking its kind"]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r" This function must be guarded with a call to [AstNode::can_cast]"]
    #[doc = r" or a match on [SyntaxNode::kind]"]
    #[inline]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }
    pub fn as_fields(&self) -> WildPatFields {
        WildPatFields {
            __token: self.__token(),
        }
    }
    pub fn __token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 0usize)
    }
}
impl Serialize for WildPat {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_fields().serialize(serializer)
    }
}
#[derive(Serialize)]
pub struct WildPatFields {
    pub __token: SyntaxResult<SyntaxToken>,
}
#[derive(Clone, PartialEq, Eq, Hash, Serialize)]
pub enum ArgPat {
    NamePatField(NamePatField),
    Pat(Pat),
}
impl ArgPat {
    pub fn as_name_pat_field(&self) -> Option<&NamePatField> {
        match &self {
            Self::NamePatField(item) => Some(item),
            _ => None,
        }
    }
    pub fn as_pat(&self) -> Option<&Pat> {
        match &self {
            Self::Pat(item) => Some(item),
            _ => None,
        }
    }
}
#[derive(Clone, PartialEq, Eq, Hash, Serialize)]
pub enum Expr {
    AppExpr(AppExpr),
    BinExpr(BinExpr),
    BogusExpr(BogusExpr),
    FunExpr(FunExpr),
    IfExpr(IfExpr),
    LetExpr(LetExpr),
    Literal(Literal),
    MemberAccessExpr(MemberAccessExpr),
    ParenExpr(ParenExpr),
    SeqExpr(SeqExpr),
    VarExpr(VarExpr),
}
impl Expr {
    pub fn as_app_expr(&self) -> Option<&AppExpr> {
        match &self {
            Self::AppExpr(item) => Some(item),
            _ => None,
        }
    }
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
    pub fn as_fun_expr(&self) -> Option<&FunExpr> {
        match &self {
            Self::FunExpr(item) => Some(item),
            _ => None,
        }
    }
    pub fn as_if_expr(&self) -> Option<&IfExpr> {
        match &self {
            Self::IfExpr(item) => Some(item),
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
    pub fn as_member_access_expr(&self) -> Option<&MemberAccessExpr> {
        match &self {
            Self::MemberAccessExpr(item) => Some(item),
            _ => None,
        }
    }
    pub fn as_paren_expr(&self) -> Option<&ParenExpr> {
        match &self {
            Self::ParenExpr(item) => Some(item),
            _ => None,
        }
    }
    pub fn as_seq_expr(&self) -> Option<&SeqExpr> {
        match &self {
            Self::SeqExpr(item) => Some(item),
            _ => None,
        }
    }
    pub fn as_var_expr(&self) -> Option<&VarExpr> {
        match &self {
            Self::VarExpr(item) => Some(item),
            _ => None,
        }
    }
}
#[derive(Clone, PartialEq, Eq, Hash, Serialize)]
pub enum ModuleItem {
    BogusDecl(BogusDecl),
    Expr(Expr),
    InnerModuleItem(InnerModuleItem),
    LetDecl(LetDecl),
    OpenDecl(OpenDecl),
}
impl ModuleItem {
    pub fn as_bogus_decl(&self) -> Option<&BogusDecl> {
        match &self {
            Self::BogusDecl(item) => Some(item),
            _ => None,
        }
    }
    pub fn as_expr(&self) -> Option<&Expr> {
        match &self {
            Self::Expr(item) => Some(item),
            _ => None,
        }
    }
    pub fn as_inner_module_item(&self) -> Option<&InnerModuleItem> {
        match &self {
            Self::InnerModuleItem(item) => Some(item),
            _ => None,
        }
    }
    pub fn as_let_decl(&self) -> Option<&LetDecl> {
        match &self {
            Self::LetDecl(item) => Some(item),
            _ => None,
        }
    }
    pub fn as_open_decl(&self) -> Option<&OpenDecl> {
        match &self {
            Self::OpenDecl(item) => Some(item),
            _ => None,
        }
    }
}
#[derive(Clone, PartialEq, Eq, Hash, Serialize)]
pub enum Pat {
    AndPat(AndPat),
    AsPat(AsPat),
    BogusPat(BogusPat),
    ConsPat(ConsPat),
    FuncPat(FuncPat),
    ListPat(ListPat),
    LiteralPat(LiteralPat),
    NamedPat(NamedPat),
    OrPat(OrPat),
    ParenPat(ParenPat),
    RecordPat(RecordPat),
    TuplePat(TuplePat),
    TypedPat(TypedPat),
    WildPat(WildPat),
}
impl Pat {
    pub fn as_and_pat(&self) -> Option<&AndPat> {
        match &self {
            Self::AndPat(item) => Some(item),
            _ => None,
        }
    }
    pub fn as_as_pat(&self) -> Option<&AsPat> {
        match &self {
            Self::AsPat(item) => Some(item),
            _ => None,
        }
    }
    pub fn as_bogus_pat(&self) -> Option<&BogusPat> {
        match &self {
            Self::BogusPat(item) => Some(item),
            _ => None,
        }
    }
    pub fn as_cons_pat(&self) -> Option<&ConsPat> {
        match &self {
            Self::ConsPat(item) => Some(item),
            _ => None,
        }
    }
    pub fn as_func_pat(&self) -> Option<&FuncPat> {
        match &self {
            Self::FuncPat(item) => Some(item),
            _ => None,
        }
    }
    pub fn as_list_pat(&self) -> Option<&ListPat> {
        match &self {
            Self::ListPat(item) => Some(item),
            _ => None,
        }
    }
    pub fn as_literal_pat(&self) -> Option<&LiteralPat> {
        match &self {
            Self::LiteralPat(item) => Some(item),
            _ => None,
        }
    }
    pub fn as_named_pat(&self) -> Option<&NamedPat> {
        match &self {
            Self::NamedPat(item) => Some(item),
            _ => None,
        }
    }
    pub fn as_or_pat(&self) -> Option<&OrPat> {
        match &self {
            Self::OrPat(item) => Some(item),
            _ => None,
        }
    }
    pub fn as_paren_pat(&self) -> Option<&ParenPat> {
        match &self {
            Self::ParenPat(item) => Some(item),
            _ => None,
        }
    }
    pub fn as_record_pat(&self) -> Option<&RecordPat> {
        match &self {
            Self::RecordPat(item) => Some(item),
            _ => None,
        }
    }
    pub fn as_tuple_pat(&self) -> Option<&TuplePat> {
        match &self {
            Self::TuplePat(item) => Some(item),
            _ => None,
        }
    }
    pub fn as_typed_pat(&self) -> Option<&TypedPat> {
        match &self {
            Self::TypedPat(item) => Some(item),
            _ => None,
        }
    }
    pub fn as_wild_pat(&self) -> Option<&WildPat> {
        match &self {
            Self::WildPat(item) => Some(item),
            _ => None,
        }
    }
}
#[derive(Clone, PartialEq, Eq, Hash, Serialize)]
pub enum Ty {
    BogusTy(BogusTy),
    FnTy(FnTy),
    InferTy(InferTy),
    ParenTy(ParenTy),
    QTy(QTy),
    TupleTy(TupleTy),
}
impl Ty {
    pub fn as_bogus_ty(&self) -> Option<&BogusTy> {
        match &self {
            Self::BogusTy(item) => Some(item),
            _ => None,
        }
    }
    pub fn as_fn_ty(&self) -> Option<&FnTy> {
        match &self {
            Self::FnTy(item) => Some(item),
            _ => None,
        }
    }
    pub fn as_infer_ty(&self) -> Option<&InferTy> {
        match &self {
            Self::InferTy(item) => Some(item),
            _ => None,
        }
    }
    pub fn as_paren_ty(&self) -> Option<&ParenTy> {
        match &self {
            Self::ParenTy(item) => Some(item),
            _ => None,
        }
    }
    pub fn as_q_ty(&self) -> Option<&QTy> {
        match &self {
            Self::QTy(item) => Some(item),
            _ => None,
        }
    }
    pub fn as_tuple_ty(&self) -> Option<&TupleTy> {
        match &self {
            Self::TupleTy(item) => Some(item),
            _ => None,
        }
    }
}
impl AstNode for AndPat {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(AND_PAT as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == AND_PAT
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
impl std::fmt::Debug for AndPat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("AndPat")
                .field("Lhs", &support::DebugSyntaxResult(self.Lhs()))
                .field("amp_token", &support::DebugSyntaxResult(self.amp_token()))
                .field("Rhs", &support::DebugSyntaxResult(self.Rhs()))
                .finish()
        } else {
            f.debug_struct("AndPat").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<AndPat> for SyntaxNode {
    fn from(n: AndPat) -> Self {
        n.syntax
    }
}
impl From<AndPat> for SyntaxElement {
    fn from(n: AndPat) -> Self {
        n.syntax.into()
    }
}
impl AstNode for AppExpr {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(APP_EXPR as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == APP_EXPR
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
impl std::fmt::Debug for AppExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("AppExpr")
                .field("Func", &support::DebugSyntaxResult(self.Func()))
                .field("Arg", &support::DebugSyntaxResult(self.Arg()))
                .finish()
        } else {
            f.debug_struct("AppExpr").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<AppExpr> for SyntaxNode {
    fn from(n: AppExpr) -> Self {
        n.syntax
    }
}
impl From<AppExpr> for SyntaxElement {
    fn from(n: AppExpr) -> Self {
        n.syntax.into()
    }
}
impl AstNode for AsPat {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> = SyntaxKindSet::from_raw(RawSyntaxKind(AS_PAT as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == AS_PAT
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
impl std::fmt::Debug for AsPat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("AsPat")
                .field("Lhs", &support::DebugSyntaxResult(self.Lhs()))
                .field("as_token", &support::DebugSyntaxResult(self.as_token()))
                .field("Rhs", &support::DebugSyntaxResult(self.Rhs()))
                .finish()
        } else {
            f.debug_struct("AsPat").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<AsPat> for SyntaxNode {
    fn from(n: AsPat) -> Self {
        n.syntax
    }
}
impl From<AsPat> for SyntaxElement {
    fn from(n: AsPat) -> Self {
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
                .field("Left", &support::DebugSyntaxResult(self.Left()))
                .field("Op_token", &support::DebugSyntaxResult(self.Op_token()))
                .field("Right", &support::DebugSyntaxResult(self.Right()))
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
impl AstNode for Binding {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(BINDING as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == BINDING
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
impl std::fmt::Debug for Binding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("Binding")
                .field("Pat", &support::DebugSyntaxResult(self.Pat()))
                .field("eq_token", &support::DebugSyntaxResult(self.eq_token()))
                .field("Expr", &support::DebugSyntaxResult(self.Expr()))
                .finish()
        } else {
            f.debug_struct("Binding").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<Binding> for SyntaxNode {
    fn from(n: Binding) -> Self {
        n.syntax
    }
}
impl From<Binding> for SyntaxElement {
    fn from(n: Binding) -> Self {
        n.syntax.into()
    }
}
impl AstNode for BoolLiteral {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(BOOL_LITERAL as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == BOOL_LITERAL
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
impl std::fmt::Debug for BoolLiteral {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("BoolLiteral")
                .field("ValueToken", &support::DebugSyntaxResult(self.ValueToken()))
                .finish()
        } else {
            f.debug_struct("BoolLiteral").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<BoolLiteral> for SyntaxNode {
    fn from(n: BoolLiteral) -> Self {
        n.syntax
    }
}
impl From<BoolLiteral> for SyntaxElement {
    fn from(n: BoolLiteral) -> Self {
        n.syntax.into()
    }
}
impl AstNode for CharLiteral {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(CHAR_LITERAL as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == CHAR_LITERAL
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
impl std::fmt::Debug for CharLiteral {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("CharLiteral")
                .field(
                    "Value_token",
                    &support::DebugSyntaxResult(self.Value_token()),
                )
                .finish()
        } else {
            f.debug_struct("CharLiteral").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<CharLiteral> for SyntaxNode {
    fn from(n: CharLiteral) -> Self {
        n.syntax
    }
}
impl From<CharLiteral> for SyntaxElement {
    fn from(n: CharLiteral) -> Self {
        n.syntax.into()
    }
}
impl AstNode for ConsPat {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(CONS_PAT as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == CONS_PAT
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
impl std::fmt::Debug for ConsPat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("ConsPat")
                .field("Head", &support::DebugSyntaxResult(self.Head()))
                .field(
                    "double_colon_token",
                    &support::DebugSyntaxResult(self.double_colon_token()),
                )
                .field("Tail", &support::DebugSyntaxResult(self.Tail()))
                .finish()
        } else {
            f.debug_struct("ConsPat").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<ConsPat> for SyntaxNode {
    fn from(n: ConsPat) -> Self {
        n.syntax
    }
}
impl From<ConsPat> for SyntaxElement {
    fn from(n: ConsPat) -> Self {
        n.syntax.into()
    }
}
impl AstNode for FnTy {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> = SyntaxKindSet::from_raw(RawSyntaxKind(FN_TY as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == FN_TY
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
impl std::fmt::Debug for FnTy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("FnTy")
                .field("Arg", &support::DebugSyntaxResult(self.Arg()))
                .field(
                    "arrow_token",
                    &support::DebugSyntaxResult(self.arrow_token()),
                )
                .field("Ret", &support::DebugSyntaxResult(self.Ret()))
                .finish()
        } else {
            f.debug_struct("FnTy").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<FnTy> for SyntaxNode {
    fn from(n: FnTy) -> Self {
        n.syntax
    }
}
impl From<FnTy> for SyntaxElement {
    fn from(n: FnTy) -> Self {
        n.syntax.into()
    }
}
impl AstNode for FunExpr {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(FUN_EXPR as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == FUN_EXPR
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
impl std::fmt::Debug for FunExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("FunExpr")
                .field("fun_token", &support::DebugSyntaxResult(self.fun_token()))
                .field("Arg", &support::DebugSyntaxResult(self.Arg()))
                .field(
                    "arrow_token",
                    &support::DebugSyntaxResult(self.arrow_token()),
                )
                .field("Body", &support::DebugSyntaxResult(self.Body()))
                .finish()
        } else {
            f.debug_struct("FunExpr").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<FunExpr> for SyntaxNode {
    fn from(n: FunExpr) -> Self {
        n.syntax
    }
}
impl From<FunExpr> for SyntaxElement {
    fn from(n: FunExpr) -> Self {
        n.syntax.into()
    }
}
impl AstNode for FuncPat {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(FUNC_PAT as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == FUNC_PAT
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
impl std::fmt::Debug for FuncPat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("FuncPat")
                .field("Func", &support::DebugSyntaxResult(self.Func()))
                .field("Args", &self.Args())
                .finish()
        } else {
            f.debug_struct("FuncPat").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<FuncPat> for SyntaxNode {
    fn from(n: FuncPat) -> Self {
        n.syntax
    }
}
impl From<FuncPat> for SyntaxElement {
    fn from(n: FuncPat) -> Self {
        n.syntax.into()
    }
}
impl AstNode for IfExpr {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(IF_EXPR as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == IF_EXPR
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
impl std::fmt::Debug for IfExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("IfExpr")
                .field("if_token", &support::DebugSyntaxResult(self.if_token()))
                .field("Cond", &support::DebugSyntaxResult(self.Cond()))
                .field("then_token", &support::DebugSyntaxResult(self.then_token()))
                .field("ThenBranch", &support::DebugSyntaxResult(self.ThenBranch()))
                .field(
                    "else_token",
                    &support::DebugOptionalElement(self.else_token()),
                )
                .field(
                    "ElseBranch",
                    &support::DebugOptionalElement(self.ElseBranch()),
                )
                .finish()
        } else {
            f.debug_struct("IfExpr").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<IfExpr> for SyntaxNode {
    fn from(n: IfExpr) -> Self {
        n.syntax
    }
}
impl From<IfExpr> for SyntaxElement {
    fn from(n: IfExpr) -> Self {
        n.syntax.into()
    }
}
impl AstNode for InferTy {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(INFER_TY as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == INFER_TY
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
impl std::fmt::Debug for InferTy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("InferTy")
                .field("__token", &support::DebugSyntaxResult(self.__token()))
                .finish()
        } else {
            f.debug_struct("InferTy").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<InferTy> for SyntaxNode {
    fn from(n: InferTy) -> Self {
        n.syntax
    }
}
impl From<InferTy> for SyntaxElement {
    fn from(n: InferTy) -> Self {
        n.syntax.into()
    }
}
impl AstNode for InnerModuleItem {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(INNER_MODULE_ITEM as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == INNER_MODULE_ITEM
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
impl std::fmt::Debug for InnerModuleItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("InnerModuleItem")
                .field("Preamble", &support::DebugOptionalElement(self.Preamble()))
                .field("eq_token", &support::DebugSyntaxResult(self.eq_token()))
                .field("Decls", &self.Decls())
                .finish()
        } else {
            f.debug_struct("InnerModuleItem").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<InnerModuleItem> for SyntaxNode {
    fn from(n: InnerModuleItem) -> Self {
        n.syntax
    }
}
impl From<InnerModuleItem> for SyntaxElement {
    fn from(n: InnerModuleItem) -> Self {
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
                    "Value_token",
                    &support::DebugSyntaxResult(self.Value_token()),
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
impl AstNode for LetDecl {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(LET_DECL as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == LET_DECL
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
impl std::fmt::Debug for LetDecl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("LetDecl")
                .field("let_token", &support::DebugSyntaxResult(self.let_token()))
                .field("Name", &support::DebugSyntaxResult(self.Name()))
                .field("eq_token", &support::DebugSyntaxResult(self.eq_token()))
                .field("Expr", &support::DebugSyntaxResult(self.Expr()))
                .finish()
        } else {
            f.debug_struct("LetDecl").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<LetDecl> for SyntaxNode {
    fn from(n: LetDecl) -> Self {
        n.syntax
    }
}
impl From<LetDecl> for SyntaxElement {
    fn from(n: LetDecl) -> Self {
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
                .field("Decl", &support::DebugSyntaxResult(self.Decl()))
                .field("Body", &support::DebugSyntaxResult(self.Body()))
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
impl AstNode for ListExpr {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(LIST_EXPR as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == LIST_EXPR
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
impl std::fmt::Debug for ListExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("ListExpr")
                .field(
                    "l_brack_token",
                    &support::DebugSyntaxResult(self.l_brack_token()),
                )
                .field("Elements", &self.Elements())
                .field(
                    "r_brack_token",
                    &support::DebugSyntaxResult(self.r_brack_token()),
                )
                .finish()
        } else {
            f.debug_struct("ListExpr").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<ListExpr> for SyntaxNode {
    fn from(n: ListExpr) -> Self {
        n.syntax
    }
}
impl From<ListExpr> for SyntaxElement {
    fn from(n: ListExpr) -> Self {
        n.syntax.into()
    }
}
impl AstNode for ListPat {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(LIST_PAT as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == LIST_PAT
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
impl std::fmt::Debug for ListPat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("ListPat")
                .field(
                    "l_brack_token",
                    &support::DebugSyntaxResult(self.l_brack_token()),
                )
                .field("Elements", &self.Elements())
                .field(
                    "r_brack_token",
                    &support::DebugSyntaxResult(self.r_brack_token()),
                )
                .finish()
        } else {
            f.debug_struct("ListPat").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<ListPat> for SyntaxNode {
    fn from(n: ListPat) -> Self {
        n.syntax
    }
}
impl From<ListPat> for SyntaxElement {
    fn from(n: ListPat) -> Self {
        n.syntax.into()
    }
}
impl AstNode for Literal {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(LITERAL as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == LITERAL
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
impl std::fmt::Debug for Literal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("Literal")
                .field(
                    "Value_token",
                    &support::DebugSyntaxResult(self.Value_token()),
                )
                .finish()
        } else {
            f.debug_struct("Literal").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<Literal> for SyntaxNode {
    fn from(n: Literal) -> Self {
        n.syntax
    }
}
impl From<Literal> for SyntaxElement {
    fn from(n: Literal) -> Self {
        n.syntax.into()
    }
}
impl AstNode for LiteralPat {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(LITERAL_PAT as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == LITERAL_PAT
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
impl std::fmt::Debug for LiteralPat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("LiteralPat")
                .field("literal", &support::DebugSyntaxResult(self.literal()))
                .finish()
        } else {
            f.debug_struct("LiteralPat").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<LiteralPat> for SyntaxNode {
    fn from(n: LiteralPat) -> Self {
        n.syntax
    }
}
impl From<LiteralPat> for SyntaxElement {
    fn from(n: LiteralPat) -> Self {
        n.syntax.into()
    }
}
impl AstNode for MatchCase {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(MATCH_CASE as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == MATCH_CASE
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
impl std::fmt::Debug for MatchCase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("MatchCase")
                .field("pat", &support::DebugSyntaxResult(self.pat()))
                .field("Guard", &support::DebugOptionalElement(self.Guard()))
                .field(
                    "arrow_token",
                    &support::DebugSyntaxResult(self.arrow_token()),
                )
                .field("Body", &support::DebugSyntaxResult(self.Body()))
                .finish()
        } else {
            f.debug_struct("MatchCase").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<MatchCase> for SyntaxNode {
    fn from(n: MatchCase) -> Self {
        n.syntax
    }
}
impl From<MatchCase> for SyntaxElement {
    fn from(n: MatchCase) -> Self {
        n.syntax.into()
    }
}
impl AstNode for MatchExpr {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(MATCH_EXPR as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == MATCH_EXPR
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
impl std::fmt::Debug for MatchExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("MatchExpr")
                .field(
                    "match_token",
                    &support::DebugSyntaxResult(self.match_token()),
                )
                .field("Scrutinee", &support::DebugSyntaxResult(self.Scrutinee()))
                .field("with_token", &support::DebugSyntaxResult(self.with_token()))
                .field(
                    "LeadingPipeToken_token",
                    &support::DebugOptionalElement(self.LeadingPipeToken_token()),
                )
                .field("Cases", &self.Cases())
                .finish()
        } else {
            f.debug_struct("MatchExpr").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<MatchExpr> for SyntaxNode {
    fn from(n: MatchExpr) -> Self {
        n.syntax
    }
}
impl From<MatchExpr> for SyntaxElement {
    fn from(n: MatchExpr) -> Self {
        n.syntax.into()
    }
}
impl AstNode for MatchGuard {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(MATCH_GUARD as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == MATCH_GUARD
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
impl std::fmt::Debug for MatchGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("MatchGuard")
                .field("when_token", &support::DebugSyntaxResult(self.when_token()))
                .field("Cond", &support::DebugSyntaxResult(self.Cond()))
                .finish()
        } else {
            f.debug_struct("MatchGuard").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<MatchGuard> for SyntaxNode {
    fn from(n: MatchGuard) -> Self {
        n.syntax
    }
}
impl From<MatchGuard> for SyntaxElement {
    fn from(n: MatchGuard) -> Self {
        n.syntax.into()
    }
}
impl AstNode for MemberAccessExpr {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(MEMBER_ACCESS_EXPR as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == MEMBER_ACCESS_EXPR
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
impl std::fmt::Debug for MemberAccessExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("MemberAccessExpr")
                .field("Target", &support::DebugSyntaxResult(self.Target()))
                .field("dot_token", &support::DebugSyntaxResult(self.dot_token()))
                .field("Member", &support::DebugSyntaxResult(self.Member()))
                .finish()
        } else {
            f.debug_struct("MemberAccessExpr").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<MemberAccessExpr> for SyntaxNode {
    fn from(n: MemberAccessExpr) -> Self {
        n.syntax
    }
}
impl From<MemberAccessExpr> for SyntaxElement {
    fn from(n: MemberAccessExpr) -> Self {
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
                .field(
                    "module_token",
                    &support::DebugSyntaxResult(self.module_token()),
                )
                .field("Name", &support::DebugSyntaxResult(self.Name()))
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
                    "Value_token",
                    &support::DebugSyntaxResult(self.Value_token()),
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
impl AstNode for NamePatField {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(NAME_PAT_FIELD as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == NAME_PAT_FIELD
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
impl std::fmt::Debug for NamePatField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("NamePatField")
                .field("Name", &support::DebugSyntaxResult(self.Name()))
                .field("eq_token", &support::DebugSyntaxResult(self.eq_token()))
                .field("pat", &support::DebugSyntaxResult(self.pat()))
                .finish()
        } else {
            f.debug_struct("NamePatField").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<NamePatField> for SyntaxNode {
    fn from(n: NamePatField) -> Self {
        n.syntax
    }
}
impl From<NamePatField> for SyntaxElement {
    fn from(n: NamePatField) -> Self {
        n.syntax.into()
    }
}
impl AstNode for NamedPat {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(NAMED_PAT as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == NAMED_PAT
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
impl std::fmt::Debug for NamedPat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("NamedPat")
                .field("Name", &support::DebugSyntaxResult(self.Name()))
                .finish()
        } else {
            f.debug_struct("NamedPat").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<NamedPat> for SyntaxNode {
    fn from(n: NamedPat) -> Self {
        n.syntax
    }
}
impl From<NamedPat> for SyntaxElement {
    fn from(n: NamedPat) -> Self {
        n.syntax.into()
    }
}
impl AstNode for OpenDecl {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(OPEN_DECL as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == OPEN_DECL
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
impl std::fmt::Debug for OpenDecl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("OpenDecl")
                .field("open_token", &support::DebugSyntaxResult(self.open_token()))
                .field("Module", &support::DebugSyntaxResult(self.Module()))
                .finish()
        } else {
            f.debug_struct("OpenDecl").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<OpenDecl> for SyntaxNode {
    fn from(n: OpenDecl) -> Self {
        n.syntax
    }
}
impl From<OpenDecl> for SyntaxElement {
    fn from(n: OpenDecl) -> Self {
        n.syntax.into()
    }
}
impl AstNode for Operator {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(OPERATOR as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == OPERATOR
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
impl std::fmt::Debug for Operator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("Operator")
                .field(
                    "Value_token",
                    &support::DebugSyntaxResult(self.Value_token()),
                )
                .finish()
        } else {
            f.debug_struct("Operator").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<Operator> for SyntaxNode {
    fn from(n: Operator) -> Self {
        n.syntax
    }
}
impl From<Operator> for SyntaxElement {
    fn from(n: Operator) -> Self {
        n.syntax.into()
    }
}
impl AstNode for OrPat {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> = SyntaxKindSet::from_raw(RawSyntaxKind(OR_PAT as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == OR_PAT
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
impl std::fmt::Debug for OrPat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("OrPat")
                .field("Lhs", &support::DebugSyntaxResult(self.Lhs()))
                .field(
                    "bitwise_or_token",
                    &support::DebugSyntaxResult(self.bitwise_or_token()),
                )
                .field("Rhs", &support::DebugSyntaxResult(self.Rhs()))
                .finish()
        } else {
            f.debug_struct("OrPat").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<OrPat> for SyntaxNode {
    fn from(n: OrPat) -> Self {
        n.syntax
    }
}
impl From<OrPat> for SyntaxElement {
    fn from(n: OrPat) -> Self {
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
impl AstNode for ParenPat {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(PAREN_PAT as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == PAREN_PAT
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
impl std::fmt::Debug for ParenPat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("ParenPat")
                .field(
                    "l_paren_token",
                    &support::DebugSyntaxResult(self.l_paren_token()),
                )
                .field("pat", &support::DebugSyntaxResult(self.pat()))
                .field(
                    "r_paren_token",
                    &support::DebugSyntaxResult(self.r_paren_token()),
                )
                .finish()
        } else {
            f.debug_struct("ParenPat").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<ParenPat> for SyntaxNode {
    fn from(n: ParenPat) -> Self {
        n.syntax
    }
}
impl From<ParenPat> for SyntaxElement {
    fn from(n: ParenPat) -> Self {
        n.syntax.into()
    }
}
impl AstNode for ParenTy {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(PAREN_TY as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == PAREN_TY
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
impl std::fmt::Debug for ParenTy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("ParenTy")
                .field(
                    "l_paren_token",
                    &support::DebugSyntaxResult(self.l_paren_token()),
                )
                .field("ty", &support::DebugSyntaxResult(self.ty()))
                .field(
                    "r_paren_token",
                    &support::DebugSyntaxResult(self.r_paren_token()),
                )
                .finish()
        } else {
            f.debug_struct("ParenTy").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<ParenTy> for SyntaxNode {
    fn from(n: ParenTy) -> Self {
        n.syntax
    }
}
impl From<ParenTy> for SyntaxElement {
    fn from(n: ParenTy) -> Self {
        n.syntax.into()
    }
}
impl AstNode for QName {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> = SyntaxKindSet::from_raw(RawSyntaxKind(Q_NAME as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == Q_NAME
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
impl std::fmt::Debug for QName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("QName")
                .field(
                    "Qualifier",
                    &support::DebugOptionalElement(self.Qualifier()),
                )
                .field(
                    "dot_token",
                    &support::DebugOptionalElement(self.dot_token()),
                )
                .field("Segment", &support::DebugSyntaxResult(self.Segment()))
                .finish()
        } else {
            f.debug_struct("QName").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<QName> for SyntaxNode {
    fn from(n: QName) -> Self {
        n.syntax
    }
}
impl From<QName> for SyntaxElement {
    fn from(n: QName) -> Self {
        n.syntax.into()
    }
}
impl AstNode for QNameSegment {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(Q_NAME_SEGMENT as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == Q_NAME_SEGMENT
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
impl std::fmt::Debug for QNameSegment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("QNameSegment")
                .field("Name", &support::DebugSyntaxResult(self.Name()))
                .finish()
        } else {
            f.debug_struct("QNameSegment").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<QNameSegment> for SyntaxNode {
    fn from(n: QNameSegment) -> Self {
        n.syntax
    }
}
impl From<QNameSegment> for SyntaxElement {
    fn from(n: QNameSegment) -> Self {
        n.syntax.into()
    }
}
impl AstNode for QTy {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> = SyntaxKindSet::from_raw(RawSyntaxKind(Q_TY as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == Q_TY
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
impl std::fmt::Debug for QTy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("QTy")
                .field("Name", &support::DebugSyntaxResult(self.Name()))
                .finish()
        } else {
            f.debug_struct("QTy").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<QTy> for SyntaxNode {
    fn from(n: QTy) -> Self {
        n.syntax
    }
}
impl From<QTy> for SyntaxElement {
    fn from(n: QTy) -> Self {
        n.syntax.into()
    }
}
impl AstNode for RecordPat {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(RECORD_PAT as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == RECORD_PAT
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
impl std::fmt::Debug for RecordPat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("RecordPat")
                .field(
                    "l_curly_token",
                    &support::DebugSyntaxResult(self.l_curly_token()),
                )
                .field("Fields", &self.Fields())
                .field(
                    "r_curly_token",
                    &support::DebugSyntaxResult(self.r_curly_token()),
                )
                .finish()
        } else {
            f.debug_struct("RecordPat").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<RecordPat> for SyntaxNode {
    fn from(n: RecordPat) -> Self {
        n.syntax
    }
}
impl From<RecordPat> for SyntaxElement {
    fn from(n: RecordPat) -> Self {
        n.syntax.into()
    }
}
impl AstNode for SeqExpr {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(SEQ_EXPR as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SEQ_EXPR
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
impl std::fmt::Debug for SeqExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("SeqExpr")
                .field("First", &support::DebugSyntaxResult(self.First()))
                .field("Second", &support::DebugSyntaxResult(self.Second()))
                .finish()
        } else {
            f.debug_struct("SeqExpr").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<SeqExpr> for SyntaxNode {
    fn from(n: SeqExpr) -> Self {
        n.syntax
    }
}
impl From<SeqExpr> for SyntaxElement {
    fn from(n: SeqExpr) -> Self {
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
                    "Value_token",
                    &support::DebugSyntaxResult(self.Value_token()),
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
impl AstNode for TypedExpr {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(TYPED_EXPR as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == TYPED_EXPR
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
impl std::fmt::Debug for TypedExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("TypedExpr")
                .field("Expr", &support::DebugSyntaxResult(self.Expr()))
                .field(
                    "colon_token",
                    &support::DebugSyntaxResult(self.colon_token()),
                )
                .field("Target", &support::DebugSyntaxResult(self.Target()))
                .finish()
        } else {
            f.debug_struct("TypedExpr").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<TypedExpr> for SyntaxNode {
    fn from(n: TypedExpr) -> Self {
        n.syntax
    }
}
impl From<TypedExpr> for SyntaxElement {
    fn from(n: TypedExpr) -> Self {
        n.syntax.into()
    }
}
impl AstNode for TypedPat {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(TYPED_PAT as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == TYPED_PAT
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
impl std::fmt::Debug for TypedPat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("TypedPat")
                .field("pat", &support::DebugSyntaxResult(self.pat()))
                .field(
                    "colon_token",
                    &support::DebugSyntaxResult(self.colon_token()),
                )
                .field("Target", &support::DebugSyntaxResult(self.Target()))
                .finish()
        } else {
            f.debug_struct("TypedPat").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<TypedPat> for SyntaxNode {
    fn from(n: TypedPat) -> Self {
        n.syntax
    }
}
impl From<TypedPat> for SyntaxElement {
    fn from(n: TypedPat) -> Self {
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
                .field("Op", &support::DebugSyntaxResult(self.Op()))
                .field("Operand", &support::DebugSyntaxResult(self.Operand()))
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
impl AstNode for UnitLiteral {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(UNIT_LITERAL as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == UNIT_LITERAL
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
impl std::fmt::Debug for UnitLiteral {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("UnitLiteral")
                .field(
                    "l_paren_token",
                    &support::DebugSyntaxResult(self.l_paren_token()),
                )
                .field(
                    "r_paren_token",
                    &support::DebugSyntaxResult(self.r_paren_token()),
                )
                .finish()
        } else {
            f.debug_struct("UnitLiteral").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<UnitLiteral> for SyntaxNode {
    fn from(n: UnitLiteral) -> Self {
        n.syntax
    }
}
impl From<UnitLiteral> for SyntaxElement {
    fn from(n: UnitLiteral) -> Self {
        n.syntax.into()
    }
}
impl AstNode for VarExpr {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(VAR_EXPR as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == VAR_EXPR
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
impl std::fmt::Debug for VarExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("VarExpr")
                .field("Name", &support::DebugSyntaxResult(self.Name()))
                .finish()
        } else {
            f.debug_struct("VarExpr").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<VarExpr> for SyntaxNode {
    fn from(n: VarExpr) -> Self {
        n.syntax
    }
}
impl From<VarExpr> for SyntaxElement {
    fn from(n: VarExpr) -> Self {
        n.syntax.into()
    }
}
impl AstNode for WildPat {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(WILD_PAT as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == WILD_PAT
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
impl std::fmt::Debug for WildPat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH : std :: cell :: Cell < u8 > = const { std :: cell :: Cell :: new (0) } };
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("WildPat")
                .field("__token", &support::DebugSyntaxResult(self.__token()))
                .finish()
        } else {
            f.debug_struct("WildPat").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}
impl From<WildPat> for SyntaxNode {
    fn from(n: WildPat) -> Self {
        n.syntax
    }
}
impl From<WildPat> for SyntaxElement {
    fn from(n: WildPat) -> Self {
        n.syntax.into()
    }
}
impl From<NamePatField> for ArgPat {
    fn from(node: NamePatField) -> Self {
        Self::NamePatField(node)
    }
}
impl AstNode for ArgPat {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> = NamePatField::KIND_SET.union(Pat::KIND_SET);
    fn can_cast(kind: SyntaxKind) -> bool {
        match kind {
            NAME_PAT_FIELD => true,
            k if Pat::can_cast(k) => true,
            _ => false,
        }
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        let res = match syntax.kind() {
            NAME_PAT_FIELD => Self::NamePatField(NamePatField { syntax }),
            _ => {
                if let Some(pat) = Pat::cast(syntax) {
                    return Some(Self::Pat(pat));
                }
                return None;
            },
        };
        Some(res)
    }
    fn syntax(&self) -> &SyntaxNode {
        match self {
            Self::NamePatField(it) => it.syntax(),
            Self::Pat(it) => it.syntax(),
        }
    }
    fn into_syntax(self) -> SyntaxNode {
        match self {
            Self::NamePatField(it) => it.into_syntax(),
            Self::Pat(it) => it.into_syntax(),
        }
    }
}
impl std::fmt::Debug for ArgPat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NamePatField(it) => std::fmt::Debug::fmt(it, f),
            Self::Pat(it) => std::fmt::Debug::fmt(it, f),
        }
    }
}
impl From<ArgPat> for SyntaxNode {
    fn from(n: ArgPat) -> Self {
        match n {
            ArgPat::NamePatField(it) => it.into_syntax(),
            ArgPat::Pat(it) => it.into_syntax(),
        }
    }
}
impl From<ArgPat> for SyntaxElement {
    fn from(n: ArgPat) -> Self {
        let node: SyntaxNode = n.into();
        node.into()
    }
}
impl From<AppExpr> for Expr {
    fn from(node: AppExpr) -> Self {
        Self::AppExpr(node)
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
impl From<FunExpr> for Expr {
    fn from(node: FunExpr) -> Self {
        Self::FunExpr(node)
    }
}
impl From<IfExpr> for Expr {
    fn from(node: IfExpr) -> Self {
        Self::IfExpr(node)
    }
}
impl From<LetExpr> for Expr {
    fn from(node: LetExpr) -> Self {
        Self::LetExpr(node)
    }
}
impl From<Literal> for Expr {
    fn from(node: Literal) -> Self {
        Self::Literal(node)
    }
}
impl From<MemberAccessExpr> for Expr {
    fn from(node: MemberAccessExpr) -> Self {
        Self::MemberAccessExpr(node)
    }
}
impl From<ParenExpr> for Expr {
    fn from(node: ParenExpr) -> Self {
        Self::ParenExpr(node)
    }
}
impl From<SeqExpr> for Expr {
    fn from(node: SeqExpr) -> Self {
        Self::SeqExpr(node)
    }
}
impl From<VarExpr> for Expr {
    fn from(node: VarExpr) -> Self {
        Self::VarExpr(node)
    }
}
impl AstNode for Expr {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> = AppExpr::KIND_SET
        .union(BinExpr::KIND_SET)
        .union(BogusExpr::KIND_SET)
        .union(FunExpr::KIND_SET)
        .union(IfExpr::KIND_SET)
        .union(LetExpr::KIND_SET)
        .union(Literal::KIND_SET)
        .union(MemberAccessExpr::KIND_SET)
        .union(ParenExpr::KIND_SET)
        .union(SeqExpr::KIND_SET)
        .union(VarExpr::KIND_SET);
    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(
            kind,
            APP_EXPR
                | BIN_EXPR
                | BOGUS_EXPR
                | FUN_EXPR
                | IF_EXPR
                | LET_EXPR
                | LITERAL
                | MEMBER_ACCESS_EXPR
                | PAREN_EXPR
                | SEQ_EXPR
                | VAR_EXPR
        )
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        let res = match syntax.kind() {
            APP_EXPR => Self::AppExpr(AppExpr { syntax }),
            BIN_EXPR => Self::BinExpr(BinExpr { syntax }),
            BOGUS_EXPR => Self::BogusExpr(BogusExpr { syntax }),
            FUN_EXPR => Self::FunExpr(FunExpr { syntax }),
            IF_EXPR => Self::IfExpr(IfExpr { syntax }),
            LET_EXPR => Self::LetExpr(LetExpr { syntax }),
            LITERAL => Self::Literal(Literal { syntax }),
            MEMBER_ACCESS_EXPR => Self::MemberAccessExpr(MemberAccessExpr { syntax }),
            PAREN_EXPR => Self::ParenExpr(ParenExpr { syntax }),
            SEQ_EXPR => Self::SeqExpr(SeqExpr { syntax }),
            VAR_EXPR => Self::VarExpr(VarExpr { syntax }),
            _ => return None,
        };
        Some(res)
    }
    fn syntax(&self) -> &SyntaxNode {
        match self {
            Self::AppExpr(it) => it.syntax(),
            Self::BinExpr(it) => it.syntax(),
            Self::BogusExpr(it) => it.syntax(),
            Self::FunExpr(it) => it.syntax(),
            Self::IfExpr(it) => it.syntax(),
            Self::LetExpr(it) => it.syntax(),
            Self::Literal(it) => it.syntax(),
            Self::MemberAccessExpr(it) => it.syntax(),
            Self::ParenExpr(it) => it.syntax(),
            Self::SeqExpr(it) => it.syntax(),
            Self::VarExpr(it) => it.syntax(),
        }
    }
    fn into_syntax(self) -> SyntaxNode {
        match self {
            Self::AppExpr(it) => it.into_syntax(),
            Self::BinExpr(it) => it.into_syntax(),
            Self::BogusExpr(it) => it.into_syntax(),
            Self::FunExpr(it) => it.into_syntax(),
            Self::IfExpr(it) => it.into_syntax(),
            Self::LetExpr(it) => it.into_syntax(),
            Self::Literal(it) => it.into_syntax(),
            Self::MemberAccessExpr(it) => it.into_syntax(),
            Self::ParenExpr(it) => it.into_syntax(),
            Self::SeqExpr(it) => it.into_syntax(),
            Self::VarExpr(it) => it.into_syntax(),
        }
    }
}
impl std::fmt::Debug for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AppExpr(it) => std::fmt::Debug::fmt(it, f),
            Self::BinExpr(it) => std::fmt::Debug::fmt(it, f),
            Self::BogusExpr(it) => std::fmt::Debug::fmt(it, f),
            Self::FunExpr(it) => std::fmt::Debug::fmt(it, f),
            Self::IfExpr(it) => std::fmt::Debug::fmt(it, f),
            Self::LetExpr(it) => std::fmt::Debug::fmt(it, f),
            Self::Literal(it) => std::fmt::Debug::fmt(it, f),
            Self::MemberAccessExpr(it) => std::fmt::Debug::fmt(it, f),
            Self::ParenExpr(it) => std::fmt::Debug::fmt(it, f),
            Self::SeqExpr(it) => std::fmt::Debug::fmt(it, f),
            Self::VarExpr(it) => std::fmt::Debug::fmt(it, f),
        }
    }
}
impl From<Expr> for SyntaxNode {
    fn from(n: Expr) -> Self {
        match n {
            Expr::AppExpr(it) => it.into_syntax(),
            Expr::BinExpr(it) => it.into_syntax(),
            Expr::BogusExpr(it) => it.into_syntax(),
            Expr::FunExpr(it) => it.into_syntax(),
            Expr::IfExpr(it) => it.into_syntax(),
            Expr::LetExpr(it) => it.into_syntax(),
            Expr::Literal(it) => it.into_syntax(),
            Expr::MemberAccessExpr(it) => it.into_syntax(),
            Expr::ParenExpr(it) => it.into_syntax(),
            Expr::SeqExpr(it) => it.into_syntax(),
            Expr::VarExpr(it) => it.into_syntax(),
        }
    }
}
impl From<Expr> for SyntaxElement {
    fn from(n: Expr) -> Self {
        let node: SyntaxNode = n.into();
        node.into()
    }
}
impl From<BogusDecl> for ModuleItem {
    fn from(node: BogusDecl) -> Self {
        Self::BogusDecl(node)
    }
}
impl From<InnerModuleItem> for ModuleItem {
    fn from(node: InnerModuleItem) -> Self {
        Self::InnerModuleItem(node)
    }
}
impl From<LetDecl> for ModuleItem {
    fn from(node: LetDecl) -> Self {
        Self::LetDecl(node)
    }
}
impl From<OpenDecl> for ModuleItem {
    fn from(node: OpenDecl) -> Self {
        Self::OpenDecl(node)
    }
}
impl AstNode for ModuleItem {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> = BogusDecl::KIND_SET
        .union(Expr::KIND_SET)
        .union(InnerModuleItem::KIND_SET)
        .union(LetDecl::KIND_SET)
        .union(OpenDecl::KIND_SET);
    fn can_cast(kind: SyntaxKind) -> bool {
        match kind {
            BOGUS_DECL | INNER_MODULE_ITEM | LET_DECL | OPEN_DECL => true,
            k if Expr::can_cast(k) => true,
            _ => false,
        }
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        let res = match syntax.kind() {
            BOGUS_DECL => Self::BogusDecl(BogusDecl { syntax }),
            INNER_MODULE_ITEM => Self::InnerModuleItem(InnerModuleItem { syntax }),
            LET_DECL => Self::LetDecl(LetDecl { syntax }),
            OPEN_DECL => Self::OpenDecl(OpenDecl { syntax }),
            _ => {
                if let Some(expr) = Expr::cast(syntax) {
                    return Some(Self::Expr(expr));
                }
                return None;
            },
        };
        Some(res)
    }
    fn syntax(&self) -> &SyntaxNode {
        match self {
            Self::BogusDecl(it) => it.syntax(),
            Self::InnerModuleItem(it) => it.syntax(),
            Self::LetDecl(it) => it.syntax(),
            Self::OpenDecl(it) => it.syntax(),
            Self::Expr(it) => it.syntax(),
        }
    }
    fn into_syntax(self) -> SyntaxNode {
        match self {
            Self::BogusDecl(it) => it.into_syntax(),
            Self::InnerModuleItem(it) => it.into_syntax(),
            Self::LetDecl(it) => it.into_syntax(),
            Self::OpenDecl(it) => it.into_syntax(),
            Self::Expr(it) => it.into_syntax(),
        }
    }
}
impl std::fmt::Debug for ModuleItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BogusDecl(it) => std::fmt::Debug::fmt(it, f),
            Self::Expr(it) => std::fmt::Debug::fmt(it, f),
            Self::InnerModuleItem(it) => std::fmt::Debug::fmt(it, f),
            Self::LetDecl(it) => std::fmt::Debug::fmt(it, f),
            Self::OpenDecl(it) => std::fmt::Debug::fmt(it, f),
        }
    }
}
impl From<ModuleItem> for SyntaxNode {
    fn from(n: ModuleItem) -> Self {
        match n {
            ModuleItem::BogusDecl(it) => it.into_syntax(),
            ModuleItem::Expr(it) => it.into_syntax(),
            ModuleItem::InnerModuleItem(it) => it.into_syntax(),
            ModuleItem::LetDecl(it) => it.into_syntax(),
            ModuleItem::OpenDecl(it) => it.into_syntax(),
        }
    }
}
impl From<ModuleItem> for SyntaxElement {
    fn from(n: ModuleItem) -> Self {
        let node: SyntaxNode = n.into();
        node.into()
    }
}
impl From<AndPat> for Pat {
    fn from(node: AndPat) -> Self {
        Self::AndPat(node)
    }
}
impl From<AsPat> for Pat {
    fn from(node: AsPat) -> Self {
        Self::AsPat(node)
    }
}
impl From<BogusPat> for Pat {
    fn from(node: BogusPat) -> Self {
        Self::BogusPat(node)
    }
}
impl From<ConsPat> for Pat {
    fn from(node: ConsPat) -> Self {
        Self::ConsPat(node)
    }
}
impl From<FuncPat> for Pat {
    fn from(node: FuncPat) -> Self {
        Self::FuncPat(node)
    }
}
impl From<ListPat> for Pat {
    fn from(node: ListPat) -> Self {
        Self::ListPat(node)
    }
}
impl From<LiteralPat> for Pat {
    fn from(node: LiteralPat) -> Self {
        Self::LiteralPat(node)
    }
}
impl From<NamedPat> for Pat {
    fn from(node: NamedPat) -> Self {
        Self::NamedPat(node)
    }
}
impl From<OrPat> for Pat {
    fn from(node: OrPat) -> Self {
        Self::OrPat(node)
    }
}
impl From<ParenPat> for Pat {
    fn from(node: ParenPat) -> Self {
        Self::ParenPat(node)
    }
}
impl From<RecordPat> for Pat {
    fn from(node: RecordPat) -> Self {
        Self::RecordPat(node)
    }
}
impl From<TuplePat> for Pat {
    fn from(node: TuplePat) -> Self {
        Self::TuplePat(node)
    }
}
impl From<TypedPat> for Pat {
    fn from(node: TypedPat) -> Self {
        Self::TypedPat(node)
    }
}
impl From<WildPat> for Pat {
    fn from(node: WildPat) -> Self {
        Self::WildPat(node)
    }
}
impl AstNode for Pat {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> = AndPat::KIND_SET
        .union(AsPat::KIND_SET)
        .union(BogusPat::KIND_SET)
        .union(ConsPat::KIND_SET)
        .union(FuncPat::KIND_SET)
        .union(ListPat::KIND_SET)
        .union(LiteralPat::KIND_SET)
        .union(NamedPat::KIND_SET)
        .union(OrPat::KIND_SET)
        .union(ParenPat::KIND_SET)
        .union(RecordPat::KIND_SET)
        .union(TuplePat::KIND_SET)
        .union(TypedPat::KIND_SET)
        .union(WildPat::KIND_SET);
    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(
            kind,
            AND_PAT
                | AS_PAT
                | BOGUS_PAT
                | CONS_PAT
                | FUNC_PAT
                | LIST_PAT
                | LITERAL_PAT
                | NAMED_PAT
                | OR_PAT
                | PAREN_PAT
                | RECORD_PAT
                | TUPLE_PAT
                | TYPED_PAT
                | WILD_PAT
        )
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        let res = match syntax.kind() {
            AND_PAT => Self::AndPat(AndPat { syntax }),
            AS_PAT => Self::AsPat(AsPat { syntax }),
            BOGUS_PAT => Self::BogusPat(BogusPat { syntax }),
            CONS_PAT => Self::ConsPat(ConsPat { syntax }),
            FUNC_PAT => Self::FuncPat(FuncPat { syntax }),
            LIST_PAT => Self::ListPat(ListPat { syntax }),
            LITERAL_PAT => Self::LiteralPat(LiteralPat { syntax }),
            NAMED_PAT => Self::NamedPat(NamedPat { syntax }),
            OR_PAT => Self::OrPat(OrPat { syntax }),
            PAREN_PAT => Self::ParenPat(ParenPat { syntax }),
            RECORD_PAT => Self::RecordPat(RecordPat { syntax }),
            TUPLE_PAT => Self::TuplePat(TuplePat::cast(syntax)?),
            TYPED_PAT => Self::TypedPat(TypedPat { syntax }),
            WILD_PAT => Self::WildPat(WildPat { syntax }),
            _ => return None,
        };
        Some(res)
    }
    fn syntax(&self) -> &SyntaxNode {
        match self {
            Self::AndPat(it) => it.syntax(),
            Self::AsPat(it) => it.syntax(),
            Self::BogusPat(it) => it.syntax(),
            Self::ConsPat(it) => it.syntax(),
            Self::FuncPat(it) => it.syntax(),
            Self::ListPat(it) => it.syntax(),
            Self::LiteralPat(it) => it.syntax(),
            Self::NamedPat(it) => it.syntax(),
            Self::OrPat(it) => it.syntax(),
            Self::ParenPat(it) => it.syntax(),
            Self::RecordPat(it) => it.syntax(),
            Self::TuplePat(it) => it.syntax(),
            Self::TypedPat(it) => it.syntax(),
            Self::WildPat(it) => it.syntax(),
        }
    }
    fn into_syntax(self) -> SyntaxNode {
        match self {
            Self::AndPat(it) => it.into_syntax(),
            Self::AsPat(it) => it.into_syntax(),
            Self::BogusPat(it) => it.into_syntax(),
            Self::ConsPat(it) => it.into_syntax(),
            Self::FuncPat(it) => it.into_syntax(),
            Self::ListPat(it) => it.into_syntax(),
            Self::LiteralPat(it) => it.into_syntax(),
            Self::NamedPat(it) => it.into_syntax(),
            Self::OrPat(it) => it.into_syntax(),
            Self::ParenPat(it) => it.into_syntax(),
            Self::RecordPat(it) => it.into_syntax(),
            Self::TuplePat(it) => it.into_syntax(),
            Self::TypedPat(it) => it.into_syntax(),
            Self::WildPat(it) => it.into_syntax(),
        }
    }
}
impl std::fmt::Debug for Pat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AndPat(it) => std::fmt::Debug::fmt(it, f),
            Self::AsPat(it) => std::fmt::Debug::fmt(it, f),
            Self::BogusPat(it) => std::fmt::Debug::fmt(it, f),
            Self::ConsPat(it) => std::fmt::Debug::fmt(it, f),
            Self::FuncPat(it) => std::fmt::Debug::fmt(it, f),
            Self::ListPat(it) => std::fmt::Debug::fmt(it, f),
            Self::LiteralPat(it) => std::fmt::Debug::fmt(it, f),
            Self::NamedPat(it) => std::fmt::Debug::fmt(it, f),
            Self::OrPat(it) => std::fmt::Debug::fmt(it, f),
            Self::ParenPat(it) => std::fmt::Debug::fmt(it, f),
            Self::RecordPat(it) => std::fmt::Debug::fmt(it, f),
            Self::TuplePat(it) => std::fmt::Debug::fmt(it, f),
            Self::TypedPat(it) => std::fmt::Debug::fmt(it, f),
            Self::WildPat(it) => std::fmt::Debug::fmt(it, f),
        }
    }
}
impl From<Pat> for SyntaxNode {
    fn from(n: Pat) -> Self {
        match n {
            Pat::AndPat(it) => it.into_syntax(),
            Pat::AsPat(it) => it.into_syntax(),
            Pat::BogusPat(it) => it.into_syntax(),
            Pat::ConsPat(it) => it.into_syntax(),
            Pat::FuncPat(it) => it.into_syntax(),
            Pat::ListPat(it) => it.into_syntax(),
            Pat::LiteralPat(it) => it.into_syntax(),
            Pat::NamedPat(it) => it.into_syntax(),
            Pat::OrPat(it) => it.into_syntax(),
            Pat::ParenPat(it) => it.into_syntax(),
            Pat::RecordPat(it) => it.into_syntax(),
            Pat::TuplePat(it) => it.into_syntax(),
            Pat::TypedPat(it) => it.into_syntax(),
            Pat::WildPat(it) => it.into_syntax(),
        }
    }
}
impl From<Pat> for SyntaxElement {
    fn from(n: Pat) -> Self {
        let node: SyntaxNode = n.into();
        node.into()
    }
}
impl From<BogusTy> for Ty {
    fn from(node: BogusTy) -> Self {
        Self::BogusTy(node)
    }
}
impl From<FnTy> for Ty {
    fn from(node: FnTy) -> Self {
        Self::FnTy(node)
    }
}
impl From<InferTy> for Ty {
    fn from(node: InferTy) -> Self {
        Self::InferTy(node)
    }
}
impl From<ParenTy> for Ty {
    fn from(node: ParenTy) -> Self {
        Self::ParenTy(node)
    }
}
impl From<QTy> for Ty {
    fn from(node: QTy) -> Self {
        Self::QTy(node)
    }
}
impl From<TupleTy> for Ty {
    fn from(node: TupleTy) -> Self {
        Self::TupleTy(node)
    }
}
impl AstNode for Ty {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> = BogusTy::KIND_SET
        .union(FnTy::KIND_SET)
        .union(InferTy::KIND_SET)
        .union(ParenTy::KIND_SET)
        .union(QTy::KIND_SET)
        .union(TupleTy::KIND_SET);
    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(
            kind,
            BOGUS_TY | FN_TY | INFER_TY | PAREN_TY | Q_TY | TUPLE_TY
        )
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        let res = match syntax.kind() {
            BOGUS_TY => Self::BogusTy(BogusTy { syntax }),
            FN_TY => Self::FnTy(FnTy { syntax }),
            INFER_TY => Self::InferTy(InferTy { syntax }),
            PAREN_TY => Self::ParenTy(ParenTy { syntax }),
            Q_TY => Self::QTy(QTy { syntax }),
            TUPLE_TY => Self::TupleTy(TupleTy::cast(syntax)?),
            _ => return None,
        };
        Some(res)
    }
    fn syntax(&self) -> &SyntaxNode {
        match self {
            Self::BogusTy(it) => it.syntax(),
            Self::FnTy(it) => it.syntax(),
            Self::InferTy(it) => it.syntax(),
            Self::ParenTy(it) => it.syntax(),
            Self::QTy(it) => it.syntax(),
            Self::TupleTy(it) => it.syntax(),
        }
    }
    fn into_syntax(self) -> SyntaxNode {
        match self {
            Self::BogusTy(it) => it.into_syntax(),
            Self::FnTy(it) => it.into_syntax(),
            Self::InferTy(it) => it.into_syntax(),
            Self::ParenTy(it) => it.into_syntax(),
            Self::QTy(it) => it.into_syntax(),
            Self::TupleTy(it) => it.into_syntax(),
        }
    }
}
impl std::fmt::Debug for Ty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BogusTy(it) => std::fmt::Debug::fmt(it, f),
            Self::FnTy(it) => std::fmt::Debug::fmt(it, f),
            Self::InferTy(it) => std::fmt::Debug::fmt(it, f),
            Self::ParenTy(it) => std::fmt::Debug::fmt(it, f),
            Self::QTy(it) => std::fmt::Debug::fmt(it, f),
            Self::TupleTy(it) => std::fmt::Debug::fmt(it, f),
        }
    }
}
impl From<Ty> for SyntaxNode {
    fn from(n: Ty) -> Self {
        match n {
            Ty::BogusTy(it) => it.into_syntax(),
            Ty::FnTy(it) => it.into_syntax(),
            Ty::InferTy(it) => it.into_syntax(),
            Ty::ParenTy(it) => it.into_syntax(),
            Ty::QTy(it) => it.into_syntax(),
            Ty::TupleTy(it) => it.into_syntax(),
        }
    }
}
impl From<Ty> for SyntaxElement {
    fn from(n: Ty) -> Self {
        let node: SyntaxNode = n.into();
        node.into()
    }
}
impl std::fmt::Display for ArgPat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for Expr {
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
impl std::fmt::Display for Ty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for AndPat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for AppExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for AsPat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for BinExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for Binding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for BoolLiteral {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for CharLiteral {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for ConsPat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for FnTy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for FunExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for FuncPat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for IfExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for InferTy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for InnerModuleItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for IntLiteral {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for LetDecl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for LetExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for ListExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for ListPat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for Literal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for LiteralPat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for MatchCase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for MatchExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for MatchGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for MemberAccessExpr {
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
impl std::fmt::Display for NamePatField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for NamedPat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for OpenDecl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for Operator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for OrPat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for ParenExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for ParenPat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for ParenTy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for QName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for QNameSegment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for QTy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for RecordPat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for SeqExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for StringLiteral {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for TypedExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for TypedPat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for UnaryExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for UnitLiteral {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for VarExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
impl std::fmt::Display for WildPat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.syntax(), f)
    }
}
#[derive(Clone, PartialEq, Eq, Hash, Serialize)]
pub struct Bogus {
    syntax: SyntaxNode,
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
#[derive(Clone, PartialEq, Eq, Hash, Serialize)]
pub struct BogusDecl {
    syntax: SyntaxNode,
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
#[derive(Clone, PartialEq, Eq, Hash, Serialize)]
pub struct BogusExpr {
    syntax: SyntaxNode,
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
#[derive(Clone, PartialEq, Eq, Hash, Serialize)]
pub struct BogusPat {
    syntax: SyntaxNode,
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
#[derive(Clone, PartialEq, Eq, Hash, Serialize)]
pub struct BogusTy {
    syntax: SyntaxNode,
}
impl BogusTy {
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
impl AstNode for BogusTy {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(BOGUS_TY as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == BOGUS_TY
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
impl std::fmt::Debug for BogusTy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BogusTy")
            .field("items", &DebugSyntaxElementChildren(self.items()))
            .finish()
    }
}
impl From<BogusTy> for SyntaxNode {
    fn from(n: BogusTy) -> Self {
        n.syntax
    }
}
impl From<BogusTy> for SyntaxElement {
    fn from(n: BogusTy) -> Self {
        n.syntax.into()
    }
}
biome_rowan::declare_node_union! { pub AnyBogusNode = Bogus | BogusDecl | BogusExpr | BogusPat | BogusTy }
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct ArgPats {
    syntax_list: SyntaxList,
}
impl ArgPats {
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
impl AstNode for ArgPats {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(ARG_PATS as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == ARG_PATS
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
impl Serialize for ArgPats {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.len()))?;
        for e in self.iter() {
            seq.serialize_element(&e)?;
        }
        seq.end()
    }
}
impl AstNodeList for ArgPats {
    type Language = Language;
    type Node = ArgPat;
    fn syntax_list(&self) -> &SyntaxList {
        &self.syntax_list
    }
    fn into_syntax_list(self) -> SyntaxList {
        self.syntax_list
    }
}
impl Debug for ArgPats {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("ArgPats ")?;
        f.debug_list().entries(self.iter()).finish()
    }
}
impl IntoIterator for &ArgPats {
    type Item = ArgPat;
    type IntoIter = AstNodeListIterator<Language, ArgPat>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl IntoIterator for ArgPats {
    type Item = ArgPat;
    type IntoIter = AstNodeListIterator<Language, ArgPat>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct ListExprElements {
    syntax_list: SyntaxList,
}
impl ListExprElements {
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
impl AstNode for ListExprElements {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(LIST_EXPR_ELEMENTS as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == LIST_EXPR_ELEMENTS
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
impl Serialize for ListExprElements {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.len()))?;
        for e in self.iter() {
            seq.serialize_element(&e)?;
        }
        seq.end()
    }
}
impl AstSeparatedList for ListExprElements {
    type Language = Language;
    type Node = Expr;
    fn syntax_list(&self) -> &SyntaxList {
        &self.syntax_list
    }
    fn into_syntax_list(self) -> SyntaxList {
        self.syntax_list
    }
}
impl Debug for ListExprElements {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("ListExprElements ")?;
        f.debug_list().entries(self.elements()).finish()
    }
}
impl IntoIterator for ListExprElements {
    type Item = SyntaxResult<Expr>;
    type IntoIter = AstSeparatedListNodesIterator<Language, Expr>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl IntoIterator for &ListExprElements {
    type Item = SyntaxResult<Expr>;
    type IntoIter = AstSeparatedListNodesIterator<Language, Expr>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct ListPatElements {
    syntax_list: SyntaxList,
}
impl ListPatElements {
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
impl AstNode for ListPatElements {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(LIST_PAT_ELEMENTS as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == LIST_PAT_ELEMENTS
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
impl Serialize for ListPatElements {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.len()))?;
        for e in self.iter() {
            seq.serialize_element(&e)?;
        }
        seq.end()
    }
}
impl AstSeparatedList for ListPatElements {
    type Language = Language;
    type Node = Pat;
    fn syntax_list(&self) -> &SyntaxList {
        &self.syntax_list
    }
    fn into_syntax_list(self) -> SyntaxList {
        self.syntax_list
    }
}
impl Debug for ListPatElements {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("ListPatElements ")?;
        f.debug_list().entries(self.elements()).finish()
    }
}
impl IntoIterator for ListPatElements {
    type Item = SyntaxResult<Pat>;
    type IntoIter = AstSeparatedListNodesIterator<Language, Pat>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl IntoIterator for &ListPatElements {
    type Item = SyntaxResult<Pat>;
    type IntoIter = AstSeparatedListNodesIterator<Language, Pat>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct MatchCaseList {
    syntax_list: SyntaxList,
}
impl MatchCaseList {
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
impl AstNode for MatchCaseList {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(MATCH_CASE_LIST as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == MATCH_CASE_LIST
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
impl Serialize for MatchCaseList {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.len()))?;
        for e in self.iter() {
            seq.serialize_element(&e)?;
        }
        seq.end()
    }
}
impl AstSeparatedList for MatchCaseList {
    type Language = Language;
    type Node = MatchCase;
    fn syntax_list(&self) -> &SyntaxList {
        &self.syntax_list
    }
    fn into_syntax_list(self) -> SyntaxList {
        self.syntax_list
    }
}
impl Debug for MatchCaseList {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("MatchCaseList ")?;
        f.debug_list().entries(self.elements()).finish()
    }
}
impl IntoIterator for MatchCaseList {
    type Item = SyntaxResult<MatchCase>;
    type IntoIter = AstSeparatedListNodesIterator<Language, MatchCase>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl IntoIterator for &MatchCaseList {
    type Item = SyntaxResult<MatchCase>;
    type IntoIter = AstSeparatedListNodesIterator<Language, MatchCase>;
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
        let mut seq = serializer.serialize_seq(Some(self.len()))?;
        for e in self.iter() {
            seq.serialize_element(&e)?;
        }
        seq.end()
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
pub struct RecordFields {
    syntax_list: SyntaxList,
}
impl RecordFields {
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
impl AstNode for RecordFields {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(RECORD_FIELDS as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == RECORD_FIELDS
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
impl Serialize for RecordFields {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.len()))?;
        for e in self.iter() {
            seq.serialize_element(&e)?;
        }
        seq.end()
    }
}
impl AstSeparatedList for RecordFields {
    type Language = Language;
    type Node = NamePatField;
    fn syntax_list(&self) -> &SyntaxList {
        &self.syntax_list
    }
    fn into_syntax_list(self) -> SyntaxList {
        self.syntax_list
    }
}
impl Debug for RecordFields {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("RecordFields ")?;
        f.debug_list().entries(self.elements()).finish()
    }
}
impl IntoIterator for RecordFields {
    type Item = SyntaxResult<NamePatField>;
    type IntoIter = AstSeparatedListNodesIterator<Language, NamePatField>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl IntoIterator for &RecordFields {
    type Item = SyntaxResult<NamePatField>;
    type IntoIter = AstSeparatedListNodesIterator<Language, NamePatField>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct TupleExpr {
    syntax_list: SyntaxList,
}
impl TupleExpr {
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
impl AstNode for TupleExpr {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(TUPLE_EXPR as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == TUPLE_EXPR
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
impl Serialize for TupleExpr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.len()))?;
        for e in self.iter() {
            seq.serialize_element(&e)?;
        }
        seq.end()
    }
}
impl AstSeparatedList for TupleExpr {
    type Language = Language;
    type Node = Expr;
    fn syntax_list(&self) -> &SyntaxList {
        &self.syntax_list
    }
    fn into_syntax_list(self) -> SyntaxList {
        self.syntax_list
    }
}
impl Debug for TupleExpr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("TupleExpr ")?;
        f.debug_list().entries(self.elements()).finish()
    }
}
impl IntoIterator for TupleExpr {
    type Item = SyntaxResult<Expr>;
    type IntoIter = AstSeparatedListNodesIterator<Language, Expr>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl IntoIterator for &TupleExpr {
    type Item = SyntaxResult<Expr>;
    type IntoIter = AstSeparatedListNodesIterator<Language, Expr>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct TuplePat {
    syntax_list: SyntaxList,
}
impl TuplePat {
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
impl AstNode for TuplePat {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(TUPLE_PAT as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == TUPLE_PAT
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
impl Serialize for TuplePat {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.len()))?;
        for e in self.iter() {
            seq.serialize_element(&e)?;
        }
        seq.end()
    }
}
impl AstSeparatedList for TuplePat {
    type Language = Language;
    type Node = Pat;
    fn syntax_list(&self) -> &SyntaxList {
        &self.syntax_list
    }
    fn into_syntax_list(self) -> SyntaxList {
        self.syntax_list
    }
}
impl Debug for TuplePat {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("TuplePat ")?;
        f.debug_list().entries(self.elements()).finish()
    }
}
impl IntoIterator for TuplePat {
    type Item = SyntaxResult<Pat>;
    type IntoIter = AstSeparatedListNodesIterator<Language, Pat>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl IntoIterator for &TuplePat {
    type Item = SyntaxResult<Pat>;
    type IntoIter = AstSeparatedListNodesIterator<Language, Pat>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct TupleTy {
    syntax_list: SyntaxList,
}
impl TupleTy {
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
impl AstNode for TupleTy {
    type Language = Language;
    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(TUPLE_TY as u16));
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == TUPLE_TY
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
impl Serialize for TupleTy {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.len()))?;
        for e in self.iter() {
            seq.serialize_element(&e)?;
        }
        seq.end()
    }
}
impl AstSeparatedList for TupleTy {
    type Language = Language;
    type Node = Ty;
    fn syntax_list(&self) -> &SyntaxList {
        &self.syntax_list
    }
    fn into_syntax_list(self) -> SyntaxList {
        self.syntax_list
    }
}
impl Debug for TupleTy {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("TupleTy ")?;
        f.debug_list().entries(self.elements()).finish()
    }
}
impl IntoIterator for TupleTy {
    type Item = SyntaxResult<Ty>;
    type IntoIter = AstSeparatedListNodesIterator<Language, Ty>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl IntoIterator for &TupleTy {
    type Item = SyntaxResult<Ty>;
    type IntoIter = AstSeparatedListNodesIterator<Language, Ty>;
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
