//! The paths of a check that cross modules: a module of the project, a module of another
//! project, and a name a module re-exports ([ADR-0016], [ADR-0017]).
//!
//! The world is the one a driver holds, cut down to what the passes read: the standard library
//! of the compiler ([mlkc-stdlib]) and a project `app` that depends on it. The check of a body
//! is called directly, with the resolution and the deps a driver would have assembled --- the
//! closure gathered over the surface of the module and over its bodies ([`closure::of_check`]).
//!
//! [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md
//! [ADR-0017]: ../../docs/adr/0017-resolved-types.md

use std::{collections::BTreeMap, sync::Arc};

use mlkc_hir_def::{
    Body, BodyEntityLoc, ClassLoc, Closure, EntityLoc, Interface, ItemLocLike, ItemTree, ModuleId,
    ModuleIndex, Name, ProjectData, ProjectGraph, ProjectId,
};
use mlkc_hir_ty::{Builtins, CheckDeps, ModuleTypes, Ty};
use mlkc_lower::{lower_body, lower_module};
use mlkc_parser::parse;
use mlkc_resolve::{Resolution, ResolveDeps, closure, resolve_module};
use mlkc_syntax::ModuleRoot;
use mlkc_typeck::{TypeError, check_body, resolve_module_types};
use mlkc_vfs::{FileId, RelPathBuf};

/// A world of modules: what a check of a body reads of the rest of the project.
struct World {
    /// The projects, what each of them depends on, and the project of every module.
    graph: Arc<ProjectGraph>,
    /// The surface of every module, by module.
    trees: BTreeMap<ModuleId, ItemTree>,
    /// The bodies of every module, by module, in the order it declares them.
    bodies: BTreeMap<ModuleId, Vec<(BodyEntityLoc, Body)>>,
    /// The interface of every module, by module.
    interfaces: BTreeMap<ModuleId, Arc<Interface>>,
    /// The module index of every project, by project.
    indexes: BTreeMap<ProjectId, Arc<ModuleIndex>>,
}

impl World {
    /// The module a path names in a project.
    fn module(&self, project: &str, path: &str) -> ModuleId {
        let project = ProjectId::new(project);
        let segments: Vec<Name> = path.split("::").map(Name::new).collect();

        self.indexes
            .get(&project)
            .and_then(|index| index.get(&segments))
            .unwrap_or_else(|| panic!("the project `{project}` to hold a module `{path}`"))
    }

    /// The surface of a module.
    fn tree(&self, module: ModuleId) -> &ItemTree {
        self.trees
            .get(&module)
            .unwrap_or_else(|| panic!("the world to hold {module:?}"))
    }

    /// The body of an entity of a module.
    fn body_of(&self, module: ModuleId, name: &str) -> &(BodyEntityLoc, Body) {
        self.bodies[&module]
            .iter()
            .find(|(owner, _)| owner.item.name() == Some(&Name::new(name)))
            .unwrap_or_else(|| panic!("{module:?} to declare a body for `{name}`"))
    }

    /// The closure of a resolution: the modules the paths of the surface reach.
    fn closure(&self, module: ModuleId) -> Closure {
        let interfaces = &self.interfaces;

        closure::of(
            module,
            self.tree(module),
            &self.graph,
            &self.indexes,
            &mut |module| interfaces.get(&module).cloned(),
        )
    }

    /// The closure of a check: the surface, and every body of the module.
    fn check_closure(&self, module: ModuleId) -> Closure {
        let interfaces = &self.interfaces;
        let bodies: Vec<&Body> = self.bodies[&module].iter().map(|(_, body)| body).collect();

        closure::of_check(
            module,
            self.tree(module),
            bodies.iter().copied(),
            &self.graph,
            &self.indexes,
            &mut |module| interfaces.get(&module).cloned(),
        )
    }

    /// The resolution of a module: what its names denote, and what its imports resolved to.
    fn resolution(&self, module: ModuleId) -> Resolution {
        let deps = ResolveDeps {
            graph: Arc::clone(&self.graph),
            closure: self.closure(module),
        };

        resolve_module(module, self.tree(module), &deps)
    }

    /// The classes of the language, read off `std::core`.
    fn builtins(&self) -> Builtins {
        let core = self.tree(self.module(mlkc_stdlib::PROJECT, mlkc_stdlib::CORE));

        Builtins::new(
            class_of(core, "Int"),
            class_of(core, "Unit"),
            class_of(core, "String"),
            class_of(core, "Bool"),
        )
    }

