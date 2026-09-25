//! A type as a module wrote it.

use mlkc_hir_def::TypeRef;
use mlkc_syntax::Type as TypeSyntax;

use crate::path;

/// A type at a position that may hold none, or one the parser could not read.
pub(crate) fn type_ref(ty: Option<TypeSyntax>) -> TypeRef {
    match ty {
        Some(ty) => type_of(ty),
        None => TypeRef::Missing,
    }
}

/// A type as it is written.
pub(crate) fn type_of(ty: TypeSyntax) -> TypeRef {
    match ty {
        TypeSyntax::PathType(path_type) => {
            // A path the parser could not read is a type that is missing: a path of no
            // segments names nothing, and inventing one would be a lie.
            let Ok(path) = path_type.path() else {
                return TypeRef::Missing;
            };

            TypeRef::Path(path::data(&path))
        },
        TypeSyntax::InferType(_) => TypeRef::Infer,
        TypeSyntax::BogusType(_) => TypeRef::Missing,
    }
}
