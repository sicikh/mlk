//! The resolution of a module, called as a pass.
//!
//! A resolution is a function of one module's own item tree and of the values its paths name
//! ([ADR-0016]), and this is where it is called directly: a world of projects is built by
//! lowering the text of each module ([mlkc-lower]), the interfaces and the module indexes are
//! derived from what was lowered, and the pass is handed a closure a driver would have assembled
//! ([ADR-0009]). No driver and no file system stand between the pass and what a test reads: the
//! scope a module resolved to, what its walk found wrong, and what the closure holds.
//!
//! [ADR-0009]: ../../docs/adr/0009-pass-contract.md
//! [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md

use std::{collections::BTreeMap, sync::Arc};

use mlkc_hir_def::{
    Body, BodyEntityLoc, Closure, EntityLoc, Interface, ItemKind, ItemLocLike, ItemTree, ModuleId,
    ModuleIndex, ModuleLocator, ModuleScope, Name, Namespace, PathAnchor, PathRoot, PlainPath,
    PlainPathId, Prelude, ProjectData, ProjectGraph, ProjectId, Read, ResolveError, Visibility,
    dump::TypePlace,
};
use mlkc_lower::lower_module;
use mlkc_resolve::{
    ImportCycles, NamedModules, NamedPlace, ResolveDeps, ResolveDiag, closure, import_cycles,
    named_modules, resolve_module,
};
use mlkc_syntax::ModuleRoot;
use mlkc_vfs::{FileId, RelPathBuf};

/// A world of projects and modules: what a resolution is called in.
///
/// It is what the driver holds, cut down to what a pass reads: the graph, the item tree of every
/// module, the interface every module shows, and the module index of every project. The module of
/// a path is found the way a walk finds it, in the index of the project.
struct World {
    /// The projects, what each of them depends on, and the project of every module.
    graph: Arc<ProjectGraph>,
    /// The surface of every module, by module.
    trees: BTreeMap<ModuleId, ItemTree>,
    /// The interface of every module, by module.
    interfaces: BTreeMap<ModuleId, Arc<Interface>>,
    /// The module index of every project, by project.
    indexes: BTreeMap<ProjectId, Arc<ModuleIndex>>,
    /// The module every file of the world was lowered as, by the place of the file.
    places: BTreeMap<String, ModuleId>,
}

impl World {
    /// The module a path names in a project: `world.module("app", "types::utils")`.
    fn module(&self, project: &str, path: &str) -> ModuleId {
        let project = ProjectId::new(project);
        let segments = segments(path);

        self.indexes
            .get(&project)
            .and_then(|index| index.get(&segments))
            .unwrap_or_else(|| panic!("the project `{project}` to hold a module `{path}`"))
    }

    /// The module whose file stands at `at`, which is how a module of no project is named.
    fn at(&self, at: &str) -> ModuleId {
        self.places
            .get(at)
            .copied()
            .unwrap_or_else(|| panic!("no file of the world stands at `{at}`"))
    }

    /// The surface of a module.
    fn tree(&self, module: ModuleId) -> &ItemTree {
        self.trees
            .get(&module)
            .unwrap_or_else(|| panic!("the world to hold {module:?}"))
    }

    /// The closure of a module: the walk of its paths, run once to gather what it reads
    /// ([ADR-0009]).
    ///
    /// [ADR-0009]: ../../docs/adr/0009-pass-contract.md
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

    /// Resolves a module: the closure is gathered, and the pass walks the same paths over it.
    fn resolve(&self, module: ModuleId) -> (ModuleScope, Vec<ResolveDiag>) {
        let deps = ResolveDeps {
            graph: Arc::clone(&self.graph),
            closure: self.closure(module),
        };

        // A test reads the parts a resolution is made of; the driver keeps the whole value.
        let resolved = resolve_module(module, self.tree(module), &deps);

        (
            ModuleScope::clone(resolved.resolution().scope()),
            resolved.diagnostics().to_vec(),
        )
    }

