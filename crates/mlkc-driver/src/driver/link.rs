//! The link stage of a project: the compiled modules, the order they are instantiated in, and
//! where the program begins ([ADR-0021]).
//!
//! One module of the language becomes one WASM module, and the modules are joined by the host
//! that runs them: the plan is the manifest of that run. The order is a topological order of
//! the import graph, so every module is instantiated after the modules it imports; an import of
//! an extern is not an edge, because a host is what provides it.
//!
//! The stage also reads what a declaration alone cannot say about an entry point: that a
//! project declares one `#[entry]`, and that its signature is `() -> Unit` ([ADR-0021]).
//!
//! [adr-0021]: ../../docs/adr/0021-translation-units.md

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use mlkc_codegen_wasm::{
    CodegenDiag, DebugLevel, FnSignature, FunctionCtx, ModuleMir, WasmModule, assemble_module,
    emit_function, layout,
};
use mlkc_diagnostics::{Category, DiagKind, Diagnostic, Label, Level};
use mlkc_hir_def::{
    BodyLoc, EntityData, EntityLoc, FunctionLoc, ItemLoc, ModuleId, ProjectGraph, ProjectId,
};
use mlkc_hir_ty::{Builtins, Ty};
use mlkc_span::Span;

use super::{Driver, LinkSlot, Pass, Unit, entries_are_the_same};

/// A program ready to be run ([ADR-0021]).
///
/// The plan is a manifest rather than a merged binary: a host walks [`LinkPlan::order`],
/// instantiates the module of every id, hands the exports of the modules it instantiated as the
/// imports of the next, and calls the entry point.
///
/// [adr-0021]: ../../docs/adr/0021-translation-units.md
#[derive(Debug, Clone, PartialEq)]
pub struct LinkPlan {
    /// The modules of the program, providers before the modules that import them.
    pub order: Vec<ModuleId>,
    /// The compiled module of every module of the program, by module.
    pub modules: BTreeMap<ModuleId, Arc<WasmModule>>,
    /// The canonical name of every module of the program, by module.
    pub names: BTreeMap<ModuleId, String>,
    /// The entry point: the module, and the name of the function a host calls to run the
    /// program. `None` when the project declares no `#[entry]`, or one it cannot run.
    pub entry: Option<(ModuleId, String)>,
    /// What the code generator and the link stage reported about the program.
    pub diagnostics: Vec<Diagnostic>,
}

/// What the link stage of a project reports ([ADR-0021]).
///
/// [adr-0021]: ../../docs/adr/0021-translation-units.md
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkError {
    /// The project declares more than one `#[entry]`.
    ///
    /// A program begins in one place, and two entries are two answers to where it begins.
    MultipleEntries {
        /// The entry declared first.
        first: EntityLoc<FunctionLoc>,
        /// The entry declared after it, which is what the error is about.
        second: EntityLoc<FunctionLoc>,
    },
    /// The signature of the entry point is not `() -> Unit`.
    ///
    /// The host calls the entry with nothing and reads a word back, so the signature a program
    /// can begin in is fixed.
    EntrySignature {
        /// The entry.
        entry: EntityLoc<FunctionLoc>,
        /// The signature it writes.
        ty: Ty,
    },
    /// An import names a module the program does not hold.
    ///
    /// A resolution promises that a name of a body is a name of a module the project may name,
    /// so a provider that is not in the plan is a bug of the stages before the link.
    MissingProvider {
        /// The canonical path of the module that is not there.
        module: String,
        /// The function of it that is imported.
        name: String,
    },
}

impl LinkError {
    /// The message of the error, in one line.
    pub fn message(&self) -> String {
        match self {
            Self::MultipleEntries { first, second } => {
                format!(
                    "the project declares more than one `#[entry]`: `{second:?}` is one after \
                     `{first:?}`",
                )
            },
            Self::EntrySignature { entry, ty } => {
                format!(
                    "`{entry:?}` is the entry point and has the signature `{ty}`: a program \
                     begins in a function of `() -> Unit`",
                )
            },
            Self::MissingProvider { module, name } => {
                format!("the module `{module}` provides `{name}`, and is not one of the program",)
            },
        }
    }
}

impl DiagKind for LinkError {
    fn level(&self) -> Level {
        Level::Error
    }

    fn category(&self) -> Category {
        Category::Link
    }

    fn code(&self) -> &'static str {
        match self {
            Self::MultipleEntries { .. } => "01",
            Self::EntrySignature { .. } => "02",
            Self::MissingProvider { .. } => "03",
        }
    }
}

/// What the link stage reads: the module of every module of the program, the entries of the
/// project a host asked to link, and the classes of the language.
struct LinkInputs {
    /// The module of every module of the program, by module.
    modules: BTreeMap<ModuleId, Arc<ModuleMir>>,
    /// The functions of the root project declared `#[entry]`, in the order of their modules.
    entries: Vec<EntryCandidate>,
    /// The classes of the language, which say what the unit is.
    builtins: Builtins,
}

