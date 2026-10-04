//! The module of one module: the functions it declares, the functions it calls, and the
//! signature of every one of them ([ADR-0021]).
//!
//! A callee the module declares is called by its index; every other callee is an import: a
//! function of another module of the program, or one declared `#[extern]`, which a host
//! implements. What an import carries is the name the linker resolves --- the canonical path of
//! the module the function belongs to, and the name of the function ([ADR-0021]).
//!
//! [adr-0021]: ../../docs/adr/0021-translation-units.md

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use mlkc_codegen_wasm::{FnSignature, ModuleFunction, ModuleImport, ModuleMir};
use mlkc_hir_def::{
    BodyLoc, EntityData, EntityLoc, FunctionLoc, ItemLoc, ItemLocLike, ModuleId, Name, Pat,
};
use mlkc_hir_ty::Ty;

use super::{Driver, Lowered, ModuleMirSlot, Pass, Unit, entries_are_the_same};

impl Driver {
    /// The canonical name of `module`: the path of the project it belongs to, and the path of
    /// the module inside it ([ADR-0021]).
    ///
    /// `None` when the module belongs to no project the graph holds, which is a module a host
    /// pushed on its own: it has no name another module could import.
    ///
    /// [adr-0021]: ../../docs/adr/0021-translation-units.md
    pub fn module_name(&mut self, module: ModuleId) -> Option<String> {
        let project = self.projects.project_of(module)?.clone();
        let index = self.module_index(&project)?;
        let path = index
            .iter()
            .find(|(_, it)| *it == module)
            .map(|(path, _)| path.iter().map(Name::as_str).collect::<Vec<_>>().join("::"))?;

        if path.is_empty() {
            return Some(project.as_str().to_owned());
        }

        Some(format!("{}::{path}", project.as_str()))
    }