    /// The modules the names of a module reach, read the way a driver reads them.
    ///
    /// The names of a world are the ones of the surface --- the imports and the types the
    /// declarations write --- and a name a body writes reaches a module the same way
    /// ([`named_modules`]): a driver hands the bodies over, and a test of the graph has none.
    fn named(&self, module: ModuleId) -> NamedModules {
        let deps = ResolveDeps {
            graph: Arc::clone(&self.graph),
            closure: self.closure(module),
        };
        let resolved = resolve_module(module, self.tree(module), &deps);

        named_modules(
            module,
            self.tree(module),
            std::iter::empty::<(BodyEntityLoc, &Body)>(),
            resolved.resolution(),
            &deps,
        )
    }

    /// The cycles of the names of every module of the world.
    fn cycles(&self) -> ImportCycles {
        let named: Vec<(ModuleId, PlainPathId, NamedModules)> = self
            .trees
            .keys()
            .map(|module| (*module, self.tree(*module).path(), self.named(*module)))
            .collect();

        import_cycles(
            named
                .iter()
                .map(|(module, path, names)| (*module, path.clone(), names)),
        )
    }
}

/// The projects and the modules of a [`World`], lowered from the text of each module.
struct WorldBuilder {
    graph: ProjectGraph,
    trees: BTreeMap<ModuleId, ItemTree>,
    places: BTreeMap<String, ModuleId>,
    next: u32,
}

impl WorldBuilder {
    /// A world that holds nothing yet.
    fn new() -> Self {
        Self {
            graph: ProjectGraph::default(),
            trees: BTreeMap::new(),
            places: BTreeMap::new(),
            next: 0,
        }
    }

    /// Records a project: what it depends on, and the prelude its modules are given.
    fn project(mut self, name: &str, data: ProjectData) -> Self {
        self.graph.insert(ProjectId::new(name), data);
        self
    }

    /// Lowers a module of `project`, standing at the place `at` in it: a file at `types/utils.mlk`
    /// is the module `types::utils`, unless its preamble declares another path.
    fn module(self, project: &str, at: &str, source: &str) -> Self {
        let project = ProjectId::new(project);

        self.lower(Some(project), at, source)
    }

    /// Lowers a file no project claims: a module that belongs to no project, which names the
    /// projects of the graph and no module of its own.
    fn unclaimed(self, at: &str, source: &str) -> Self {
        self.lower(None, at, source)
    }

    /// The world: the modules were lowered, and the interfaces and the indexes are derived from
    /// what the lowering made of them.
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
            interfaces,
            indexes: indexes
                .into_iter()
                .map(|(project, index)| (project, Arc::new(index)))
                .collect(),
            places: self.places,
        }
    }

    /// Lowers one file, as the module of `project` or as a module of none.
    ///
    /// A module of a test is a module the language accepts: the parse and the lowering are
    /// expected to report nothing, and what a test asserts is what the resolution found.
    fn lower(mut self, project: Option<ProjectId>, at: &str, source: &str) -> Self {
        let module = ModuleId(FileId::from_raw(self.next));
        self.next += 1;

        if let Some(project) = &project {
            self.graph.set_module_project(module, project.clone());
        }

        let relative =
            RelPathBuf::try_from(at).expect("the place of a module to be a relative path");
        let parsed = mlkc_parser::parse(source);
        let root = parsed.tree::<ModuleRoot>();

        assert!(
            parsed.diagnostics().is_empty(),
            "the module at `{at}` does not parse: {:?}",
            parsed.diagnostics(),
        );

        // The projects the module may name, which is what the lowering reads the name a type is
        // rooted at against: the projects its project depends on, or every project of the world
        // for a module of no project. A module's own project is called by the keyword, and not
        // by a name ([ADR-0016]).
        //
        // [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md
        let mut projects: Vec<ProjectId> = match &project {
            Some(project) => {
                match self.graph.project(project) {
                    Some(data) => data.dependencies.values().cloned().collect(),
                    None => Vec::new(),
                }
            },
            None => {
                self.graph
                    .projects()
                    .map(|(project, _)| project.clone())
                    .collect()
            },
        };

        projects.sort();

        let prelude = self.graph.prelude_of(module).clone();
        let lowered = lower_module(module, &root, &prelude, &projects, relative.as_path());

        assert!(
            lowered.diagnostics.is_empty(),
            "the module at `{at}` does not lower: {:?}",
            lowered.diagnostics,
        );

        self.places.insert(at.to_owned(), module);
        self.trees.insert(module, lowered.item_tree);
        self
    }
}

