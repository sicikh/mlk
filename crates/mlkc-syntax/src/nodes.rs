use mlkc_rowan::{
    AstNode, AstNodeList, AstNodeListIterator, RawSyntaxKind, SyntaxKindSet, SyntaxResult, support,
};
use serde::{Serialize, Serializer, ser::SerializeSeq};

use crate::{
    MlkLanguage as Language,
    MlkSyntaxKind::{self as SyntaxKind, *},
    SyntaxElement, SyntaxElementChildren, SyntaxList, SyntaxNode, SyntaxToken,
};

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct NumberLiteralAtom {
    pub(crate) syntax: SyntaxNode,
}

impl NumberLiteralAtom {
    /// Create an AstNode from a SyntaxNode without checking its kind
    ///
    /// ## Safety
    ///
    /// This function must be guarded with a call to [AstNode::can_cast]
    /// or a match on [SyntaxNode::kind]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> NumberLiteralAtom {
        NumberLiteralAtom { syntax }
    }

    pub fn value_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 0usize)
    }

    pub fn as_fields(&self) -> NumberLiteralAtomFields {
        NumberLiteralAtomFields {
            value_token: self.value_token(),
        }
    }
}

#[derive(Serialize)]
pub struct NumberLiteralAtomFields {
    pub value_token: SyntaxResult<SyntaxToken>,
}

impl AstNode for NumberLiteralAtom {
    type Language = Language;

    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(NUMBER_LITERAL_ATOM as u16));

    fn can_cast(kind: SyntaxKind) -> bool {
        kind == NUMBER_LITERAL_ATOM
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

impl From<NumberLiteralAtom> for SyntaxNode {
    fn from(n: NumberLiteralAtom) -> Self {
        n.syntax
    }
}

impl From<NumberLiteralAtom> for SyntaxElement {
    fn from(n: NumberLiteralAtom) -> Self {
        n.syntax.into()
    }
}

impl std::fmt::Debug for NumberLiteralAtom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH: std::cell::Cell<u8> = const { std ::cell::Cell::new(0) } }
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("NumberLiteralAtom")
                .field(
                    "value_token",
                    &support::DebugSyntaxResult(self.value_token()),
                )
                .finish()
        } else {
            f.debug_struct("NumberLiteralAtom").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}

impl Serialize for NumberLiteralAtom {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_fields().serialize(serializer)
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct BooleanLiteralAtom {
    pub(crate) syntax: SyntaxNode,
}

#[derive(Serialize)]
pub struct BooleanLiteralAtomFields {
    pub value_token: SyntaxResult<SyntaxToken>,
}

impl BooleanLiteralAtom {
    /// Create an AstNode from a SyntaxNode without checking its kind
    ///
    /// ## Safety
    ///
    /// This function must be guarded with a call to [AstNode::can_cast]
    /// or a match on [SyntaxNode::kind]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> BooleanLiteralAtom {
        BooleanLiteralAtom { syntax }
    }

    pub fn value_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 0usize)
    }

    pub fn as_fields(&self) -> BooleanLiteralAtomFields {
        BooleanLiteralAtomFields {
            value_token: self.value_token(),
        }
    }
}

impl AstNode for BooleanLiteralAtom {
    type Language = Language;

    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(BOOLEAN_LITERAL_ATOM as u16));

    fn can_cast(kind: SyntaxKind) -> bool {
        kind == BOOLEAN_LITERAL_ATOM
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

impl From<BooleanLiteralAtom> for SyntaxNode {
    fn from(n: BooleanLiteralAtom) -> Self {
        n.syntax
    }
}

impl From<BooleanLiteralAtom> for SyntaxElement {
    fn from(n: BooleanLiteralAtom) -> Self {
        n.syntax.into()
    }
}

impl std::fmt::Debug for BooleanLiteralAtom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH: std::cell::Cell<u8> = const { std ::cell::Cell::new(0) } }
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("BooleanLiteralAtom")
                .field(
                    "value_token",
                    &support::DebugSyntaxResult(self.value_token()),
                )
                .finish()
        } else {
            f.debug_struct("BooleanLiteralAtom").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}