    /// The module of `module`: the bodies of its functions, and the functions they call.
    ///
    /// `None` when the module is not one the front end read clean, or when a body of it, a
    /// signature it writes, or an import of a callee cannot be read: a module that is not whole
    /// is not a value a host assembles.
    pub fn mir_module(&mut self, module: ModuleId) -> Option<Arc<ModuleMir>> {
        let name = self.module_name(module)?;
        let lowered = self.lower(module.0)?;
        let builtins = self.builtins()?;
        let types = self.module_types(module)?;
        let mut functions = Vec::new();
        let mut bodies = Vec::new();

        for body in lowered.bodies() {
            let owner = body.owner();
            let BodyLoc::Function(_) = &owner.item else {
                // A constant's body is a body, but it has no function ABI and nothing calls it.
                continue;
            };

            let entity = EntityLoc::from(owner.clone());
            let Some(Ty::Fn { params, ret }) = types.get(&entity) else {
                return None;
            };
            let Some(EntityData::Function(data)) =
                lowered.item_tree().entity_data(entity.item.clone())
            else {
                return None;
            };
            let Some(function_name) = owner.item.name().map(ToString::to_string) else {
                continue;
            };

            let hir = &body.body().body;
            let param_names = hir
                .params()
                .iter()
                .map(|pat| {
                    match &hir[*pat] {
                        Pat::Bind(name) => Some(name.clone()),
                        Pat::Missing | Pat::Wildcard => None,
                    }
                })
                .collect();

            let body = self.mir_ssa(owner)?;
            bodies.push(Arc::clone(&body));

            functions.push(ModuleFunction {
                owner: owner.clone(),
                name: function_name,
                signature: FnSignature {
                    params: params.clone(),
                    ret: ret.as_ref().clone(),
                },
                param_names,
                exported: data.visibility.is_public(),
                body,
            });
        }

        let local: BTreeSet<EntityLoc<FunctionLoc>> = functions
            .iter()
            .filter_map(|function| {
                match &function.owner.item {
                    BodyLoc::Function(loc) => {
                        Some(EntityLoc {
                            module: function.owner.module,
                            item: loc.clone(),
                        })
                    },
                    BodyLoc::Const(_) => None,
                }
            })
            .collect();
        let mut imports = Vec::new();
        let mut callees = BTreeMap::new();

        for entity in callees_of(&functions) {
            if local.contains(&entity) {
                continue;
            }

            let Some(name) = entity.item.name().map(ToString::to_string) else {
                continue;
            };
            let module_name = self.module_name(entity.module)?;
            let callee_types = self.module_types(entity.module)?;

            callees.insert(entity.module, Arc::clone(&callee_types));

            let Some(Ty::Fn { params, ret }) = callee_types.get(&EntityLoc::from(entity.clone()))
            else {
                return None;
            };
            let callee_lowered = self.lower(entity.module.0)?;
            let external = external_of(&callee_lowered, &entity);

            imports.push(ModuleImport {
                entity,
                module: module_name,
                name,
                signature: FnSignature {
                    params: params.clone(),
                    ret: ret.as_ref().clone(),
                },
                external,
            });
        }

        let held = self
            .mir_modules
            .get(&module)
            .filter(|slot| {
                Arc::ptr_eq(&slot.lowered, &lowered)
                    && Arc::ptr_eq(&slot.types, &types)
                    && slot.builtins == builtins
                    && slot.bodies.len() == bodies.len()
                    && slot
                        .bodies
                        .iter()
                        .zip(&bodies)
                        .all(|(held, current)| Arc::ptr_eq(held, current))
                    && entries_are_the_same(&slot.callees, &callees)
            })
            .map(|slot| Arc::clone(&slot.value));
        let unit = Unit::Module(module);

        if let Some(value) = held {
            self.stats.consulted(Pass::MirModule, &unit, true, true);

            return Some(value);
        }

        self.stats.consulted(
            Pass::MirModule,
            &unit,
            self.mir_modules.contains_key(&module),
            false,
        );

        let started = self.ticking();
        let key_builtins = builtins.clone();
        let value = self.guarded(
            |driver| {
                format!(
                    "building the module of `{}`",
                    driver.unit_name(&Unit::Module(module)),
                )
            },
            || {
                ModuleMir {
                    name,
                    builtins,
                    imports,
                    functions,
                }
            },
        );

        self.stats.ran(Pass::MirModule, &unit, self.clock, started);

        let value = Arc::new(value?);

        // A module equal to the one the driver holds is the value it holds ([ADR-0008]).
        //
        // [ADR-0008]: ../../docs/adr/0008-compiler-driver.md
        let value = match self.mir_modules.get(&module) {
            Some(slot) if *slot.value == *value => {
                self.stats.kept(Pass::MirModule, &unit);

                Arc::clone(&slot.value)
            },
            _ => value,
        };

        self.mir_modules.insert(module, ModuleMirSlot {
            lowered,
            types,
            builtins: key_builtins,
            bodies,
            callees,
            value: Arc::clone(&value),
        });

        Some(value)
    }
}

/// The entities a call of a body names, in the order of their entities.
///
/// A call written inside a lambda is a call of the module like any other: the lambdas of a body
/// are walked with it ([ADR-0026][adr-0026]).
///
/// [adr-0026]: ../../docs/adr/0026-closure-representation.md
fn callees_of(functions: &[ModuleFunction]) -> BTreeSet<EntityLoc<FunctionLoc>> {
    use mlkc_mir::{Callee, CodeRef, Rvalue, StmtKind};

    fn collect(code: CodeRef<'_>, callees: &mut BTreeSet<EntityLoc<FunctionLoc>>) {
        for (_, block) in code.blocks.iter() {
            for stmt in &block.stmts {
                let StmtKind::Assign { rvalue, .. } = &stmt.kind;

                if let Rvalue::Call {
                    callee: Callee::Entity(entity),
                    ..
                } = rvalue
                {
                    callees.insert(entity.clone());
                }
            }
        }
    }

    let mut callees = BTreeSet::new();

    for function in functions {
        collect(function.body.code(), &mut callees);

        for (_, lambda) in function.body.lambdas.iter() {
            collect(lambda.code(), &mut callees);
        }
    }

    callees
}

/// Whether the function is declared `#[extern]` in the module that declares it.
fn external_of(lowered: &Lowered, entity: &EntityLoc<FunctionLoc>) -> bool {
    let item = ItemLoc::Function(entity.item.clone());

    matches!(
        lowered.item_tree().entity_data(item),
        Some(EntityData::Function(data)) if data.attributes.external,
    )
}