/// The world of most tests: one project, `app`, which depends on nothing and gives its modules
/// no prelude.
fn app() -> WorldBuilder {
    WorldBuilder::new().project("app", bare_project())
}

/// A project that depends on nothing and gives its modules no prelude.
fn bare_project() -> ProjectData {
    ProjectData {
        prelude: Prelude::none(),
        ..ProjectData::default()
    }
}

/// A project that gives its modules no prelude and depends on the projects named.
fn depends_on(dependencies: &[&str]) -> ProjectData {
    let mut data = bare_project();

    for dependency in dependencies {
        data.dependencies
            .insert(name(dependency), ProjectId::new(dependency));
    }

    data
}

/// Records the standard library, module by module, as the compiler records it.
fn standard_library(world: WorldBuilder) -> WorldBuilder {
    let mut world = world.project(mlkc_stdlib::PROJECT, mlkc_stdlib::project());

    for module in mlkc_stdlib::modules() {
        world = world.module(
            mlkc_stdlib::PROJECT,
            &format!("{}.mlk", module.name),
            module.source,
        );
    }

    world
}

/// The project of a test that reads the standard library: it depends on the library, and its
/// modules are given the prelude of the language.
fn on_std() -> ProjectData {
    let mut data = ProjectData::default();

    data.dependencies.insert(
        name(mlkc_stdlib::PROJECT),
        ProjectId::new(mlkc_stdlib::PROJECT),
    );

    data
}

/// The name a test writes.
fn name(text: &str) -> Name {
    Name::new(text)
}

/// The path a test writes, rooted at the keyword `project`: `["types", "Point"]` is
/// `project::types::Point`.
fn path(segments: &[&str]) -> PlainPathId {
    PlainPathId::new(PlainPath::from_root(
        PathRoot::Project,
        segments.iter().map(|segment| Name::new(segment)),
    ))
}

/// The segments of a path a test writes: `types::utils` is two names.
fn segments(path: &str) -> Vec<Name> {
    path.split("::").map(Name::new).collect()
}

/// What a name a module declares denotes: the entity the module wrote of it.
fn declared(world: &World, module: ModuleId, text: &str, namespace: Namespace) -> EntityLoc {
    match world.tree(module).scope().anchor(&name(text), namespace) {
        PathAnchor::Item(entity) => entity,
        anchor => panic!("`{text}` to be a name of {module:?}: {anchor:?}"),
    }
}

/// What a name of a resolved scope denotes where a type belongs.
fn ty_of(scope: &ModuleScope, text: &str) -> Option<(EntityLoc, Visibility)> {
    scope.get(&name(text))?.ty.clone()
}

/// What a name of a resolved scope denotes where a value belongs.
fn value_of(scope: &ModuleScope, text: &str) -> Option<(EntityLoc, Visibility)> {
    scope.get(&name(text))?.value.clone()
}

/// What a name of a resolved scope denotes where a module belongs.
fn module_of(scope: &ModuleScope, text: &str) -> Option<(ModuleLocator, Visibility)> {
    scope.get(&name(text))?.module.clone()
}

/// The only diagnostic of a resolution, or a panic.
fn only(diagnostics: &[ResolveDiag]) -> &ResolveDiag {
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");

    &diagnostics[0]
}

/// An entry a walk read of a module index: the module a path names, if it names one.
fn read_module(project: ProjectId, segments: &[&str], found: Option<ModuleId>) -> Read {
    Read::Module {
        project,
        path: path(segments),
        found,
    }
}

