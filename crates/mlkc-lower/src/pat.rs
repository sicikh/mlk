//! A pattern as a body wrote it.

use mlkc_hir_def::{Name, Pat};
use mlkc_syntax::Pat as PatSyntax;

use crate::syntax;

/// A pattern as it is written.
pub(crate) fn pat(pat: &PatSyntax) -> Pat {
    match pat {
        PatSyntax::IdentPat(ident) => Pat::Bind(syntax::name(ident.name())),
        PatSyntax::WildcardPat(_) => Pat::Wildcard,
        // A pattern the parser could not read binds nothing, which is what a missing pattern
        // means; a reader of the HIR does not have to care how it broke.
        PatSyntax::BogusPat(_) => Pat::Missing,
    }
}

/// The names a pattern binds, in the order they are written.
///
/// A pattern that binds no name --- the wildcard, or one the parser could not read --- binds
/// nothing, and so does a name that is not there: a body cannot refer to it.
pub(crate) fn bindings(pat: &Pat) -> Vec<Name> {
    match pat {
        Pat::Bind(name) if !name.is_missing() => vec![name.clone()],
        Pat::Bind(_) | Pat::Wildcard | Pat::Missing => Vec::new(),
    }
}