    /// The type surface of a module, as the driver would resolve it: over the closure of the
    /// module's own surface.
    fn types(&self, module: ModuleId) -> ModuleTypes {
        let resolution = self.resolution(module);
        let deps = CheckDeps::new(self.builtins())
            .with_graph(Arc::clone(&self.graph))
            .with_closure(self.closure(module));

        resolve_module_types(self.tree(module), &resolution, &deps).0
    }

    /// What the check of a body of `module` reads: the closure of the check, and the type
    /// surface of every module of the world.
    fn deps(&self, module: ModuleId) -> CheckDeps {
        let mut deps = CheckDeps::new(self.builtins())
            .with_graph(Arc::clone(&self.graph))
            .with_closure(self.check_closure(module));

        for module in self.trees.keys() {
            deps = deps.with_types(*module, Arc::new(self.types(*module)));
        }

        deps
    }
}

/// The projects and the modules of a [`World`], lowered from the text of each module.
struct WorldBuilder {
    graph: ProjectGraph,
    trees: BTreeMap<ModuleId, ItemTree>,
    bodies: BTreeMap<ModuleId, Vec<(BodyEntityLoc, Body)>>,
    next: u32,
}

impl WorldBuilder {
    /// A world that holds nothing yet.
    fn new() -> Self {
        Self {
            graph: ProjectGraph::default(),
            trees: BTreeMap::new(),
            bodies: BTreeMap::new(),
            next: 0,
        }
    }

    /// Records a project: what it depends on, and the prelude its modules are given.
    fn project(mut self, name: &str, data: ProjectData) -> Self {
        self.graph.insert(ProjectId::new(name), data);
        self
    }

    /// Lowers a module of `project`.
    fn module(mut self, project: &str, at: &str, source: &str) -> Self {
        let project = ProjectId::new(project);
        let module = ModuleId(FileId::from_raw(self.next));
        self.next += 1;
        self.graph.set_module_project(module, project.clone());

        // The projects the module may name ([ADR-0016]).
        let projects: Vec<ProjectId> = self
            .graph
            .project(&project)
            .map(|data| data.dependencies.values().cloned().collect())
            .unwrap_or_default();
        let prelude = self.graph.prelude_of(module).clone();
        let relative = RelPathBuf::try_from(at).expect("the place of a module to be a path");
        let parsed = parse(source);
        assert!(
            parsed.diagnostics().is_empty(),
            "the module at `{at}` does not parse: {:?}",
            parsed.diagnostics(),
        );

        let root = parsed.tree::<ModuleRoot>();
        let lowered = lower_module(module, &root, &prelude, &projects, relative.as_path());
        assert!(
            lowered.diagnostics.is_empty(),
            "the module at `{at}` does not lower: {:?}",
            lowered.diagnostics,
        );

        let bodies = lowered
            .bodies
            .iter()
            .filter_map(|decl| {
                lower_body(&lowered.item_tree, &decl.decl)
                    .map(|lowered_body| (decl.owner.clone(), lowered_body.body))
            })
            .collect();

        self.trees.insert(module, lowered.item_tree);
        self.bodies.insert(module, bodies);
        self
    }

    /// The world: the interfaces and the module indexes are derived from what was lowered.
    fn build(self) -> World {
        let interfaces: BTreeMap<ModuleId, Arc<Interface>> = self
            .trees
            .iter()
            .map(|(module, tree)| (*module, Arc::new(Interface::of(tree))))
            .collect();

        let mut indexes: BTreeMap<ProjectId, ModuleIndex> = BTreeMap::new();

        for (module, tree) in &self.trees {
            let Some(project) = self.graph.project_of(*module) else {
                continue;
            };

            indexes
                .entry(project.clone())
                .or_insert_with(|| ModuleIndex::new(project.clone()))
                .insert(*module, &tree.path());
        }

        World {
            graph: Arc::new(self.graph),
            trees: self.trees,
            bodies: self.bodies,
            interfaces,
            indexes: indexes
                .into_iter()
                .map(|(project, index)| (project, Arc::new(index)))
                .collect(),
        }
    }
}