impl Serialize for BooleanLiteralAtom {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_fields().serialize(serializer)
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct StringLiteralAtom {
    pub(crate) syntax: SyntaxNode,
}

#[derive(Serialize)]
pub struct StringLiteralAtomFields {
    pub value_token: SyntaxResult<SyntaxToken>,
}

impl StringLiteralAtom {
    /// Create an AstNode from a SyntaxNode without checking its kind
    ///
    /// ## Safety
    ///
    /// This function must be guarded with a call to [AstNode::can_cast]
    /// or a match on [SyntaxNode::kind]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> StringLiteralAtom {
        StringLiteralAtom { syntax }
    }

    pub fn value_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 0usize)
    }

    pub fn as_fields(&self) -> StringLiteralAtomFields {
        StringLiteralAtomFields {
            value_token: self.value_token(),
        }
    }
}

impl Serialize for StringLiteralAtom {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_fields().serialize(serializer)
    }
}

impl AstNode for StringLiteralAtom {
    type Language = Language;

    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(NUMBER_LITERAL as u16));

    fn can_cast(kind: SyntaxKind) -> bool {
        kind == NUMBER_LITERAL
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

impl From<StringLiteralAtom> for SyntaxNode {
    fn from(n: StringLiteralAtom) -> Self {
        n.syntax
    }
}

impl From<StringLiteralAtom> for SyntaxElement {
    fn from(n: StringLiteralAtom) -> Self {
        n.syntax.into()
    }
}

impl std::fmt::Debug for StringLiteralAtom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH: std::cell::Cell<u8> = const { std ::cell::Cell::new(0) } }
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("StringLiteralAtom")
                .field(
                    "value_token",
                    &support::DebugSyntaxResult(self.value_token()),
                )
                .finish()
        } else {
            f.debug_struct("StringLiteralAtom").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct SymbolAtom {
    pub(crate) syntax: SyntaxNode,
}

#[derive(Serialize)]
pub struct SymbolAtomFields {
    pub value_token: SyntaxResult<SyntaxToken>,
}

impl SymbolAtom {
    /// Create an AstNode from a SyntaxNode without checking its kind
    ///
    /// ## Safety
    ///
    /// This function must be guarded with a call to [AstNode::can_cast]
    /// or a match on [SyntaxNode::kind]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> SymbolAtom {
        SymbolAtom { syntax }
    }

    pub fn value_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 0usize)
    }

    pub fn as_fields(&self) -> SymbolAtomFields {
        SymbolAtomFields {
            value_token: self.value_token(),
        }
    }
}

impl AstNode for SymbolAtom {
    type Language = Language;

    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(NUMBER_LITERAL as u16));

    fn can_cast(kind: SyntaxKind) -> bool {
        kind == NUMBER_LITERAL
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

impl From<SymbolAtom> for SyntaxNode {
    fn from(n: SymbolAtom) -> Self {
        n.syntax
    }
}

impl From<SymbolAtom> for SyntaxElement {
    fn from(n: SymbolAtom) -> Self {
        n.syntax.into()
    }
}

impl std::fmt::Debug for SymbolAtom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH: std::cell::Cell<u8> = const { std ::cell::Cell::new(0) } }
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("SymbolAtom")
                .field(
                    "value_token",
                    &support::DebugSyntaxResult(self.value_token()),
                )
                .finish()
        } else {
            f.debug_struct("SymbolAtom").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}

impl Serialize for SymbolAtom {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_fields().serialize(serializer)
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Serialize)]
pub enum AtomExpr {
    NumberLiteral(NumberLiteralAtom),
    BooleanLiteral(BooleanLiteralAtom),
    StringLiteral(StringLiteralAtom),
    Symbol(SymbolAtom),
}

impl AtomExpr {
    pub fn as_number(&self) -> Option<&NumberLiteralAtom> {
        match &self {
            AtomExpr::NumberLiteral(n) => Some(n),
            _ => None,
        }
    }

