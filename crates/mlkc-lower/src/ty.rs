//! A type as a module wrote it.

use mlkc_hir_def::TypeRef;
use mlkc_syntax::Type as TypeSyntax;
use mlkc_vfs::FileId;

use crate::{LoweringDiag, path};

/// A type at a position that may hold none, or one the parser could not read.
pub(crate) fn type_ref(
    ty: Option<TypeSyntax>,
    file: FileId,
    diagnostics: &mut Vec<LoweringDiag>,
) -> TypeRef {
    match ty {
        Some(ty) => type_of(ty, file, diagnostics),
        None => TypeRef::Missing,
    }
}

/// A type as it is written.
pub(crate) fn type_of(
    ty: TypeSyntax,
    file: FileId,
    diagnostics: &mut Vec<LoweringDiag>,
) -> TypeRef {
    match ty {
        TypeSyntax::PathType(path_type) => {
            // A path the parser could not read is a type that is missing: a path of no
            // segments names nothing, and inventing one would be a lie.
            let Ok(path) = path_type.path() else {
                return TypeRef::Missing;
            };

            TypeRef::Path(path::data(&path, file, diagnostics))
        },
        TypeSyntax::InferType(_) => TypeRef::Infer,
        TypeSyntax::BogusType(_) => TypeRef::Missing,
    }
}