/// One function declared `#[entry]`, and what the link stage reads of it.
struct EntryCandidate {
    /// The module that declares it.
    module: ModuleId,
    /// The entity of the function.
    entity: EntityLoc<FunctionLoc>,
    /// The name it is called by.
    name: String,
    /// What it takes and gives back.
    signature: FnSignature,
    /// Where it is written.
    span: Option<Span>,
}

impl Driver {
    /// The plan of `project`: every module of it and of the projects it depends on, compiled
    /// and ordered, and the entry point it declares ([ADR-0021]).
    ///
    /// `None` when a module of the program is not one the front end read clean, or when a pass
    /// bugged: a program that is not whole is not a value a host runs.
    ///
    /// [adr-0021]: ../../docs/adr/0021-translation-units.md
    pub fn link(&mut self, project: &ProjectId) -> Option<Arc<LinkPlan>> {
        // The program is the project and the projects it depends on: the standard library is
        // compiled like any other project ([ADR-0021]).
        //
        // [ADR-0021]: ../../docs/adr/0021-translation-units.md
        let projects = self.project_closure(project);
        let mut modules: BTreeMap<ModuleId, Arc<ModuleMir>> = BTreeMap::new();

        for it in &projects {
            let index = self.module_index(it)?;

            for (_, module) in index.iter() {
                modules.insert(module, self.mir_module(module)?);
            }
        }

        let builtins = self.builtins()?;
        let mut entries = Vec::new();
        let index = self.module_index(project)?;

        // The entry is a function of the project a host asked to link, and never one of a
        // project it depends on.
        for (_, module) in index.iter() {
            let lowered = self.lower(module.0)?;

            for (item, _) in lowered.item_tree().entities() {
                let Some(EntityData::Function(data)) =
                    lowered.item_tree().entity_data(item.clone())
                else {
                    continue;
                };

                if !data.attributes.entry {
                    continue;
                }

                let Ok(loc) = FunctionLoc::try_from(item.clone()) else {
                    continue;
                };
                let entity = EntityLoc {
                    module,
                    item: loc.clone(),
                };
                let Some(function) = modules.get(&module)?.functions.iter().find(|function| {
                    matches!(&function.owner.item, BodyLoc::Function(loc) if *loc == entity.item)
                }) else {
                    continue;
                };
                let span = lowered
                    .item_range(&ItemLoc::Function(entity.item.clone()))
                    .map(|range| {
                        Span {
                            file: module.0,
                            range,
                        }
                    });

                entries.push(EntryCandidate {
                    module,
                    entity,
                    name: function.name.clone(),
                    signature: function.signature.clone(),
                    span,
                });
            }
        }

        let held = self
            .links
            .get(project)
            .filter(|slot| entries_are_the_same(&slot.modules, &modules))
            .map(|slot| Arc::clone(&slot.value));
        let unit = Unit::Project(project.clone());

        if let Some(value) = held {
            self.stats.consulted(Pass::Link, &unit, true, true);

            return Some(value);
        }

        self.stats
            .consulted(Pass::Link, &unit, self.links.contains_key(project), false);

        let started = self.ticking();
        let inputs = LinkInputs {
            modules,
            entries,
            builtins,
        };
        let value = self.guarded(
            |driver| {
                format!(
                    "linking the project `{}`",
                    driver.unit_name(&Unit::Project(project.clone())),
                )
            },
            || link_plan(&inputs),
        );

        self.stats.ran(Pass::Link, &unit, self.clock, started);

        let value = Arc::new(value?);

        // A plan equal to the one the driver holds is the value it holds ([ADR-0008]).
        //
        // [ADR-0008]: ../../docs/adr/0008-compiler-driver.md
        let value = match self.links.get(project) {
            Some(slot) if *slot.value == *value => {
                self.stats.kept(Pass::Link, &unit);

                Arc::clone(&slot.value)
            },
            _ => value,
        };

        self.links.insert(project.clone(), LinkSlot {
            modules: inputs.modules,
            value: Arc::clone(&value),
        });

        Some(value)
    }

    /// The projects a project is built from: its dependencies, each before it, and itself last.
    ///
    /// A dependency a project names twice is one project, and a graph with a cycle is one the
    /// walk stops at: a project that depends on itself is a mistake the loader reports.
    fn project_closure(&self, root: &ProjectId) -> Vec<ProjectId> {
        fn visit(
            graph: &ProjectGraph,
            project: &ProjectId,
            order: &mut Vec<ProjectId>,
            seen: &mut BTreeSet<ProjectId>,
        ) {
            if !seen.insert(project.clone()) {
                return;
            }

            if let Some(data) = graph.project(project) {
                for dependency in data.dependencies.values() {
                    visit(graph, dependency, order, seen);
                }
            }

            order.push(project.clone());
        }

        let mut order = Vec::new();

        visit(&self.projects, root, &mut order, &mut BTreeSet::new());

        order
    }
}