    pub fn as_boolean(&self) -> Option<&BooleanLiteralAtom> {
        match &self {
            AtomExpr::BooleanLiteral(b) => Some(b),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<&StringLiteralAtom> {
        match &self {
            AtomExpr::StringLiteral(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_symbol(&self) -> Option<&SymbolAtom> {
        match &self {
            AtomExpr::Symbol(s) => Some(s),
            _ => None,
        }
    }
}

impl From<NumberLiteralAtom> for AtomExpr {
    fn from(n: NumberLiteralAtom) -> Self {
        AtomExpr::NumberLiteral(n)
    }
}

impl From<BooleanLiteralAtom> for AtomExpr {
    fn from(b: BooleanLiteralAtom) -> Self {
        AtomExpr::BooleanLiteral(b)
    }
}

impl From<StringLiteralAtom> for AtomExpr {
    fn from(s: StringLiteralAtom) -> Self {
        AtomExpr::StringLiteral(s)
    }
}

impl From<SymbolAtom> for AtomExpr {
    fn from(n: SymbolAtom) -> Self {
        AtomExpr::Symbol(n)
    }
}

impl AstNode for AtomExpr {
    type Language = Language;

    const KIND_SET: SyntaxKindSet<Self::Language> = NumberLiteralAtom::KIND_SET
        .union(BooleanLiteralAtom::KIND_SET)
        .union(StringLiteralAtom::KIND_SET)
        .union(SymbolAtom::KIND_SET);

    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(
            kind,
            NUMBER_LITERAL_ATOM | BOOLEAN_LITERAL_ATOM | STRING_LITERAL_ATOM | SYMBOL_ATOM
        )
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        let res = match syntax.kind() {
            NUMBER_LITERAL_ATOM => Self::NumberLiteral(NumberLiteralAtom { syntax }),
            BOOLEAN_LITERAL_ATOM => Self::BooleanLiteral(BooleanLiteralAtom { syntax }),
            STRING_LITERAL_ATOM => Self::StringLiteral(StringLiteralAtom { syntax }),
            SYMBOL_ATOM => Self::Symbol(SymbolAtom { syntax }),
            _ => return None,
        };
        Some(res)
    }

    fn syntax(&self) -> &SyntaxNode {
        match self {
            AtomExpr::NumberLiteral(n) => n.syntax(),
            AtomExpr::BooleanLiteral(b) => b.syntax(),
            AtomExpr::StringLiteral(s) => s.syntax(),
            AtomExpr::Symbol(s) => s.syntax(),
        }
    }

    fn into_syntax(self) -> SyntaxNode {
        match self {
            AtomExpr::NumberLiteral(n) => n.into_syntax(),
            AtomExpr::BooleanLiteral(b) => b.into_syntax(),
            AtomExpr::StringLiteral(s) => s.into_syntax(),
            AtomExpr::Symbol(s) => s.into_syntax(),
        }
    }
}

impl std::fmt::Debug for AtomExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AtomExpr::NumberLiteral(n) => n.fmt(f),
            AtomExpr::BooleanLiteral(b) => b.fmt(f),
            AtomExpr::StringLiteral(s) => s.fmt(f),
            AtomExpr::Symbol(s) => s.fmt(f),
        }
    }
}

impl From<AtomExpr> for SyntaxNode {
    fn from(a: AtomExpr) -> Self {
        a.into_syntax()
    }
}

impl From<AtomExpr> for SyntaxElement {
    fn from(a: AtomExpr) -> Self {
        a.into_syntax().into()
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ExprList {
    syntax_list: SyntaxList,
}

impl ExprList {
    /// Create an AstNode from a SyntaxNode without checking its kind
    ///
    /// ## Safety
    ///
    /// This function must be guarded with a call to [AstNode::can_cast]
    /// or a match on [SyntaxNode::kind]
    pub unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self {
            syntax_list: syntax.into_list(),
        }
    }
}

impl AstNode for ExprList {
    type Language = Language;

    const KIND_SET: SyntaxKindSet<Self::Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(EXPR_LIST as u16));

