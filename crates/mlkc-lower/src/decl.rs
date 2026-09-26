//! What a declaration says about a function: its parameters, and the type of its result.

use mlkc_hir_def::{Name, ParamData, Pat, Signature, TypeRef, Visibility};
use mlkc_rowan::AstNode;
use mlkc_syntax::{AnyParameter, Attribute, AttributeList, FunDecl, Parameter, SyntaxToken};
use mlkc_vfs::FileId;

use crate::{LoweringDiag, pat, ty};

/// The visibility a declaration is written with.
///
/// The language writes `pub` in front of a declaration and nothing else: a declaration
/// without it is visible inside its module, which is what the default visibility is.
pub(crate) fn visibility(token: Option<SyntaxToken>) -> Visibility {
    if token.is_some() {
        Visibility::Public
    } else {
        Visibility::default()
    }
}

/// The attributes an attribute list carries, in the order they are written.
pub(crate) fn attributes(list: &AttributeList) -> Vec<Attribute> {
    list.syntax()
        .children()
        .filter_map(Attribute::cast)
        .collect()
}

/// The signature a function declares: what a caller reads.
///
/// A function that declares no return type has none here as well: the type of its result is
/// not written down, and inferring it is a pass of its own, which is why the HIR keeps "not
/// written" apart from the `_` a module writes to mean "inferred".
pub(crate) fn signature(
    decl: &FunDecl,
    file: FileId,
    diagnostics: &mut Vec<LoweringDiag>,
) -> Signature {
    let params = parameters(decl)
        .iter()
        .map(|parameter| parameter_of(parameter, file, diagnostics))
        .collect();

    Signature {
        params,
        ret: ret(decl, file, diagnostics),
    }
}

/// The type a function declares for its result.
fn ret(decl: &FunDecl, file: FileId, diagnostics: &mut Vec<LoweringDiag>) -> Option<TypeRef> {
    let annotation = decl.return_type_annotation()?;

    Some(ty::type_ref(
        annotation.return_type().ok(),
        file,
        diagnostics,
    ))
}

/// The parameters of a declaration, in the order they are written.
///
/// A parameter the parser could not read at all is one of them, and `None` stands for it: the
/// declaration wrote an argument list of this arity, and neither the pattern nor the type of
/// the one that broke is there. A parameter whose pattern alone broke is `Some`, and what
/// a caller reads of it is the type the module did write. What a caller reads is therefore
/// the arity the module wrote, and a reader of the HIR sees that the parameter is broken
/// rather than a function that takes one argument fewer.
pub(crate) fn parameters(decl: &FunDecl) -> Vec<Option<Parameter>> {
    let Ok(parameters) = decl.parameters() else {
        return Vec::new();
    };

    parameters
        .items()
        .syntax()
        .children()
        .map(|node| {
            match AnyParameter::cast(node) {
                Some(AnyParameter::Parameter(parameter)) => Some(parameter),
                Some(AnyParameter::BogusParameter(_)) | None => None,
            }
        })
        .collect()
}

/// One parameter of a signature: what a caller reads of it, which is its type.
///
/// What the parameter binds is not part of a signature: the pattern is the body's, and the
/// body of the function is where it is lowered.
fn parameter_of(
    parameter: &Option<Parameter>,
    file: FileId,
    diagnostics: &mut Vec<LoweringDiag>,
) -> ParamData {
    let ty = parameter.as_ref().and_then(|parameter| {
        parameter
            .type_annotation()
            .map(|annotation| ty::type_ref(annotation.ty().ok(), file, diagnostics))
    });

    ParamData { ty }
}

/// The pattern a parameter is written as, or a pattern that is not there.
///
/// A parameter is a pattern rather than a name: a function that ignores an argument writes
/// the wildcard where a function that uses it writes a name, and a parameter the parser could
/// not read is a pattern that is missing rather than a name that is.
pub(crate) fn pattern(parameter: &Option<Parameter>) -> Pat {
    match parameter {
        Some(parameter) => pat::at(parameter.pat().ok()),
        None => Pat::Missing,
    }
}

/// The name a parameter binds, if it binds one.
pub(crate) fn parameter_name(parameter: &Parameter) -> Option<Name> {
    parameter.pat().ok().and_then(|pattern| pat::name(&pattern))
}
