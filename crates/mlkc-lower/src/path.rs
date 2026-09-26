//! A path as a module wrote it: where it starts, and the names it is made of.

use mlkc_hir_def::{
    Name, PathAnchor, PathData, PathRoot, PlainPath, TypeRef, path::PathSegmentData,
};
use mlkc_rowan::AstNode;
use mlkc_syntax::{Path, PathRoot as PathRootSyntax, PathSegment, Type as TypeSyntax, TypeArgs};
use mlkc_vfs::FileId;

use crate::{LoweringDiag, LoweringError, syntax, ty};

/// A path as it was written, with no resolution of its root.
///
/// What the root of a path denotes is decided by the caller that knows more than the syntax
/// does: a body anchors a path to the binding it names, and the item tree's builder anchors
/// what is left against the names the module declares. A path that neither of them knows
/// stays [`PathAnchor::Unresolved`], and the stage that holds the scopes of the project
/// resolves it --- unless it is rooted at the project, which is what the module knows of
/// itself without a manifest.
pub(crate) fn data(path: &Path, file: FileId, diagnostics: &mut Vec<LoweringDiag>) -> PathData {
    let root = root(path);
    let args = first_segment(path).and_then(|segment| segment.type_args());

    // A project is not a type: the keyword names the project the module is written in, and what
    // is applied to it is applied to nothing. What a root written as a name denotes is not
    // a module's to decide, and whether an argument belongs to it is what the stage that holds
    // the scopes reads.
    if matches!(root, PathRoot::Project)
        && let Some(args) = &args
    {
        let error = LoweringError::ProjectWithTypeArguments { path: plain(path) };
        let diagnostic = LoweringDiag::new(error, syntax::span(file, args.syntax()));

        diagnostics.push(diagnostic);
    }

    PathData {
        anchor: match root {
            PathRoot::Project => PathAnchor::Project,
            PathRoot::Named(_) => PathAnchor::Unresolved,
        },
        root,
        root_args: arguments(args, file, diagnostics),
        segments: names(path, file, diagnostics),
    }
}

/// A path as an interned plain path: where it starts, and the names it is made of, which is
/// what an import and the path a module declares itself as are written as.
///
/// A plain path carries no type arguments and no resolution: it is a path as a name of
/// something in the project, and what it names is what the stage that holds the scopes says.
pub(crate) fn plain(path: &Path) -> PlainPath {
    let mut segments = Vec::new();

    collect(path, &mut segments, &mut |segment| segment_name(segment));

    // The root is where the path starts, and a plain path holds it apart from the names: the
    // segments of the path are the root first, and the names of the plain path are what follows.
    PlainPath::from_root(root(path), segments.into_iter().skip(1))
}

/// The type arguments written at a path, if it carries any.
///
/// A path that names something of the project is a path of names: an argument written at a
/// segment is what a type of the language is applied to, and it is the first such argument
/// that a caller reports.
pub(crate) fn type_args(path: &Path) -> Option<TypeArgs> {
    // The qualifier comes first, so the arguments of a path are read from its innermost
    // segment outwards, which is the order they are written in.
    if let Some(qualifier) = path.qualifier()
        && let Ok(prefix) = qualifier.path()
        && let Some(args) = type_args(&prefix)
    {
        return Some(args);
    }

    path.segment().ok().and_then(|segment| segment.type_args())
}

/// Where a path starts: the project the module is written in, or the name it starts at.
///
/// The root is the first segment of the path, and a path the parser could not read has none:
/// where such a path starts is a name that is not there.
fn root(path: &Path) -> PathRoot {
    match first_segment(path).and_then(|segment| segment.root().ok()) {
        Some(PathRootSyntax::Project(_)) => PathRoot::Project,
        Some(PathRootSyntax::Name(name)) => PathRoot::Named(syntax::name_of(&name)),
        None => PathRoot::Named(Name::missing()),
    }
}

/// The first segment of a path.
///
/// A path is written as a qualifier and a segment, one inside the other, so the first segment
/// of a path is the segment of its innermost qualifier.
fn first_segment(path: &Path) -> Option<PathSegment> {
    match path.qualifier().and_then(|qualifier| qualifier.path().ok()) {
        Some(prefix) => first_segment(&prefix),
        None => path.segment().ok(),
    }
}

/// The names of a path after its root, in the order they are written.
fn names(path: &Path, file: FileId, diagnostics: &mut Vec<LoweringDiag>) -> Vec<PathSegmentData> {
    segments(path, file, diagnostics)
        .into_iter()
        .skip(1)
        .collect()
}

/// The segments of a path, the root first.
///
/// A path is written as a qualifier and a segment, one inside the other, so the segments of
/// the whole path are the segments of its qualifier and then its own.
fn segments(
    path: &Path,
    file: FileId,
    diagnostics: &mut Vec<LoweringDiag>,
) -> Vec<PathSegmentData> {
    let mut segments = Vec::new();

    collect(path, &mut segments, &mut |segment| {
        segment_data(segment, file, diagnostics)
    });

    segments
}

/// Collects the segments of a path, its qualifier first, as `read` makes of them.
fn collect<T>(path: &Path, segments: &mut Vec<T>, read: &mut impl FnMut(&PathSegment) -> T) {
    if let Some(qualifier) = path.qualifier()
        && let Ok(prefix) = qualifier.path()
    {
        collect(&prefix, segments, read);
    }

    if let Ok(segment) = path.segment() {
        segments.push(read(&segment));
    }
}

/// One segment of a path.
fn segment_data(
    segment: &PathSegment,
    file: FileId,
    diagnostics: &mut Vec<LoweringDiag>,
) -> PathSegmentData {
    PathSegmentData {
        name: segment_name(segment),
        args: arguments(segment.type_args(), file, diagnostics),
    }
}

/// The name a segment is written as, or a name that is not there.
///
/// A segment that is written as the project is a root rather than a name, and a path that is
/// rooted there has no name of its own: what is left to a caller is the keyword itself.
fn segment_name(segment: &PathSegment) -> Name {
    match segment.root().ok().as_ref() {
        Some(PathRootSyntax::Name(name)) => syntax::name_of(name),
        _ => Name::missing(),
    }
}

/// The type arguments written at a segment.
///
/// A segment a type is written at is what a generic type is applied to; the types a mistake
/// left out are not arguments of it.
fn arguments(
    args: Option<TypeArgs>,
    file: FileId,
    diagnostics: &mut Vec<LoweringDiag>,
) -> Vec<TypeRef> {
    let Some(args) = args else {
        return Vec::new();
    };

    args.type_arg_list()
        .syntax()
        .children()
        .filter_map(TypeSyntax::cast)
        .map(|ty| ty::type_of(ty, file, diagnostics))
        .collect()
}