    fn can_cast(kind: SyntaxKind) -> bool {
        kind == EXPR_LIST
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

impl AstNodeList for ExprList {
    type Language = Language;
    type Node = Expr;

    fn syntax_list(&self) -> &SyntaxList {
        &self.syntax_list
    }

    fn into_syntax_list(self) -> SyntaxList {
        self.syntax_list
    }
}

impl std::fmt::Debug for ExprList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("List ")?;
        f.debug_list().entries(self.iter()).finish()
    }
}

impl IntoIterator for ExprList {
    type IntoIter = AstNodeListIterator<Language, Expr>;
    type Item = Expr;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl IntoIterator for &ExprList {
    type IntoIter = AstNodeListIterator<Language, Expr>;
    type Item = Expr;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl Serialize for ExprList {
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

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ListExpr {
    syntax: SyntaxNode,
}

#[derive(Serialize)]
pub struct ListExprFields {
    l_paren: SyntaxResult<SyntaxToken>,
    expr_list: ExprList,
    r_paren: SyntaxResult<SyntaxToken>,
}

impl ListExpr {
    /// Create an AstNode from a SyntaxNode without checking its kind
    ///
    /// ## Safety
    ///
    /// This function must be guarded with a call to [AstNode::can_cast]
    /// or a match on [SyntaxNode::kind]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }

    pub fn l_paren(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 0usize)
    }

    pub fn inner(&self) -> ExprList {
        support::list(&self.syntax, 1usize)
    }

    pub fn r_paren(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 2usize)
    }

    pub fn as_fields(&self) -> ListExprFields {
        ListExprFields {
            l_paren: self.l_paren(),
            expr_list: self.inner(),
            r_paren: self.r_paren(),
        }
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
            Some(ListExpr { syntax })
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

impl From<ListExpr> for SyntaxNode {
    fn from(l: ListExpr) -> Self {
        l.syntax
    }
}

impl From<ListExpr> for SyntaxElement {
    fn from(l: ListExpr) -> Self {
        l.syntax.into()
    }
}

impl std::fmt::Debug for ListExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH: std::cell::Cell<u8> = const { std ::cell::Cell::new(0) } }
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("ListExpr")
                .field("l_paren", &support::DebugSyntaxResult(self.l_paren()))
                .field("expr_list", &self.inner())
                .field("r_paren", &support::DebugSyntaxResult(self.r_paren()))
                .finish()
        } else {
            f.debug_struct("ListExpr").finish()
        };
        DEPTH.set(current_depth);
        result
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

#[derive(Clone, PartialEq, Eq, Hash, Serialize)]
pub struct BogusExpr {
    syntax: SyntaxNode,
}

impl BogusExpr {
    /// Create an AstNode from a SyntaxNode without checking its kind
    ///
    /// ## Safety
    ///
    /// This function must be guarded with a call to [AstNode::can_cast]
    /// or a match on [SyntaxNode::kind]
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
            Some(BogusExpr { syntax })
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
    fn from(b: BogusExpr) -> Self {
        b.syntax
    }
}

impl From<BogusExpr> for SyntaxElement {
    fn from(b: BogusExpr) -> Self {
        b.syntax.into()
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Serialize)]
pub enum Expr {
    Atom(AtomExpr),
    List(ListExpr),
    Bogus(BogusExpr),
}

impl Expr {
    pub const fn as_atom(&self) -> Option<&AtomExpr> {
        match self {
            Expr::Atom(a) => Some(a),
            _ => None,
        }
    }

    pub const fn as_list(&self) -> Option<&ListExpr> {
        match self {
            Expr::List(l) => Some(l),
            _ => None,
        }
    }
}

impl AstNode for Expr {
    type Language = Language;

    const KIND_SET: SyntaxKindSet<Language> = AtomExpr::KIND_SET.union(ListExpr::KIND_SET);

    fn can_cast(kind: SyntaxKind) -> bool {
        match kind {
            LIST_EXPR | BOGUS_EXPR => true,
            _ if AtomExpr::can_cast(kind) => true,
            _ => false,
        }
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        let res = match syntax.kind() {
            LIST_EXPR => Self::List(ListExpr::cast(syntax)?),
            BOGUS_EXPR => Self::Bogus(BogusExpr::cast(syntax)?),
            _ => {
                if let Some(atom) = AtomExpr::cast(syntax) {
                    return Some(Self::Atom(atom));
                }
                return None;
            },
        };

        Some(res)
    }

