//! The type surface of a module: the types its declarations write, resolved.
//!
//! The pass reads the item tree and the names the resolution produced, and never a body: a body
//! edit cannot change what a module shows, and what the module shows is not invalidated by one
//! ([ADR-0017]). A type a declaration writes is resolved like any path: through the imports of
//! the module, and over the closure of the check when it names a module ([ADR-0016]). A
//! declaration that does not write a type the first check needs is reported: a body is checked
//! on its own, and a signature cannot be inferred from another body yet.
//!
//! [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md
//! [ADR-0017]: ../../docs/adr/0017-resolved-types.md

use mlkc_hir_def::{
    EntityData, EntityLoc, FunctionLoc, ItemTree, TypeRef, dump::TypePlace as DeclaredType,
};
use mlkc_hir_ty::{CheckDeps, ModuleTypes, Ty};
use mlkc_resolve::Resolution;

use crate::{
    diagnostic::{TypeDiag, TypeError, TypePlace},
    resolve::PathResolver,
};

/// Resolves the types a module's declarations write into its type surface.
///
/// The surface holds the type of every entity that has one, a function above all: the resolved
/// types of its parameters and of its result, as a [`Ty::Fn`]. A type a declaration does not
/// write, and a `_` where one is required, are reported and checked as the error type, so that
/// the bodies of the module still check.
///
/// A public function that writes no type is already reported by the lowering, which requires the
/// signature of a public function ([ADR-0004]); this pass does not report it again. What it
/// reports is what the first check requires of every declaration.
///
/// The deps carry the projects and the closure the paths of the surface walk; the type surfaces
/// of other modules are not read here, since a written type names a class and never a resolved
/// type of another module.
///
/// [ADR-0004]: ../../docs/adr/0004-module-system.md
pub fn resolve_module_types(
    tree: &ItemTree,
    resolution: &Resolution,
    deps: &CheckDeps,
) -> (ModuleTypes, Vec<TypeDiag>) {
    let mut types = ModuleTypes::new();
    let mut diagnostics = Vec::new();
    let mut resolver = PathResolver::new(tree.module(), resolution, deps);

    for (item, _) in tree.entities() {
        match tree.entity_data(item.clone()) {
            Some(EntityData::Function(data)) => {
                let Ok(function) = FunctionLoc::try_from(item.clone()) else {
                    continue;
                };

                let public = data.visibility.is_public();
                let mut params = Vec::new();

                for (index, param) in data.signature.params.iter().enumerate() {
                    let place = TypePlace::Declared {
                        item: item.clone(),
                        place: DeclaredType::Parameter(index),
                    };

                    match &param.ty {
                        Some(TypeRef::Infer) => {
                            diagnostics.push(TypeDiag::new(
                                TypeError::MissingType {
                                    function: function.clone(),
                                    parameter: Some(index),
                                },
                                place,
                            ));
                            params.push(Ty::Error);
                        },
                        Some(ty) => {
                            let (resolved, errors) = resolver.type_of(ty);
                            diagnostics.extend(elsewhere(errors, &place));
                            params.push(resolved);
                        },
                        None => {
                            // A public function without a parameter type is the lowering's to
                            // report; the first check needs the type all the same.
                            if !public {
                                diagnostics.push(TypeDiag::new(
                                    TypeError::MissingType {
                                        function: function.clone(),
                                        parameter: Some(index),
                                    },
                                    place,
                                ));
                            }

                            params.push(Ty::Error);
                        },
                    }
                }

                let place = TypePlace::Declared {
                    item: item.clone(),
                    place: DeclaredType::Result,
                };
                let ret = match &data.signature.ret {
                    Some(TypeRef::Infer) => {
                        diagnostics.push(TypeDiag::new(
                            TypeError::MissingType {
                                function: function.clone(),
                                parameter: None,
                            },
                            place,
                        ));
                        Ty::Error
                    },
                    Some(ty) => {
                        let (resolved, errors) = resolver.type_of(ty);
                        diagnostics.extend(elsewhere(errors, &place));
                        resolved
                    },
                    None => {
                        if !public {
                            diagnostics.push(TypeDiag::new(
                                TypeError::MissingType {
                                    function,
                                    parameter: None,
                                },
                                place,
                            ));
                        }

                        Ty::Error
                    },
                };

                types.insert(
                    EntityLoc {
                        module: tree.module(),
                        item: item.clone(),
                    },
                    Ty::function(params, ret),
                );
            },
            // The language writes no constant yet; a type it writes resolves like any other.
            Some(EntityData::Const(data)) => {
                let place = TypePlace::Entity(item.clone());
                let ty = data.ty.as_ref().map_or(Ty::Error, |ty| {
                    let (resolved, errors) = resolver.type_of(ty);
                    diagnostics.extend(elsewhere(errors, &place));
                    resolved
                });

                types.insert(
                    EntityLoc {
                        module: tree.module(),
                        item: item.clone(),
                    },
                    ty,
                );
            },
            // A class has no type of a value, and an import stands for the entity it names.
            _ => {},
        }
    }

    (types, diagnostics)
}