/// An entry a walk read of a module index: whether modules stand under a path.
fn read_prefix(project: ProjectId, segments: &[&str], found: bool) -> Read {
    Read::Prefix {
        project,
        path: path(segments),
        found,
    }
}

/// The module `types`: it declares a data type named `Point`.
const DATA: &str = "pub data Point\n";

/// The module `types`: it declares a data type and a function of one name.
const DATA_BOTH_NAMESPACES: &str = "\
pub data Point

pub fun Point(): Point = Point
";

#[test]
fn an_import_of_a_name_of_another_module_resolves_to_the_entity_it_denotes() {
    let world = app()
        .module("app", "types.mlk", DATA)
        .module(
            "app",
            "main.mlk",
            "\
use project::types::Point

fun get(): Point = get()
",
        )
        .build();

    let main = world.module("app", "main");
    let types = world.module("app", "types");

    let (scope, diagnostics) = world.resolve(main);

    assert!(diagnostics.is_empty(), "{diagnostics:?}");

    // The name an import brings in denotes what the path resolved to, with the visibility of
    // the import: the reach of the name in the module that holds it.
    assert_eq!(
        ty_of(&scope, "Point"),
        Some((
            declared(&world, types, "Point", Namespace::Ty),
            Visibility::Private,
        )),
    );
    assert_eq!(value_of(&scope, "Point"), None);
}

#[test]
fn a_name_of_another_module_denotes_what_the_module_shows_in_every_namespace() {
    let world = app()
        .module("app", "types.mlk", DATA_BOTH_NAMESPACES)
        .module(
            "app",
            "main.mlk",
            "\
use project::types::Point

fun make(): Point = Point()
",
        )
        .build();

    let main = world.module("app", "main");
    let types = world.module("app", "types");

    let (scope, diagnostics) = world.resolve(main);

    assert!(diagnostics.is_empty(), "{diagnostics:?}");

    // A data type and a function of one name are two names, and the import of `Point` brings in
    // both: what it lands in is not the module alone to say.
    assert_eq!(
        ty_of(&scope, "Point"),
        Some((
            declared(&world, types, "Point", Namespace::Ty),
            Visibility::Private,
        )),
    );
    assert_eq!(
        value_of(&scope, "Point"),
        Some((
            declared(&world, types, "Point", Namespace::Value),
            Visibility::Private,
        )),
    );
}

#[test]
fn a_name_a_module_declares_denotes_what_it_wrote() {
    let world = app()
        .module(
            "app",
            "main.mlk",
            "\
pub data Int

fun helper(): Int = helper()
",
        )
        .build();

    let main = world.module("app", "main");

    let (scope, diagnostics) = world.resolve(main);

    assert!(diagnostics.is_empty(), "{diagnostics:?}");

    assert_eq!(
        ty_of(&scope, "Int"),
        Some((
            declared(&world, main, "Int", Namespace::Ty),
            Visibility::Public,
        )),
    );
    assert_eq!(
        value_of(&scope, "helper"),
        Some((
            declared(&world, main, "helper", Namespace::Value),
            Visibility::Private,
        )),
    );
}

#[test]
fn a_module_names_its_own_project_by_the_keyword() {
    let world = app()
        .module("app", "types.mlk", DATA)
        .module(
            "app",
            "main.mlk",
            "\
use project::types::Point

fun get(): Point = get()
",
        )
        .build();

    let main = world.module("app", "main");
    let types = world.module("app", "types");

    let (scope, diagnostics) = world.resolve(main);

    assert!(diagnostics.is_empty(), "{diagnostics:?}");

    // The keyword is the project the module is written in, and the names after it are read
    // inside it.
    assert_eq!(
        ty_of(&scope, "Point"),
        Some((
            declared(&world, types, "Point", Namespace::Ty),
            Visibility::Private,
        )),
    );
}