    fn syntax(&self) -> &SyntaxNode {
        match self {
            Expr::Atom(a) => a.syntax(),
            Expr::List(l) => l.syntax(),
            Expr::Bogus(b) => b.syntax(),
        }
    }

    fn into_syntax(self) -> SyntaxNode {
        match self {
            Expr::Atom(a) => a.into_syntax(),
            Expr::List(l) => l.into_syntax(),
            Expr::Bogus(b) => b.into_syntax(),
        }
    }
}

impl std::fmt::Debug for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::Atom(a) => a.fmt(f),
            Expr::List(l) => l.fmt(f),
            Expr::Bogus(b) => b.fmt(f),
        }
    }
}

impl From<AtomExpr> for Expr {
    fn from(a: AtomExpr) -> Self {
        Expr::Atom(a)
    }
}

impl From<ListExpr> for Expr {
    fn from(l: ListExpr) -> Self {
        Expr::List(l)
    }
}

impl From<Expr> for SyntaxNode {
    fn from(e: Expr) -> Self {
        e.into_syntax()
    }
}

impl From<Expr> for SyntaxElement {
    fn from(e: Expr) -> Self {
        e.into_syntax().into()
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct RootModule {
    pub(crate) syntax: SyntaxNode,
}

pub struct RootModuleFields {
    pub items: ExprList,
    pub eof_token: SyntaxResult<SyntaxToken>,
}

impl RootModule {
    /// Create an AstNode from a SyntaxNode without checking its kind
    ///
    /// ## Safety
    ///
    /// This function must be guarded with a call to [AstNode::can_cast]
    /// or a match on [SyntaxNode::kind]
    pub const unsafe fn new_unchecked(syntax: SyntaxNode) -> Self {
        Self { syntax }
    }

    pub fn items(&self) -> ExprList {
        support::list(&self.syntax, 0usize)
    }

    pub fn eof_token(&self) -> SyntaxResult<SyntaxToken> {
        support::required_token(&self.syntax, 1usize)
    }

    pub fn as_fields(&self) -> RootModuleFields {
        RootModuleFields {
            items: self.items(),
            eof_token: self.eof_token(),
        }
    }
}

impl AstNode for RootModule {
    type Language = Language;

    const KIND_SET: SyntaxKindSet<Language> =
        SyntaxKindSet::from_raw(RawSyntaxKind(ROOT_MODULE as u16));

    fn can_cast(kind: SyntaxKind) -> bool {
        kind == ROOT_MODULE
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

impl std::fmt::Debug for RootModule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        thread_local! { static DEPTH: std::cell::Cell<u8> = const { std ::cell::Cell::new(0) } }
        let current_depth = DEPTH.get();
        let result = if current_depth < 16 {
            DEPTH.set(current_depth + 1);
            f.debug_struct("RootModule")
                .field("items", &self.items())
                .field("eof_token", &support::DebugSyntaxResult(self.eof_token()))
                .finish()
        } else {
            f.debug_struct("RootModule").finish()
        };
        DEPTH.set(current_depth);
        result
    }
}

impl From<RootModule> for SyntaxNode {
    fn from(value: RootModule) -> Self {
        value.into_syntax()
    }
}

impl From<RootModule> for SyntaxElement {
    fn from(value: RootModule) -> Self {
        value.into_syntax().into()
    }
}

#[derive(Clone)]
pub struct DebugSyntaxElementChildren(pub SyntaxElementChildren);
impl std::fmt::Debug for DebugSyntaxElementChildren {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list()
            .entries(self.clone().0.map(DebugSyntaxElement))
            .finish()
    }
}

struct DebugSyntaxElement(SyntaxElement);
impl std::fmt::Debug for DebugSyntaxElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            SyntaxElement::Node(node) => std::fmt::Debug::fmt(node, f),
            SyntaxElement::Token(token) => std::fmt::Debug::fmt(token, f),
        }
    }
}