/// The diagnostics of a written type, headed for the place it is written in.
///
/// A path of the surface that does not resolve was reported by the resolution of the module,
/// which walked the same paths over the same closure; the check reports the paths a resolution
/// does not read ([`crate::resolve`]).
fn elsewhere(errors: Vec<TypeError>, place: &TypePlace) -> Vec<TypeDiag> {
    errors
        .into_iter()
        .filter(|error| !matches!(error, TypeError::Unresolved { .. }))
        .map(|error| TypeDiag::new(error, place.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, sync::Arc};

    use mlkc_hir_def::{
        ClassLoc, EntityLoc, ItemLocLike, ItemTree, ModuleId, ModuleScope, Name, Prelude,
        ProjectGraph, ProjectId,
    };
    use mlkc_hir_ty::{Builtins, CheckDeps, Ty};
    use mlkc_lower::lower_module;
    use mlkc_parser::parse;
    use mlkc_resolve::{Resolution, closure};
    use mlkc_syntax::ModuleRoot;
    use mlkc_vfs::{FileId, RelPathBuf};

    use super::{TypeError, resolve_module_types};

    /// The classes of the language, as a module of a test declares them.
    const CLASSES: &str = "#[builtin]\ntype Int\n\n#[builtin]\ntype Unit\n\n#[builtin]\ntype String\n\n#[builtin]\ntype Bool\n";

    fn module(source: &str) -> ItemTree {
        let source = format!("{CLASSES}\n{source}");
        let parsed = parse(&source);
        let root = parsed.tree::<ModuleRoot>();
        let relative = RelPathBuf::try_from("main.mlk").expect("a relative path");

        lower_module(
            ModuleId(FileId::from_raw(0)),
            &root,
            &Prelude::none(),
            &[ProjectId::new("std")],
            relative.as_path(),
        )
        .item_tree
    }

    fn entity(tree: &ItemTree, name: &str) -> EntityLoc {
        tree.entities()
            .find(|(item, _)| item.name() == Some(&Name::new(name)))
            .map(|(item, _)| {
                EntityLoc {
                    module: tree.module(),
                    item,
                }
            })
            .expect("the entity to be declared")
    }

    fn class(tree: &ItemTree, name: &str) -> EntityLoc<ClassLoc> {
        let entity = entity(tree, name);

        EntityLoc {
            module: entity.module,
            item: ClassLoc::try_from(entity.item).expect("a class"),
        }
    }

    /// The resolution and the deps of a module of a test: the names of the module are its own,
    /// and the closure is the one of its surface.
    fn world(tree: &ItemTree) -> (Resolution, CheckDeps) {
        let graph = Arc::new(ProjectGraph::default());
        let closure = closure::of(tree.module(), tree, &graph, &BTreeMap::new(), &mut |_| None);
        let resolution = Resolution::new(
            Arc::new(ModuleScope::default()),
            BTreeMap::new(),
            Arc::from(Vec::new()),
        );
        let builtins = Builtins::new(
            class(tree, "Int"),
            class(tree, "Unit"),
            class(tree, "String"),
            class(tree, "Bool"),
        );
        let deps = CheckDeps::new(builtins)
            .with_graph(graph)
            .with_closure(closure);

        (resolution, deps)
    }

    #[test]
    fn a_written_signature_resolves_into_the_types_of_its_function() {
        let tree = module("fun id(x: Int): Int = x\nfun pair(x: Int, y: Bool): Int = x\n");
        let (resolution, deps) = world(&tree);
        let (types, diagnostics) = resolve_module_types(&tree, &resolution, &deps);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");

        let int = Ty::class(class(&tree, "Int"));
        let boolean = Ty::class(class(&tree, "Bool"));

        assert_eq!(
            types.get(&entity(&tree, "id")),
            Some(&Ty::function(vec![int.clone()], int.clone())),
        );
        assert_eq!(
            types.get(&entity(&tree, "pair")),
            Some(&Ty::function(vec![int.clone(), boolean], int)),
        );
        assert_eq!(types.len(), 2);
    }

    #[test]
    fn a_private_signature_that_is_not_written_is_reported() {
        let tree = module("fun f(x): Int = x\nfun g(x: Int) = x\n");
        let (resolution, deps) = world(&tree);
        let (types, diagnostics) = resolve_module_types(&tree, &resolution, &deps);
        assert_eq!(diagnostics.len(), 2);

        // The types are still there, with the error type where nothing was written.
        assert_eq!(
            types.get(&entity(&tree, "f")),
            Some(&Ty::function(
                vec![Ty::Error],
                Ty::class(class(&tree, "Int"))
            )),
        );
        assert_eq!(
            types.get(&entity(&tree, "g")),
            Some(&Ty::function(
                vec![Ty::class(class(&tree, "Int"))],
                Ty::Error
            )),
        );

        assert!(matches!(diagnostics[0].error(), TypeError::MissingType {
            parameter: Some(0),
            ..
        }));
        assert!(matches!(diagnostics[1].error(), TypeError::MissingType {
            parameter: None,
            ..
        }));
    }

    #[test]
    fn an_inferred_type_in_a_signature_is_reported() {
        let tree = module("fun f(x: _): Int = x\n");
        let (resolution, deps) = world(&tree);
        let (types, diagnostics) = resolve_module_types(&tree, &resolution, &deps);
        assert_eq!(diagnostics.len(), 1);
        assert!(matches!(diagnostics[0].error(), TypeError::MissingType {
            parameter: Some(0),
            ..
        }));
        assert_eq!(
            types.get(&entity(&tree, "f")),
            Some(&Ty::function(
                vec![Ty::Error],
                Ty::class(class(&tree, "Int"))
            )),
        );
    }

    #[test]
    fn a_public_function_is_the_lowerings_to_report() {
        // The lowering requires a signature of a public function ([ADR-0004]); the check does
        // not report the same mistake a second time.
        let tree = module("pub fun f(x: Int) = x\n");
        let (resolution, deps) = world(&tree);
        let (_, diagnostics) = resolve_module_types(&tree, &resolution, &deps);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn a_class_applied_to_arguments_is_reported() {
        let tree = module("fun f(x: Int[Bool]): Int = x\n");
        let (resolution, deps) = world(&tree);
        let (types, diagnostics) = resolve_module_types(&tree, &resolution, &deps);
        assert_eq!(diagnostics.len(), 1);
        assert!(matches!(
            diagnostics[0].error(),
            TypeError::TypeArguments { .. }
        ));

        let int = class(&tree, "Int");
        assert_eq!(
            types.get(&entity(&tree, "f")),
            Some(&Ty::function(
                vec![Ty::Class {
                    class: int.clone(),
                    args: vec![Ty::class(class(&tree, "Bool"))],
                }],
                Ty::class(int),
            )),
        );
    }
}
