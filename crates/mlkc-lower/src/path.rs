//! A path as a module wrote it, and its segments.

use mlkc_hir_def::{Name, PathAnchor, PathData, PlainPath, TypeRef, path::PathSegmentData};
use mlkc_rowan::AstNode;
use mlkc_syntax::{Path, PathSegment, Type as TypeSyntax, TypeArgs};

use crate::{syntax, ty};

/// A path as it was written, with no resolution of its base.
///
/// What the base denotes is decided by the caller that knows more than the syntax does:
/// a body anchors a path to the binding it names, and the item tree's builder anchors what
/// is left against the names the module declares. A path that neither of them knows stays
/// [`PathAnchor::Unresolved`], and the stage that holds the scopes of the project resolves
/// it.
pub(crate) fn data(path: &Path) -> PathData {
    PathData {
        segments: segments(path),
        anchor: PathAnchor::Unresolved,
    }
}

/// A path as an interned plain path: the names it is made of, which is what an import and the
/// path a module declares itself as are written as.
///
/// A plain path carries no type arguments and no resolution: it is a path as a name of
/// something in the project, and what it names is what the stage that holds the scopes says.
pub(crate) fn plain(path: &Path) -> PlainPath {
    PlainPath::from_segments(segments(path).into_iter().map(|segment| segment.name))
}

/// A path of one segment: a name a body refers to, anchored by the caller.
pub(crate) fn ident(name: Name, anchor: PathAnchor) -> PathData {
    PathData::ident(name, anchor)
}

/// The segments of a path, in the order they are written.
///
/// A path is written as a qualifier and a segment, one inside the other, so the segments of
/// the whole path are the segments of its qualifier and then its own.
fn segments(path: &Path) -> Vec<PathSegmentData> {
    let mut segments = Vec::new();

    collect(path, &mut segments);

    segments
}

/// Collects the segments of a path, its qualifier first.
fn collect(path: &Path, segments: &mut Vec<PathSegmentData>) {
    if let Some(qualifier) = path.qualifier()
        && let Ok(prefix) = qualifier.path()
    {
        collect(&prefix, segments);
    }

    if let Ok(segment) = path.segment() {
        segments.push(segment_data(&segment));
    }
}

/// One segment of a path.
fn segment_data(segment: &PathSegment) -> PathSegmentData {
    PathSegmentData {
        name: syntax::name(segment.name()),
        args: arguments(segment.type_args()),
    }
}

/// The type arguments written at a segment.
///
/// A segment a type is written at is what a generic type is applied to; the types a mistake
/// left out are not arguments of it.
fn arguments(args: Option<TypeArgs>) -> Vec<TypeRef> {
    let Some(args) = args else {
        return Vec::new();
    };

    args.type_arg_list()
        .syntax()
        .children()
        .filter_map(TypeSyntax::cast)
        .map(ty::type_of)
        .collect()
}