#[test]
fn a_module_does_not_name_its_own_project_by_its_name() {
    let world = app()
        .module("app", "types.mlk", DATA)
        .module(
            "app",
            "main.mlk",
            "\
use app::types::Point

fun get() = 1
",
        )
        .build();

    let main = world.module("app", "main");

    let (scope, diagnostics) = world.resolve(main);

    // The name the project is declared under is not one a module of it may write: a path rooted
    // at it names no project of the module, and is not read as a module of the project either.
    assert_eq!(only(&diagnostics).error(), &ResolveError::UnknownProject {
        name: name("app")
    },);
    assert_eq!(scope.get(&name("Point")), None);
}

#[test]
fn a_name_a_module_keeps_to_itself_is_not_a_name_another_module_reads() {
    let world = app()
        .module("app", "types.mlk", "data Hidden\n")
        .module(
            "app",
            "main.mlk",
            "\
use project::types::Hidden

fun get() = 1
",
        )
        .build();

    let main = world.module("app", "main");
    let types = world.module("app", "types");

    let (scope, diagnostics) = world.resolve(main);

    // A name the module does not export is not a name of the interface it shows, so the walk
    // finds no name the path ends at, and the diagnostic is the one of an unknown name.
    let diagnostic = only(&diagnostics);

    assert_eq!(diagnostic.error(), &ResolveError::UnknownName {
        path: path(&["types"]),
        module: types,
        name: name("Hidden"),
    },);
    assert_eq!(diagnostic.place().item().kind(), ItemKind::Use);

    // A name that resolved to nothing is no name of the module.
    assert_eq!(scope.get(&name("Hidden")), None);
}

#[test]
fn a_path_that_names_no_module_of_the_project_is_reported() {
    let world = app()
        .module(
            "app",
            "main.mlk",
            "\
use project::nope::Point

fun get() = 1
",
        )
        .build();

    let main = world.module("app", "main");

    let (_, diagnostics) = world.resolve(main);

    assert_eq!(only(&diagnostics).error(), &ResolveError::UnknownModule {
        project: ProjectId::new("app"),
        name: name("nope"),
    },);
}

#[test]
fn a_name_a_module_does_not_hold_is_reported() {
    let world = app()
        .module("app", "types.mlk", DATA)
        .module(
            "app",
            "main.mlk",
            "\
use project::types::Other

fun get() = 1
",
        )
        .build();

    let main = world.module("app", "main");
    let types = world.module("app", "types");

    let (_, diagnostics) = world.resolve(main);

    assert_eq!(only(&diagnostics).error(), &ResolveError::UnknownName {
        path: path(&["types"]),
        module: types,
        name: name("Other"),
    },);
}

#[test]
fn a_module_names_a_project_it_depends_on() {
    let world = WorldBuilder::new()
        .project("app", depends_on(&["other"]))
        .project("other", bare_project())
        .module("other", "thing.mlk", "pub data X\n")
        .module(
            "app",
            "main.mlk",
            "\
use other::thing::X

fun get(): other::thing::X = get()
",
        )
        .build();

    let main = world.module("app", "main");
    let other = world.module("other", "thing");

    let (scope, diagnostics) = world.resolve(main);

    assert!(diagnostics.is_empty(), "{diagnostics:?}");

    assert_eq!(
        ty_of(&scope, "X"),
        Some((
            declared(&world, other, "X", Namespace::Ty),
            Visibility::Private,
        )),
    );
}

#[test]
fn a_module_names_the_projects_it_depends_on_and_no_other() {
    let world = app()
        .project("other", bare_project())
        .module("other", "thing.mlk", "pub data X\n")
        .module(
            "app",
            "main.mlk",
            "\
use other::thing::X

fun get() = 1
",
        )
        .build();

    let main = world.module("app", "main");

    let (_, diagnostics) = world.resolve(main);

    // `other` is no project this module may name, so the name is read as the name of a module of
    // the module's own project, and the project holds no module called `other`.
    assert_eq!(only(&diagnostics).error(), &ResolveError::UnknownModule {
        project: ProjectId::new("app"),
        name: name("other"),
    },);
}

