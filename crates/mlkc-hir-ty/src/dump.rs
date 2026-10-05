//! A reading of the values of this crate, for a person and for a diff.
//!
//! The dump of a value is a text a snapshot test compares ([ADR-0006]). It is a function of the
//! value alone: no store, no item tree, no file. A class reads as the name it was declared
//! under, and a reader that needs to know which module it came from reads the ids, not the text.
//!
//! [ADR-0006]: ../../docs/adr/0006-snapshot-testing.md

use std::fmt::Write as _;

use mlkc_hir_def::ItemLocLike;

use crate::{checked::CheckedBody, module_types::ModuleTypes, ty::Ty};

/// The surface of a module, as text, one entity per line: `fun double: (Int) -> Int`.
pub fn module_types(types: &ModuleTypes) -> String {
    let mut out = String::from("MODULE TYPES");

    for (entity, ty) in types.iter() {
        let _ = write!(out, "\n  {} ", entity.item.kind().keyword());

        match entity.item.name() {
            Some(name) => {
                let _ = write!(out, "{name}");
            },
            None => {
                let _ = write!(out, "{:?}", entity.item);
            },
        }

        let _ = write!(out, ": {ty}");
    }

    out.push('\n');
    out
}

/// The types of one checked body, as text, in the order of the positions of the body.
///
/// A position is the label the HIR gives a node of the body: the same node reads under the same
/// number in the dump of the body and in the dump of the types of its nodes.
pub fn checked_body(body: &CheckedBody) -> String {
    let mut out = String::from("CHECKED BODY");

    let mut exprs: Vec<_> = body.expr_types().collect();
    exprs.sort_by_key(|(id, _)| *id);

    for (id, ty) in exprs {
        // An id counts from one, and the label a person reads a node by counts from zero, as the
        // HIR dump counts.
        let _ = write!(out, "\n  expr #{}: {ty}", id.index());
    }

    let mut pats: Vec<_> = body.pat_types().collect();
    pats.sort_by_key(|(id, _)| *id);

    for (id, ty) in pats {
        let _ = write!(out, "\n  pat #{}: {ty}", id.index());
    }

    let mut locals: Vec<_> = body.local_types().collect();
    locals.sort_by_key(|(id, _)| *id);

    for (id, ty) in locals {
        let _ = write!(out, "\n  local {id:?}: {ty}");
    }

    out.push('\n');
    out
}

/// One type, as text.
pub fn ty(ty: &Ty) -> String {
    ty.to_string()
}

#[cfg(test)]
mod tests {
    use mlkc_hir_def::{BodyBuilder, Expr, Literal, LocalScope, Pat};

    use super::{CheckedBody, Ty, checked_body};

    #[test]
    fn a_checked_body_reads_in_the_order_of_the_positions_of_the_body() {
        let mut builder = BodyBuilder::new();
        let first = builder.alloc_expr(Expr::Missing);
        let second = builder.alloc_expr(Expr::Literal(Literal::Int(1)));
        let pat = builder.alloc_pat(Pat::Wildcard);
        builder.set_root(second);
        let _ = builder.finish(&LocalScope::default());

        let mut checked = CheckedBody::new();
        // Recorded out of order: the dump is what puts them in the order of the body.
        checked.set_expr_type(second, Ty::Error);
        checked.set_expr_type(first, Ty::Error);
        checked.set_pat_type(pat, Ty::Error);

        assert_eq!(
            checked_body(&checked),
            "CHECKED BODY\n  expr #0: {error}\n  expr #1: {error}\n  pat #0: {error}\n",
        );
        assert_eq!(checked.expr_type(first), Some(&Ty::Error));
        assert_eq!(checked.pat_type(pat), Some(&Ty::Error));
    }
}
