//! Resolving a written path or type into the entity or the type it denotes.
//!
//! The names of a path are read where the module wrote them: an entity of the module, an entry
//! of its import table, or a type variable of the containing entity. What goes beyond the module
//! is walked with the walk of [ADR-0016] over the closure the stage was handed --- the
//! interfaces of the modules the paths reach, and the indexes of their projects --- so a path
//! may name a module of the project, a module of another project, or a name a module re-exports,
//! and a name after a module is read inside it.
//!
//! A path of the *surface* of a module that does not resolve was reported by the resolution of
//! the module, which walked the same paths. A path of a *body* is walked by the resolver with
//! [`Walk::entity_of`]: what a name denotes is the resolver's to find, and what the check makes
//! of the answer --- a type, and the check's own words for a name that denotes nothing --- is
//! this module's.
//!
//! [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md

use mlkc_hir_def::{ClassLoc, EntityLoc, ModuleId, Namespace, PathAnchor, PathData, TypeRef};
use mlkc_hir_ty::Ty;
use mlkc_resolve::{Resolution, ResolveError, Walk};

use crate::{check::CheckDeps, diagnostic::TypeError};

/// The type a written type denotes, and what could not be resolved.
pub(crate) type ResolvedType = (Ty, Vec<TypeError>);

/// The entity a path denotes, and what could not be resolved.
pub(crate) type ResolvedEntity = (Option<EntityLoc>, Vec<TypeError>);

/// What resolves the paths of one module for the check.
///
/// The anchors of a path and the walk over the closure are the resolver's: [`Walk::entity_of`]
/// answers the same for the check and for the MIR lowering. What this type adds is the check's
/// half --- a written type read as a [`Ty`], and the check's words for what a path denotes
/// ([`entity_of`](Self::entity_of)).
pub(crate) struct PathResolver<'a> {
    /// The module whose paths are resolved.
    module: ModuleId,
    /// What each name of the module denotes, and what each import resolved to.
    resolution: &'a Resolution,
    /// The walk over the closure the driver gathered for this stage.
    walk: Walk<'a>,
}

impl<'a> PathResolver<'a> {
    /// A resolver of the paths of `module`, over what the resolution and the deps hold.
    pub(crate) fn new(module: ModuleId, resolution: &'a Resolution, deps: &'a CheckDeps) -> Self {
        Self {
            module,
            resolution,
            walk: Walk::of(deps.graph(), deps.closure()),
        }
    }

    /// The type a written type denotes.
    pub(crate) fn type_of(&mut self, ty: &TypeRef) -> ResolvedType {
        match ty {
            // A mistake is the parser's and the lowering's to report; the type of it is the
            // error type, which absorbs whatever it meets. `_` is a type to infer, and a
            // signature has nowhere to infer it from yet.
            TypeRef::Missing | TypeRef::Infer => (Ty::Error, Vec::new()),
            TypeRef::Path(path) => self.type_of_path(path),
        }
    }

    /// The entity a path denotes in `namespace`.
    ///
    /// A path whose names a body binds is not resolved here: the check knows the bindings of the
    /// body it is checking, and the resolver does not.
    pub(crate) fn entity_of(&mut self, path: &PathData, namespace: Namespace) -> ResolvedEntity {
        let (entity, errors) = self
            .walk
            .entity_of(self.module, self.resolution, path, namespace);

        (entity, errors.into_iter().map(type_error).collect())
    }

    /// The arguments a path is applied to, resolved as types.
    ///
    /// The language has no generics yet, so a path with arguments is reported; the arguments are
    /// resolved all the same, so that a mistake inside one is reported as the mistake it is.
    pub(crate) fn arguments_of(&mut self, path: &PathData) -> (Vec<Ty>, Vec<TypeError>) {
        let mut args = Vec::new();
        let mut errors = Vec::new();

        for arg in &path.root_args {
            let (ty, mut found) = self.type_of(arg);
            args.push(ty);
            errors.append(&mut found);
        }

        for segment in &path.segments {
            for arg in &segment.args {
                let (ty, mut found) = self.type_of(arg);
                args.push(ty);
                errors.append(&mut found);
            }
        }

        if !args.is_empty() {
            errors.push(TypeError::TypeArguments { name: path.name() });
        }

        (args, errors)
    }

    /// The type a written path denotes.
    fn type_of_path(&mut self, path: &PathData) -> ResolvedType {
        let (args, mut errors) = self.arguments_of(path);

        // A type variable of the entity is a type, and it is not a name any module holds.
        if let PathAnchor::TypeVar(var) = &path.anchor {
            return (Ty::Param(var.clone()), errors);
        }

        let (entity, mut found) = self.entity_of(path, Namespace::Ty);
        errors.append(&mut found);

        let Some(entity) = entity else {
            return (Ty::Error, errors);
        };

        match ClassLoc::try_from(entity.item.clone()) {
            Ok(class) => {
                let class = EntityLoc {
                    module: entity.module,
                    item: class,
                };

                (Ty::Class { class, args }, errors)
            },
            Err(_) => {
                errors.push(TypeError::NotAType { name: path.name() });
                (Ty::Error, errors)
            },
        }
    }
}

/// The check's error for what the resolver found wrong with a path.
///
/// What a path denotes is the resolver's to find; how a body is told about it is the check's.
/// The three facts a stage that reads a body has words of its own for are the check's errors,
/// and everything else is the walk's report, read as it is.
fn type_error(error: ResolveError) -> TypeError {
    match error {
        ResolveError::NotAType { name } => TypeError::NotAType { name },
        ResolveError::NotAValue { name } => TypeError::NotAValue { name },
        ResolveError::NestedName { name } => TypeError::NestedName { name },
        error => TypeError::Unresolved { error },
    }
}