#[test]
fn a_use_of_a_module_binds_the_module() {
    let world = app()
        .module("app", "types.mlk", DATA)
        .module(
            "app",
            "main.mlk",
            "\
use project::types

fun handle(value: types::Point) = 1
",
        )
        .build();

    let main = world.module("app", "main");
    let types = world.module("app", "types");

    let (scope, diagnostics) = world.resolve(main);

    assert!(diagnostics.is_empty(), "{diagnostics:?}");

    assert_eq!(
        module_of(&scope, "types"),
        Some((ModuleLocator::Module(types), Visibility::Private)),
    );
}

#[test]
fn a_use_of_a_prefix_binds_the_modules_that_stand_under_it() {
    let world = app()
        .module("app", "types/utils.mlk", DATA)
        .module(
            "app",
            "main.mlk",
            "\
use project::types

fun handle(value: types::utils::Point) = 1
",
        )
        .build();

    let main = world.module("app", "main");

    let (scope, diagnostics) = world.resolve(main);

    // A name that is no module and that modules stand under binds the prefix, and the names
    // after it are read among the modules under it.
    assert!(diagnostics.is_empty(), "{diagnostics:?}");

    assert_eq!(
        module_of(&scope, "types"),
        Some((
            ModuleLocator::Prefix {
                project: ProjectId::new("app"),
                path: path(&["types"]),
            },
            Visibility::Private,
        )),
    );
}

#[test]
fn a_name_under_a_prefix_that_is_not_there_is_reported() {
    let world = app()
        .module("app", "types/utils.mlk", DATA)
        .module(
            "app",
            "main.mlk",
            "\
use project::types

fun handle(value: types::nope::Point) = 1
",
        )
        .build();

    let main = world.module("app", "main");

    let (_, diagnostics) = world.resolve(main);

    let diagnostic = only(&diagnostics);

    assert_eq!(diagnostic.error(), &ResolveError::UnknownModule {
        project: ProjectId::new("app"),
        name: name("nope"),
    },);
    assert_eq!(
        diagnostic.place().type_place(),
        Some(TypePlace::Parameter(0))
    );
}

#[test]
fn a_path_rooted_at_the_project_is_read_among_the_modules_of_the_project() {
    let world = app()
        .module("app", "types.mlk", DATA)
        .module(
            "app",
            "main.mlk",
            "\
fun read(value: project::types::Point) = 1
",
        )
        .build();

    let main = world.module("app", "main");

    let (_, diagnostics) = world.resolve(main);

    assert!(diagnostics.is_empty(), "{diagnostics:?}");
}

#[test]
fn a_segment_after_a_data_type_is_left_to_the_naming_a_type_decides() {
    let world = app()
        .module("app", "types.mlk", "pub data Map\n")
        .module(
            "app",
            "main.mlk",
            "\
use project::types::Map

fun entry(value: Map::Entry): Map = value
",
        )
        .build();

    let main = world.module("app", "main");
    let types = world.module("app", "types");

    let (scope, diagnostics) = world.resolve(main);

    // What a name after a data type is is what a type decides, and the naming of type classes
    // is a record of its own: the path is left unresolved, and it is not reported as an unknown
    // name.
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    assert_eq!(
        ty_of(&scope, "Map"),
        Some((
            declared(&world, types, "Map", Namespace::Ty),
            Visibility::Private,
        )),
    );
}

#[test]
fn a_name_the_prelude_wrote_reaches_through_a_reexport_to_the_entity_it_names() {
    let world = standard_library(WorldBuilder::new().project("app", on_std()))
        .module("app", "main.mlk", "fun size(value: Int): Int = value\n")
        .build();

    let main = world.module("app", "main");
    let core = world.module("std", "core");
    let prelude = world.module("std", "prelude");

    let (scope, diagnostics) = world.resolve(main);

    assert!(diagnostics.is_empty(), "{diagnostics:?}");

    // `Int` is a name the prelude of the language brings in: the module that holds it re-exports
    // a name of `std::core`, and the walk follows the path the re-export wrote ([ADR-0011]).
    //
    // [ADR-0011]: ../../docs/adr/0011-module-prelude.md
    assert_eq!(
        ty_of(&scope, "Int"),
        Some((
            declared(&world, core, "Int", Namespace::Ty),
            Visibility::Private,
        )),
    );

    // The interface of the module that writes the re-export is one the walk reached, so it is
    // one the closure holds.
    let closure = world.closure(main);

    for module in [core, prelude] {
        assert!(
            closure.interfaces().contains_key(&module),
            "the walk to reach {module:?}",
        );
    }
}

