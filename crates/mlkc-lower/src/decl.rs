//! What a declaration says about a function: its parameters, and the type of its result.

use mlkc_hir_def::{Name, ParamData, Signature, TypeRef, Visibility};
use mlkc_rowan::AstNode;
use mlkc_syntax::{AnyParameter, Attribute, AttributeList, FunDecl, Parameter, SyntaxToken};

use crate::{syntax, ty};

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
pub(crate) fn signature(decl: &FunDecl) -> Signature {
    Signature {
        params: parameters(decl).iter().map(parameter).collect(),
        ret: ret(decl),
    }
}

/// The type a function declares for its result.
fn ret(decl: &FunDecl) -> Option<TypeRef> {
    let annotation = decl.return_type_annotation()?;

    Some(ty::type_ref(annotation.return_type().ok()))
}

/// The parameters of a declaration, in the order they are written.
///
/// A parameter the parser could not read is one of them, and `None` stands for it: the
/// declaration wrote an argument list of this arity, and the name and the type of the one
/// that broke are not there. What a caller reads is therefore the arity the module wrote,
/// and a reader of the HIR sees that the parameter is broken rather than a function that
/// takes one argument fewer.
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

/// One parameter of a signature.
fn parameter(parameter: &Option<Parameter>) -> ParamData {
    let Some(parameter) = parameter else {
        return ParamData {
            name: Name::missing(),
            ty: None,
        };
    };

    let ty = parameter
        .type_annotation()
        .map(|annotation| ty::type_ref(annotation.ty().ok()));

    ParamData {
        name: syntax::name(parameter.name()),
        ty,
    }
}

/// The name a parameter is declared under, or a name that is not there.
pub(crate) fn parameter_name(parameter: &Option<Parameter>) -> Name {
    match parameter {
        Some(parameter) => syntax::name(parameter.name()),
        None => Name::missing(),
    }
}
