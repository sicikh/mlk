//! The types of one checked body.

use mlkc_hir_def::{ExprId, LocalDefId, PatId};
use rustc_hash::FxHashMap;

use crate::ty::Ty;

/// What checking one body left behind: the type of every node of it.
///
/// The keys are positions inside the body, which is the natural handle inside a value that is
/// rebuilt whole ([ADR-0010]). The types are the same self-contained values everywhere else:
/// a reader of a body reads an expression's type without the state that computed it.
///
/// [ADR-0010]: ../../docs/adr/0010-stable-entity-identity.md
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CheckedBody {
    /// The type of every expression of the body.
    expr_types: FxHashMap<ExprId, Ty>,
    /// The type of every pattern of the body.
    pat_types: FxHashMap<PatId, Ty>,
    /// The type of every entity declared inside the body.
    local_types: FxHashMap<LocalDefId, Ty>,
}

impl CheckedBody {
    /// A body whose nodes have no types yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records the type of an expression, replacing what it had.
    pub fn set_expr_type(&mut self, expr: ExprId, ty: Ty) -> Option<Ty> {
        self.expr_types.insert(expr, ty)
    }

    /// Records the type of a pattern, replacing what it had.
    pub fn set_pat_type(&mut self, pat: PatId, ty: Ty) -> Option<Ty> {
        self.pat_types.insert(pat, ty)
    }

    /// Records the type of an entity declared inside the body.
    pub fn set_local_type(&mut self, local: LocalDefId, ty: Ty) -> Option<Ty> {
        self.local_types.insert(local, ty)
    }

    /// The type of an expression, if the check recorded one.
    pub fn expr_type(&self, expr: ExprId) -> Option<&Ty> {
        self.expr_types.get(&expr)
    }

    /// The type of a pattern, if the check recorded one.
    pub fn pat_type(&self, pat: PatId) -> Option<&Ty> {
        self.pat_types.get(&pat)
    }

    /// The type of an entity declared inside the body, if the check recorded one.
    pub fn local_type(&self, local: LocalDefId) -> Option<&Ty> {
        self.local_types.get(&local)
    }

    /// The types of the expressions of the body. The order is not the order of the source:
    /// a reader that wants one sorts the ids ([`dump`](crate::dump)).
    pub fn expr_types(&self) -> impl Iterator<Item = (ExprId, &Ty)> + '_ {
        self.expr_types.iter().map(|(id, ty)| (*id, ty))
    }

    /// The types of the patterns of the body.
    pub fn pat_types(&self) -> impl Iterator<Item = (PatId, &Ty)> + '_ {
        self.pat_types.iter().map(|(id, ty)| (*id, ty))
    }

    /// The types of the entities declared inside the body.
    pub fn local_types(&self) -> impl Iterator<Item = (LocalDefId, &Ty)> + '_ {
        self.local_types.iter().map(|(id, ty)| (*id, ty))
    }
}