/// The plan of a program: its modules compiled, ordered, and read for an entry point.
fn link_plan(inputs: &LinkInputs) -> LinkPlan {
    let mut diagnostics = Vec::new();
    let mut modules = BTreeMap::new();

    for (id, mir) in &inputs.modules {
        let layout = layout(mir);
        let mut artifacts = Vec::new();

        for function in &mir.functions {
            let ctx = FunctionCtx {
                name: &function.name,
                signature: &function.signature,
                param_names: &function.param_names,
                layout: &layout,
            };
            let (artifact, reports) = emit_function(&function.body, &ctx);

            artifacts.push(artifact);
            diagnostics.extend(reports.iter().map(codegen_diagnostic));
        }

        let (wasm, reports) = assemble_module(mir, &artifacts, DebugLevel::Full);

        diagnostics.extend(reports.iter().map(codegen_diagnostic));
        modules.insert(*id, Arc::new(wasm));
    }

    let order = module_order(&inputs.modules, &mut diagnostics);
    let names = inputs
        .modules
        .iter()
        .map(|(module, mir)| (*module, mir.name.clone()))
        .collect();
    let entry = entry_of(inputs, &mut diagnostics);

    LinkPlan {
        order,
        modules,
        names,
        entry,
        diagnostics,
    }
}

/// The order the modules are instantiated in: a provider before the modules that import it.
///
/// An import of an extern is not an edge: a host provides it. Every other import names a module
/// of the program, and the graph is acyclic by design, so the walk always terminates.
fn module_order(
    modules: &BTreeMap<ModuleId, Arc<ModuleMir>>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<ModuleId> {
    let by_name: BTreeMap<&str, ModuleId> = modules
        .iter()
        .map(|(module, mir)| (mir.name.as_str(), *module))
        .collect();
    let mut deps: BTreeMap<ModuleId, BTreeSet<ModuleId>> = BTreeMap::new();

    for (module, mir) in modules {
        let mut edges = BTreeSet::new();

        for import in &mir.imports {
            if import.external {
                continue;
            }

            match by_name.get(import.module.as_str()) {
                Some(provider) if provider != module => {
                    edges.insert(*provider);
                },
                Some(_) => {},
                None => {
                    diagnostics.push(link_diagnostic(
                        LinkError::MissingProvider {
                            module: import.module.clone(),
                            name: import.name.clone(),
                        },
                        None,
                        None,
                    ))
                },
            }
        }

        deps.insert(*module, edges);
    }

    let mut order = Vec::with_capacity(modules.len());
    let mut placed: BTreeSet<ModuleId> = BTreeSet::new();

    loop {
        let mut progressed = false;

        for module in modules.keys() {
            if placed.contains(module) || !deps[module].iter().all(|it| placed.contains(it)) {
                continue;
            }

            placed.insert(*module);
            order.push(*module);
            progressed = true;
        }

        if !progressed {
            break;
        }
    }

    assert_eq!(
        order.len(),
        modules.len(),
        "the imports of a program to form no cycle",
    );

    order
}

/// The entry point of the project, if it declares one.
///
/// The entry is the single function declared `#[entry]`; a second entry or a signature that is
/// not `() -> Unit` is what the link stage reports, and the plan it answers has no entry.
fn entry_of(inputs: &LinkInputs, diagnostics: &mut Vec<Diagnostic>) -> Option<(ModuleId, String)> {
    let [entry, rest @ ..] = inputs.entries.as_slice() else {
        return None;
    };

    if let Some(second) = rest.first() {
        diagnostics.push(link_diagnostic(
            LinkError::MultipleEntries {
                first: entry.entity.clone(),
                second: second.entity.clone(),
            },
            second.span,
            entry.span,
        ));

        return None;
    }

    let unit = Ty::class(inputs.builtins.unit().clone());

    if !entry.signature.params.is_empty() || entry.signature.ret != unit {
        let ty = Ty::function(entry.signature.params.clone(), entry.signature.ret.clone());

        diagnostics.push(link_diagnostic(
            LinkError::EntrySignature {
                entry: entry.entity.clone(),
                ty,
            },
            entry.span,
            None,
        ));

        return None;
    }

    Some((entry.module, entry.name.clone()))
}

/// The diagnostic of what the code generator reported about a body.
fn codegen_diagnostic(report: &CodegenDiag) -> Diagnostic {
    let code = match report {
        CodegenDiag::Unsupported { .. } => "01",
        CodegenDiag::Unexpected { .. } => "02",
    };

    Diagnostic {
        level: Level::Error,
        category: Category::Codegen,
        code,
        message: report.to_string(),
        labels: vec![Label::primary(report.span(), "")],
        notes: Vec::new(),
    }
}

/// A link error and the places it is about, as a host reads it.
fn link_diagnostic(error: LinkError, primary: Option<Span>, secondary: Option<Span>) -> Diagnostic {
    let mut diagnostic = Diagnostic::from_kind(&error, error.message());

    if let Some(span) = primary {
        diagnostic = diagnostic.with_primary(span, "");
    }

    if let Some(span) = secondary {
        diagnostic = diagnostic.with_secondary(span, "");
    }

    diagnostic
}