#[test]
fn a_name_the_prelude_brought_in_that_resolves_to_nothing_is_reported_where_it_is_used() {
    // The project is given the prelude of the language, and the graph holds no standard library:
    // the imports of the prelude resolve to nothing.
    let world = WorldBuilder::new()
        .project("app", ProjectData::default())
        .module("app", "main.mlk", "fun size(value: Int) = value\n")
        .build();

    let main = world.module("app", "main");

    let (scope, diagnostics) = world.resolve(main);

    // The import the module did not write has no place in the module to point at, so it is not
    // what is reported; the use of the name it would have brought in is.
    let diagnostic = only(&diagnostics);

    assert_eq!(diagnostic.error(), &ResolveError::UnresolvedName {
        name: name("Int")
    },);
    assert_eq!(
        diagnostic.place().type_place(),
        Some(TypePlace::Parameter(0))
    );
    assert_eq!(scope.get(&name("Int")), None);
}

#[test]
fn a_chain_of_reexports_that_returns_to_itself_resolves_to_nothing() {
    let world = app()
        .module("app", "a.mlk", "pub use project::b::X\n")
        .module("app", "b.mlk", "pub use project::a::X\n")
        .module(
            "app",
            "main.mlk",
            "\
use project::a::X

fun get() = 1
",
        )
        .build();

    let main = world.module("app", "main");
    let a = world.module("app", "a");

    // The cycle is a diagnostic of every module whose walk closes it, and the loop is inside one
    // pass: nothing converges, and no resolution waits for another one.
    let (scope, diagnostics) = world.resolve(main);

    assert_eq!(only(&diagnostics).error(), &ResolveError::CyclicImport {
        name: name("X")
    },);
    assert_eq!(scope.get(&name("X")), None);

    let (_, diagnostics) = world.resolve(a);

    assert_eq!(only(&diagnostics).error(), &ResolveError::CyclicImport {
        name: name("X")
    },);
}

#[test]
fn two_modules_that_name_each_other_are_one_cycle() {
    let world = app()
        .module(
            "app",
            "a.mlk",
            "\
use project::b::Other

pub data Mine

pub fun mine(value: Other): Mine =
    mine(value)
",
        )
        .module(
            "app",
            "b.mlk",
            "\
use project::a::Mine

pub data Other

pub fun other(value: Mine): Other =
    other(value)
",
        )
        .build();

    let a = world.module("app", "a");
    let b = world.module("app", "b");
    let cycles = world.cycles();

    // A cycle of two modules is one cycle, and neither of them is an order the other can be
    // read in ([ADR-0016]).
    //
    // [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md
    assert_eq!(cycles.cycles().len(), 1, "{cycles:?}");
    assert_eq!(cycles.cycles()[0].modules(), [a, b].as_slice());

    // Every module of the cycle is told about it, at a name that reaches a module of the cycle:
    // a reader of any of the files is told what is wrong ([ADR-0004]).
    //
    // [ADR-0004]: ../../docs/adr/0004-module-system.md
    let diagnostics = cycles.diagnostics();

    assert_eq!(diagnostics.len(), 2, "{diagnostics:?}");
    assert_eq!(diagnostics[0].module(), a);
    assert_eq!(diagnostics[0].error(), &ResolveError::ImportCycle {
        module: path(&["b"]),
    },);
    assert!(matches!(diagnostics[0].place(), NamedPlace::Import(_)));
    assert_eq!(diagnostics[1].module(), b);
    assert_eq!(diagnostics[1].error(), &ResolveError::ImportCycle {
        module: path(&["a"]),
    },);
    assert!(matches!(diagnostics[1].place(), NamedPlace::Import(_)));
}