/// The world of the tests: the standard library, and a project `app` that depends on it.
fn world() -> World {
    let mut app = ProjectData::default();
    app.dependencies.insert(
        Name::new(mlkc_stdlib::PROJECT),
        ProjectId::new(mlkc_stdlib::PROJECT),
    );

    let mut world = WorldBuilder::new().project(mlkc_stdlib::PROJECT, mlkc_stdlib::project());

    for module in mlkc_stdlib::modules() {
        world = world.module(
            mlkc_stdlib::PROJECT,
            &format!("{}.mlk", module.name),
            module.source,
        );
    }

    world
        .project("app", app)
        .module(
            "app",
            "data.mlk",
            "module project::data\n\npub fun double(value: Int): Int = value\n",
        )
        .module(
            "app",
            "main.mlk",
            "module project::main\n\n\
             fun call(): Int = project::data::double(1)\n\n\
             fun text(): String = \"hello\"\n\n\
             fun qualified(): std::core::Int = project::data::double(2)\n",
        )
        .module(
            "app",
            "broken.mlk",
            "module project::broken\n\nfun call(): Int = project::data::missing(1)\n",
        )
        .build()
}

/// The class an item tree declares under a name.
fn class_of(tree: &ItemTree, name: &str) -> EntityLoc<ClassLoc> {
    let entity = entity_of(tree, name);

    EntityLoc {
        module: tree.module(),
        item: ClassLoc::try_from(entity.item).expect("a class"),
    }
}

/// The entity an item tree declares under a name.
fn entity_of(tree: &ItemTree, name: &str) -> EntityLoc {
    tree.entities()
        .find(|(item, _)| item.name() == Some(&Name::new(name)))
        .map_or_else(
            || panic!("{} to declare `{name}`", tree.path()),
            |(item, _)| {
                EntityLoc {
                    module: tree.module(),
                    item,
                }
            },
        )
}

#[test]
fn a_signature_that_names_a_module_of_another_project_is_walked() {
    let world = world();
    let core = world.module(mlkc_stdlib::PROJECT, mlkc_stdlib::CORE);
    let int = class_of(world.tree(core), "Int");
    let data = world.module("app", "data");

    let types = world.types(data);
    let double = entity_of(world.tree(data), "double");

    assert_eq!(
        types.get(&double),
        Some(&Ty::function(
            vec![Ty::class(int.clone())],
            Ty::class(int.clone())
        )),
    );
}

#[test]
fn a_reexport_chain_and_a_qualified_path_resolve_to_one_class() {
    let world = world();
    let core = world.module(mlkc_stdlib::PROJECT, mlkc_stdlib::CORE);
    let int = class_of(world.tree(core), "Int");
    let string = class_of(world.tree(core), "String");
    let main = world.module("app", "main");

    let types = world.types(main);

    // `call` writes `Int`, a name the prelude re-exports from `std::core`; `qualified` writes
    // the path of that class itself. Both resolve to the one class of the library.
    for name in ["call", "qualified"] {
        let function = entity_of(world.tree(main), name);
        assert_eq!(
            types.get(&function),
            Some(&Ty::function(vec![], Ty::class(int.clone()))),
            "{name}",
        );
    }

    let text = entity_of(world.tree(main), "text");
    assert_eq!(
        types.get(&text),
        Some(&Ty::function(vec![], Ty::class(string))),
    );
}

#[test]
fn a_body_calls_a_module_that_its_surface_does_not_name() {
    let world = world();
    let core = world.module(mlkc_stdlib::PROJECT, mlkc_stdlib::CORE);
    let int = class_of(world.tree(core), "Int");
    let data = world.module("app", "data");
    let main = world.module("app", "main");

    // The surface of `main` names `std` and never `data`: only the closure of the check holds
    // the interface of the module the body calls.
    assert!(!world.closure(main).interfaces().contains_key(&data));
    assert!(world.check_closure(main).interfaces().contains_key(&data));

    let resolution = world.resolution(main);
    let deps = world.deps(main);
    let (owner, body) = world.body_of(main, "call");
    let (checked, diagnostics) =
        check_body(owner.clone(), world.tree(main), body, &resolution, &deps);

    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    assert_eq!(checked.expr_type(body.root()), Some(&Ty::class(int)));
}

#[test]
fn a_body_path_that_names_nothing_is_reported_by_the_check() {
    let world = world();
    let broken = world.module("app", "broken");

    let resolution = world.resolution(broken);
    let deps = world.deps(broken);
    let (owner, body) = world.body_of(broken, "call");
    let (_, diagnostics) = check_body(owner.clone(), world.tree(broken), body, &resolution, &deps);

    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    assert!(matches!(
        diagnostics[0].error(),
        TypeError::Unresolved { .. }
    ));
}
