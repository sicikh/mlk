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
//! the module, which walked the same paths; a path of a *body* is walked here for the first
//! time, and what the walk found is the check's to report.
//!
//! [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md

use mlkc_hir_def::{ClassLoc, EntityLoc, ModuleId, Name, Namespace, PathAnchor, PathData, TypeRef};
use mlkc_hir_ty::Ty;
use mlkc_resolve::{Resolution, ResolveError, Target, Walk};

use crate::{check::CheckDeps, diagnostic::TypeError};

/// The type a written type denotes, and what could not be resolved.
pub(crate) type ResolvedType = (Ty, Vec<TypeError>);

/// The entity a path denotes, and what could not be resolved.
pub(crate) type ResolvedEntity = (Option<EntityLoc>, Vec<TypeError>);

/// What resolves the paths of one module.
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
    /// body it is checking, and this resolver does not.
    pub(crate) fn entity_of(&mut self, path: &PathData, namespace: Namespace) -> ResolvedEntity {
        match path.anchor.clone() {
            // An entity of the module: what the lowering anchored. A name written after it is
            // the name of a member, and the language has no members yet.
            PathAnchor::Item(entity) => {
                match path.segments.first() {
                    Some(segment) => {
                        (None, vec![TypeError::NestedName {
                            name: segment.name.clone(),
                        }])
                    },
                    None => (Some(entity), Vec::new()),
                }
            },
            // An entry of the import table: what the resolution resolved the import to.
            PathAnchor::Use(import) => {
                let target = self.resolution.imports().get(&import).cloned();

                match target {
                    // An import that resolved to nothing is what the resolution reported.
                    None => (None, Vec::new()),
                    Some(target) => self.continue_from(target, path, namespace),
                }
            },
            // A path rooted at a project: the names after the root are the modules of it, and
            // the walk of the whole path is the walk of those names.
            PathAnchor::Project(project) => {
                let project = project.or_else(|| self.walk.project_of(self.module));
                let Some(project) = project else {
                    return (None, vec![TypeError::Unresolved {
                        error: ResolveError::NoProject,
                    }]);
                };

                let names: Vec<Name> = path
                    .segments
                    .iter()
                    .map(|segment| segment.name.clone())
                    .collect();
                let mut seen = Vec::new();

                match self.walk.walk_in(&project, &names, &mut seen) {
                    Ok(target) => take(&target, namespace, name_of(path)),
                    Err(error) => (None, vec![TypeError::Unresolved { error }]),
                }
            },
            // A binding, an entity of the body, a type variable: the check resolves those
            // itself, and a name nothing resolved is what the lowering reported.
            PathAnchor::Binding(_)
            | PathAnchor::Local(_)
            | PathAnchor::TypeVar(_)
            | PathAnchor::Unresolved => (None, Vec::new()),
        }
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
            errors.push(TypeError::TypeArguments {
                name: name_of(path),
            });
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
                errors.push(TypeError::NotAType {
                    name: name_of(path),
                });
                (Ty::Error, errors)
            },
        }
    }

    /// The entity a path denotes, given what a walk found where the path stands.
    fn continue_from(
        &mut self,
        target: Target,
        path: &PathData,
        namespace: Namespace,
    ) -> ResolvedEntity {
        if path.segments.is_empty() {
            return take(&target, namespace, name_of(path));
        }

        // A name written after an entity is the name of a member of it, and the language has no
        // members yet.
        if target.ty().is_some() || target.value().is_some() {
            return (None, vec![TypeError::NestedName {
                name: path.segments[0].name.clone(),
            }]);
        }

        // A name written after a module is read inside it; a name written after nothing at all
        // is what the walk reported.
        let (Some((project, base)), Some(locator)) = (target.place(), target.module()) else {
            return (None, Vec::new());
        };

        let names: Vec<Name> = path
            .segments
            .iter()
            .map(|segment| segment.name.clone())
            .collect();
        let mut seen = Vec::new();

        match self
            .walk
            .walk_after(project, base, locator, &names, &mut seen)
        {
            Ok(found) => take(&found, namespace, name_of(path)),
            Err(error) => (None, vec![TypeError::Unresolved { error }]),
        }
    }
}

/// What a target denotes in the namespace a place asks for.
fn take(target: &Target, namespace: Namespace, name: Name) -> ResolvedEntity {
    let entity = match namespace {
        Namespace::Ty => target.ty(),
        Namespace::Value => target.value(),
        Namespace::Module => None,
    };

    match entity {
        Some(entity) => (Some(entity.clone()), Vec::new()),
        // A name that denotes nothing at all was reported by the resolution; one that denotes
        // something else --- a module, or an entity of the other namespace --- is reported here.
        None if target.is_empty() => (None, Vec::new()),
        None => {
            (None, vec![match namespace {
                Namespace::Value => TypeError::NotAValue { name },
                _ => TypeError::NotAType { name },
            }])
        },
    }
}

/// The name a path is read as, for a message: the name it is rooted at, or the last of its
/// names when the root is the keyword `project`.
fn name_of(path: &PathData) -> Name {
    path.root
        .name()
        .or_else(|| path.segments.last().map(|segment| &segment.name))
        .cloned()
        .unwrap_or_else(Name::missing)
}