#[test]
fn names_that_never_come_back_are_no_cycle() {
    let world = app()
        .module(
            "app",
            "a.mlk",
            "\
use project::b::Other

pub data Mine

pub fun mine(value: Other): Mine =
    mine(value)
",
        )
        .module("app", "b.mlk", "pub data Other\n")
        .build();

    let a = world.module("app", "a");
    let b = world.module("app", "b");
    let named = world.named(a);

    // The names of `a` reach `b`, and where they reach it is where the import is written: the
    // graph keeps one place per module, and it is what a cycle report points at.
    assert_eq!(named.len(), 1, "{named:?}");
    assert!(named.get(b).is_some());

    let cycles = world.cycles();

    assert!(cycles.cycles().is_empty(), "{cycles:?}");
    assert!(cycles.diagnostics().is_empty(), "{cycles:?}");
}

#[test]
fn a_module_of_no_project_names_the_projects_of_the_graph() {
    let world = standard_library(WorldBuilder::new())
        .unclaimed(
            "main.mlk",
            "\
use std::core::Int

fun size(value: Int): Int = value
",
        )
        .unclaimed("orphan.mlk", "use project::core::Int\n")
        .build();

    let main = world.at("main.mlk");
    let orphan = world.at("orphan.mlk");
    let core = world.module("std", "core");

    let (scope, diagnostics) = world.resolve(main);

    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    assert_eq!(
        ty_of(&scope, "Int"),
        Some((
            declared(&world, core, "Int", Namespace::Ty),
            Visibility::Private,
        )),
    );

    // A module of no project belongs to no project, so the keyword names nothing for it and its
    // own modules are not a place its paths may be read in.
    let (_, diagnostics) = world.resolve(orphan);

    assert_eq!(only(&diagnostics).error(), &ResolveError::NoProject);
}

#[test]
fn a_closure_holds_what_the_walk_of_a_module_reaches_and_reads() {
    let world = app()
        .module("app", "types/utils.mlk", DATA)
        .module(
            "app",
            "main.mlk",
            "\
use project::types::utils::Point

fun handle(value: Point) = 1
",
        )
        .build();

    let main = world.module("app", "main");
    let utils = world.module("app", "types::utils");
    let project = ProjectId::new("app");

    let closure = world.closure(main);

    assert_eq!(closure.interfaces().keys().copied().collect::<Vec<_>>(), [
        utils
    ],);

    // The names of a path are read as a module path, the longest first, and the entry of every
    // prefix is what the key of the resolution holds.
    assert_eq!(closure.reads(), [
        read_module(project.clone(), &["types"], None),
        read_module(project.clone(), &["types", "utils"], Some(utils)),
        read_module(project, &["types", "utils", "Point"], None),
    ],);

    // A module gathered twice reads the same values, and a value that is the same is what lets a
    // driver back-date a resolution.
    assert!(world.closure(main).reads_the_same_as(&closure));
}

#[test]
fn a_walk_reads_the_prefix_a_name_stands_on_when_no_module_is_named() {
    let world = app()
        .module("app", "types/utils.mlk", DATA)
        .module(
            "app",
            "main.mlk",
            "\
use project::types

fun handle(value: types::utils::Point) = 1
",
        )
        .build();

    let main = world.module("app", "main");
    let project = ProjectId::new("app");

    let closure = world.closure(main);
    let reads = closure.reads();

    assert!(
        reads.contains(&read_prefix(project.clone(), &["types"], true)),
        "{reads:?}",
    );

    // The entry the index answered about the names under the prefix is what a resolution of a
    // prefix is keyed by: a module added under `types` changes it.
    assert!(
        reads.contains(&read_module(project, &["types"], None)),
        "{reads:?}",
    );
}
