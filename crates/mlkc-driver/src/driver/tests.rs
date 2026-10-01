use std::{cell::Cell, sync::Arc, time::Duration};

use mlkc_diagnostics::{Category, Level};
use mlkc_hir_def::{
    EntityData, EntityLoc, ItemLoc, ItemLocLike, Name, Namespace, PathAnchor, PlainPath, Prelude,
    ProjectData, ProjectId, dump::TypePlace as DeclaredType,
};
use mlkc_line_index::LineCol;
use mlkc_rowan::{AstNodeList, Direction};
use mlkc_syntax::{FUN_KW, SyntaxKind, SyntaxToken, TextRange};
use mlkc_text_size::TextLen;
use mlkc_vfs::{Change, FileId, FileState, VfsPath};

use super::*;

/// A module of the language, written the way a person writes one.
const MODULE: &str = "\
#[extern]
fun println-int(value: Int): Unit

fun main(): Unit =
    let x = 42 * 2 - 10 in
    println-int(x + 20)
";

/// The same module with the `in` of the `let` missing.
const BROKEN: &str = "fun main(): Unit =\n    let x = 1\n";

/// A module of a project: it shows a class, and has a body no other module reads.
const DATA: &str = "\
pub type Point

pub fun origin(): Point = origin()

fun helper() = 1
";

/// The same module with the body of the private function edited.
const DATA_EDITED: &str = "\
pub type Point

pub fun origin(): Point = origin()

fun helper() = 2
";

/// The same module with another name shown.
const DATA_EXTENDED: &str = "\
pub type Point

pub type Extra

pub fun origin(): Point = origin()
";

/// A module of the same project that reads what `DATA` shows.
const READER: &str = "\
use project::data::Point

fun get(): Point = get()
";

/// A path in the virtual file system: a test has no file system.
fn path(name: &str) -> VfsPath {
    VfsPath::new_virtual_path(format!("/{name}"))
}

/// A driver that holds one file, and the id of that file.
fn driver_with(name: &str, text: &str) -> (Driver, FileId) {
    let mut driver = Driver::new();
    let path = path(name);

    driver.set_file_text(path.clone(), Some(text.to_string()));

    let file = driver.file_id(&path).expect("the file to have an id");

    (driver, file)
}

/// A driver that holds one file of a project that depends on the standard library.
///
/// The names of the language are names the library declares ([ADR-0011]): a module that
/// writes `Unit` is a module whose project is one that depends on `std`, and whose prelude
/// reaches the name through the re-export the library writes.
///
/// [ADR-0011]: ../../docs/adr/0011-module-prelude.md
fn driver_with_std(name: &str, text: &str) -> (Driver, FileId) {
    let mut driver = Driver::new();
    driver.use_std();

    let mut data = ProjectData::default();
    data.dependencies.insert(
        Name::new(mlkc_stdlib::PROJECT),
        ProjectId::new(mlkc_stdlib::PROJECT),
    );
    driver.set_project(project(), data);

    let path = path(name);
    driver.set_file_text(path.clone(), Some(text.to_string()));

    let file = driver.file_id(&path).expect("the file to have an id");
    driver.set_module_project(ModuleId(file), project());

    (driver, file)
}

/// A driver that holds the project a fixture writes, one module per file.
///
/// A fixture holds a project in one value ([`mlkc_fixture`]): every module of it is headed
/// by the place it stands at, and what follows is its source.
///
/// The project depends on nothing and gives its modules no prelude: what a test writes is
/// what its modules resolve against.
fn project_of(fixture: &str) -> Driver {
    let mut driver = Driver::new();

    let data = ProjectData {
        prelude: Prelude::none(),
        ..ProjectData::default()
    };
    driver.set_project(project(), data);

    for module in mlkc_fixture::modules(fixture) {
        let path = VfsPath::new_virtual_path(module.place.clone());
        driver.set_file_text(path.clone(), Some(module.source.clone()));

        let file = driver.file_id(&path).expect("the file to have an id");
        driver.set_module_project(ModuleId(file), project());
    }

    driver
}

/// The id of the file a fixture wrote at `place`.
fn file(driver: &Driver, place: &str) -> FileId {
    driver
        .file_id(&path(place))
        .unwrap_or_else(|| panic!("a fixture to write a module at `{place}`"))
}

/// A driver that holds the project a fixture writes, every module of it depending on the
/// standard library.
///
/// The names of the language are names the library declares ([ADR-0011]): a module of the
/// fixture writes `Int` because the prelude of the project, the prelude of the language,
/// reaches the name through the re-export the library writes. A check reads the classes of
/// the language from the library, so the fixture is the one of tests that check types.
///
/// [ADR-0011]: ../../docs/adr/0011-module-prelude.md
fn std_project_of(fixture: &str) -> Driver {
    let mut driver = Driver::new();
    driver.use_std();

    let mut data = ProjectData::default();
    data.dependencies.insert(
        Name::new(mlkc_stdlib::PROJECT),
        ProjectId::new(mlkc_stdlib::PROJECT),
    );
    driver.set_project(project(), data);

    for module in mlkc_fixture::modules(fixture) {
        let path = VfsPath::new_virtual_path(module.place.clone());
        driver.set_file_text(path.clone(), Some(module.source.clone()));

        let file = driver.file_id(&path).expect("the file to have an id");
        driver.set_module_project(ModuleId(file), project());
    }

    driver
}

/// The owner of the body a module declares under `name`, in its current revision.
fn body_of(driver: &mut Driver, file: FileId, name: &str) -> BodyEntityLoc {
    let lowered = driver.lower(file).expect("the file to be lowered");

    lowered
        .bodies()
        .iter()
        .find(|body| body.owner().item.name() == Some(&Name::new(name)))
        .map_or_else(
            || panic!("the module to declare a body for `{name}`"),
            |body| body.owner().clone(),
        )
}

/// A driver that holds a project of two modules: the one of `data`, and the one that reads
/// it. Both stand at the root of the file system, which is what names them.
fn reader_and_data(data: &str, reader: &str) -> (Driver, FileId, FileId) {
    let fixture = format!("//- /data.mlk\n{data}\n//- /main.mlk\n{reader}");
    let driver = project_of(&fixture);

    let data = file(&driver, "data.mlk");
    let main = file(&driver, "main.mlk");

    (driver, main, data)
}

/// The module of the standard library the driver holds, by the name the library gives it.
fn std_module(driver: &Driver, name: &str) -> ModuleId {
    let module = mlkc_stdlib::modules()
        .iter()
        .find(|module| module.name == name)
        .expect("a module the library has");

    ModuleId(
        driver
            .file_id(&mlkc_stdlib::path(module))
            .expect("a module of the library to be pushed"),
    )
}

#[test]
fn the_parse_holds_the_source_it_was_parsed_from() {
    let (mut driver, file) = driver_with("main.mlk", MODULE);

    let parse = driver.parse(file).expect("the file to be parsed");

    assert_eq!(parse.syntax().to_string(), MODULE);
    assert!(!parse.has_errors());
}

#[test]
fn the_typed_view_names_the_items_of_the_module() {
    let (mut driver, file) = driver_with("main.mlk", MODULE);

    let parse = driver.parse(file).expect("the file to be parsed");
    let root = parse
        .module_root()
        .expect("the root of a module to be a module");

    assert_eq!(root.items().len(), 2);
}

#[test]
fn a_bug_of_a_pass_is_a_report_and_the_pull_has_no_value() {
    let mut driver = Driver::new();

    let value = driver.guarded(
        |_| "doing something for a test".to_owned(),
        || -> u32 { mlkc_diagnostics::ice!("a pass that bugged") },
    );

    assert!(value.is_none(), "a bugged pass to answer no value");

    let report = driver.ice().expect("the driver to hold the report");

    assert_eq!(report.context(), "doing something for a test");
    assert!(report.ice().message().contains("a pass that bugged"));

    // The report reads as a crash a person can file: what the driver was doing, the
    // exception, where it was raised, and the note that the bug is the compiler's.
    let text = report.to_string();
    assert!(text.contains("internal compiler error"), "{text}");
    assert!(
        text.contains("crates/mlkc-driver/src/driver/tests.rs"),
        "{text}"
    );
    assert!(text.contains("not of your program"), "{text}");

    // The first exception is the one held: what follows a bug is its wake.
    let _ = driver.guarded(
        |_| "again".to_owned(),
        || -> () { mlkc_diagnostics::ice!("another pass that bugged") },
    );

    assert_eq!(
        driver.ice().expect("the report to stay").context(),
        "doing something for a test",
    );
}

#[test]
fn a_panic_the_compiler_did_not_raise_is_a_report_too() {
    let mut driver = Driver::new();

    let value = driver.guarded(
        |_| "calling something that broke".to_owned(),
        || -> () { panic!("an assertion inside a pass") },
    );

    assert!(value.is_none(), "a bugged pass to answer no value");

    // A payload of `ice!` is the exception itself; any other panic is read the way the
    // standard library writes it, and its location is what the hook printed.
    let report = driver.ice().expect("the driver to hold the report");

    assert!(
        report
            .ice()
            .message()
            .contains("an assertion inside a pass")
    );
    assert_eq!(report.ice().location(), "<unknown>:0:0");
}

#[test]
fn a_second_pull_is_the_value_the_first_one_returned() {
    let (mut driver, file) = driver_with("main.mlk", MODULE);

    let first = driver.parse(file).expect("the file to be parsed");
    let second = driver.parse(file).expect("the file to be parsed");

    assert!(Arc::ptr_eq(&first, &second), "the slot was built twice");
}

#[test]
fn pushing_the_same_text_again_changes_nothing() {
    let (mut driver, file) = driver_with("main.mlk", MODULE);
    let first = driver.parse(file).expect("the file to be parsed");

    let changed = driver.set_file_text(path("main.mlk"), Some(MODULE.to_string()));

    assert!(!changed, "the contents were the same");
    assert!(Arc::ptr_eq(
        &first,
        &driver.parse(file).expect("the file to be parsed")
    ));
}

/// The name of the entity `name` in the surface of a lowered module.
fn item(lowered: &Lowered, name: &str) -> ItemLoc {
    lowered
        .item_tree()
        .entities()
        .map(|(loc, _)| loc)
        .find(|loc| loc.name() == Some(&Name::new(name)))
        .unwrap_or_else(|| panic!("the module to declare the name `{name}`"))
}

/// The text of the source a range covers, if there is a range.
fn covered(source: &str, range: Option<TextRange>) -> Option<&str> {
    range.map(|it| &source[usize::from(it.start())..usize::from(it.end())])
}

#[test]
fn the_hir_of_a_module_is_lowered_from_the_same_parse() {
    let (mut driver, file) = driver_with("main.mlk", MODULE);

    let lowered = driver.lower(file).expect("the file to be lowered");

    // The module declares `println-int` and `main`, and the prelude of the language brings
    // in four more names.
    assert_eq!(
        lowered.item_tree().scope().len(),
        6,
        "six names are declared"
    );
    assert_eq!(lowered.bodies().len(), 1, "one entity owns a body");

    // The HIR holds the name of an entity rather than a place in the file, and a host
    // that marks a buffer needs the place: the driver joins the two.
    let range = lowered
        .item_range(&item(&lowered, "main"))
        .expect("the function to be written");

    assert_eq!(
        &MODULE[usize::from(range.start())..usize::from(range.end())],
        "fun main(): Unit =\n    let x = 42 * 2 - 10 in\n    println-int(x + 20)"
    );
}

#[test]
fn a_type_of_a_signature_is_written_where_the_declaration_writes_it() {
    let source = "fun f(value: Map[Int]): Int =\n    1\n";
    let (mut driver, file) = driver_with("main.mlk", source);
    let lowered = driver.lower(file).expect("the file to be lowered");
    let f = item(&lowered, "f");

    assert_eq!(
        covered(source, lowered.type_range(&f, DeclaredType::Parameter(0))),
        Some("Map[Int]"),
        "a parameter is read as the type the declaration annotated it with"
    );
    assert_eq!(
        covered(source, lowered.type_range(&f, DeclaredType::Result)),
        Some("Int")
    );
    assert_eq!(
        lowered.type_range(&f, DeclaredType::Parameter(1)),
        None,
        "a function that writes one parameter has no type for a second"
    );
}

#[test]
fn the_parameters_of_a_signature_are_counted_the_way_the_declaration_writes_them() {
    // A parameter the parser could not read is a place without a type in the signature,
    // so the parameter written after it keeps its own.
    let source = "fun f(1, value: Int): Unit = 1\n";
    let (mut driver, file) = driver_with("main.mlk", source);
    let lowered = driver.lower(file).expect("the file to be lowered");
    let f = item(&lowered, "f");

    assert_eq!(
        lowered.type_range(&f, DeclaredType::Parameter(0)),
        None,
        "the parameter that broke has no type to point at"
    );
    assert_eq!(
        covered(source, lowered.type_range(&f, DeclaredType::Parameter(1))),
        Some("Int")
    );
}

#[test]
fn a_lowering_mistake_travels_with_the_diagnostics_of_the_file() {
    // One name declared twice is a mistake the parser has nothing to say about:
    // a module says it, and the HIR cannot hold it.
    let (mut driver, file) = driver_with_std(
        "main.mlk",
        "fun f(): Unit =\n    1\n\nfun f(): Unit =\n    2\n",
    );

    let diagnostics = driver.diagnostics(file).expect("the file to be diagnosed");

    assert_eq!(diagnostics.lowering().len(), 1, "{diagnostics:?}");

    let diagnostic = &diagnostics.lowering()[0];

    assert_eq!(diagnostic.category, Category::Lowering);
    assert_eq!(diagnostic.level, Level::Error);
    assert_eq!(diagnostic.code, "01");
}

#[test]
fn a_mistake_of_a_body_travels_with_the_diagnostics_of_the_file() {
    // A body is lowered from the item tree of the module, and a name it is written with is
    // lowered with it: what a body says wrong is a mistake of the module, and a host reads
    // it for the file like any other.
    let (mut driver, file) = driver_with_std("main.mlk", "fun main(): Unit =\n    nope(1)\n");

    let diagnostics = driver.diagnostics(file).expect("the file to be diagnosed");

    assert_eq!(diagnostics.lowering().len(), 1, "{diagnostics:?}");

    let diagnostic = &diagnostics.lowering()[0];

    assert_eq!(diagnostic.category, Category::Lowering);
    assert_eq!(diagnostic.level, Level::Error);
    assert_eq!(diagnostic.code, "15");
}

#[test]
fn a_lowered_module_is_the_value_the_slot_holds() {
    let (mut driver, file) = driver_with("main.mlk", MODULE);
    let first = driver.lower(file).expect("the file to be lowered");
    let second = driver.lower(file).expect("the file to be lowered");

    assert!(Arc::ptr_eq(&first, &second), "the slot was built twice");
}

#[test]
fn the_hir_follows_the_text_and_the_old_one_keeps_its_own() {
    let (mut driver, file) = driver_with("main.mlk", MODULE);
    let before = driver.lower(file).expect("the file to be lowered");

    driver.set_file_text(path("main.mlk"), Some(BROKEN.to_string()));
    let after = driver.lower(file).expect("the file to be lowered");

    assert!(!Arc::ptr_eq(&before, &after), "the slot was not rebuilt");
    assert_eq!(before.item_tree().scope().len(), 6, "the old value stands");
    assert_eq!(
        after.item_tree().scope().len(),
        5,
        "a function without an `in` is still a function"
    );
}

/// The path a name of the module denotes, which is what an import brings in.
fn import_path(lowered: &Lowered, name: &str) -> String {
    let tree = lowered.item_tree();
    let anchor = tree.scope().anchor(&Name::new(name), Namespace::Ty);
    let PathAnchor::Use(import) = anchor else {
        panic!("`{name}` to be an import: {anchor:?}");
    };

    match tree.entity_data(ItemLoc::Use(import)) {
        Some(EntityData::Use(data)) => data.path.to_string(),
        other => panic!("the import of `{name}` to be a use: {other:?}"),
    }
}

#[test]
fn the_prelude_brings_the_names_of_the_language_into_a_module() {
    let (mut driver, file) = driver_with("main.mlk", "fun main(): Int =\n    1\n");
    let lowered = driver.lower(file).expect("the file to be lowered");

    assert_eq!(import_path(&lowered, "Int"), "std::prelude::Int");
    assert_eq!(import_path(&lowered, "Unit"), "std::prelude::Unit");

    // The import is written nowhere in the buffer, so a host is given no place to mark.
    let int = item(&lowered, "Int");
    assert_eq!(lowered.item_range(&int), None);
}

#[test]
fn a_module_that_says_no_prelude_is_given_none() {
    let (mut driver, file) = driver_with(
        "main.mlk",
        "#[no-prelude]\nmodule project::main-module\n\nfun main(): Int =\n    1\n",
    );
    let lowered = driver.lower(file).expect("the file to be lowered");

    assert!(lowered.item_tree().attributes().no_prelude);
    assert_eq!(
        lowered.item_tree().scope().len(),
        1,
        "`main`, and nothing else"
    );
    assert_eq!(
        lowered
            .item_tree()
            .scope()
            .anchor(&Name::new("Int"), Namespace::Ty),
        PathAnchor::Unresolved,
    );
}

#[test]
fn setting_the_prelude_of_a_project_changes_what_its_modules_are_lowered_to() {
    let (mut driver, file) = driver_with("main.mlk", "fun main(): Int =\n    1\n");
    let before = driver.lower(file).expect("the file to be lowered");
    let parse = driver.parse(file).expect("the file to be parsed");

    assert_eq!(import_path(&before, "Int"), "std::prelude::Int");

    // A prelude of the project replaces the one of the language.
    let data = ProjectData {
        prelude: prelude_of(&["project", "core", "Int"]),
        ..ProjectData::default()
    };

    assert!(driver.set_project(project(), data.clone()));
    assert!(
        !driver.set_project(project(), data),
        "the project did not change",
    );
    assert!(driver.set_module_project(ModuleId(file), project()));

    let after = driver.lower(file).expect("the file to be lowered");

    assert!(!Arc::ptr_eq(&before, &after), "the slot was not dropped");
    assert_eq!(import_path(&after, "Int"), "project::core::Int");
    assert_eq!(
        after
            .item_tree()
            .scope()
            .anchor(&Name::new("Unit"), Namespace::Ty),
        PathAnchor::Unresolved,
        "the prelude of the project replaced the one of the language",
    );

    // A parse is not what a project changes: the one the driver already holds stands.
    assert_eq!(
        driver.project_graph().project_of(ModuleId(file)),
        Some(&project()),
    );
    let parsed_again = driver.parse(file).expect("the file to be parsed");
    assert!(Arc::ptr_eq(&parse, &parsed_again), "the parse was not kept");
}

#[test]
fn a_module_is_called_by_where_its_file_stands() {
    let (mut driver, main) = driver_with("main.mlk", "fun main(): Int =\n    1\n");
    let lib = path("lib/arith.mlk");

    driver.set_file_text(lib.clone(), Some("fun size(): Int =\n    1\n".to_string()));
    let lib = driver.file_id(&lib).expect("the file to have an id");

    // A module is called by the place of its file, and a project is a set of modules:
    // recording a module as one of the project says nothing about where it stands.
    assert!(driver.set_project(project(), ProjectData::default()));
    assert!(driver.set_module_project(ModuleId(main), project()));
    assert!(driver.set_module_project(ModuleId(lib), project()));

    let root = driver.lower(main).expect("the file to be lowered");
    let module = driver.lower(lib).expect("the file to be lowered");

    assert_eq!(called_by(&root), "project::main");
    assert_eq!(called_by(&module), "project::lib::arith");

    // A file written at the root of the file system has no place under it, and is called
    // by the name of its file: a module a host pushes on its own is the module the file it
    // is written in is called.
    let (mut driver, lone) = driver_with("lone.mlk", "fun main(): Int =\n    1\n");
    let lowered = driver.lower(lone).expect("the file to be lowered");

    assert_eq!(called_by(&lowered), "project::lone");

    // A module of a project is called by its own place: the project holds the module, and
    // says nothing about where it stands.
    let (mut driver, first) = driver_with("lib/main.mlk", "fun main(): Int =\n    1\n");
    let elsewhere = path("main.mlk");

    driver.set_file_text(
        elsewhere.clone(),
        Some("fun size(): Int =\n    1\n".to_string()),
    );
    let elsewhere = driver.file_id(&elsewhere).expect("the file to have an id");

    assert!(driver.set_project(project(), ProjectData::default()));
    assert!(driver.set_module_project(ModuleId(first), project()));
    assert!(driver.set_module_project(ModuleId(elsewhere), project()));

    let lowered = driver.lower(elsewhere).expect("the file to be lowered");

    assert_eq!(called_by(&lowered), "project::main");

    // A file pushed at the root of the file system stands at a place that names no file,
    // and a place that names no file names no module: there is nothing to lower.
    let (mut driver, root) = driver_with("", "fun main(): Int =\n    1\n");

    assert!(driver.lower(root).is_none());
}

/// The path a lowered module is called by.
fn called_by(lowered: &Lowered) -> String {
    lowered.item_tree().path().to_string()
}

#[test]
fn a_project_changes_only_what_belongs_to_it() {
    let (mut driver, main) = driver_with("main.mlk", "fun main(): Int =\n    1\n");
    driver.set_file_text(
        path("lib.mlk"),
        Some("fun size(): Int =\n    1\n".to_string()),
    );
    let lib = driver
        .file_id(&path("lib.mlk"))
        .expect("the file to have an id");

    let first = ProjectId::new("first");
    let second = ProjectId::new("second");

    assert!(driver.set_project(first.clone(), ProjectData {
        prelude: prelude_of(&["project", "core", "Int"]),
        ..ProjectData::default()
    }));
    assert!(driver.set_project(second.clone(), ProjectData::default()));
    assert!(driver.set_module_project(ModuleId(main), first.clone()));
    assert!(driver.set_module_project(ModuleId(lib), second));

    let before_main = driver.lower(main).expect("the file to be lowered");
    let before_lib = driver.lower(lib).expect("the file to be lowered");

    assert_eq!(import_path(&before_main, "Int"), "project::core::Int");
    assert_eq!(import_path(&before_lib, "Int"), "std::prelude::Int");

    // The prelude of one project changes, and the module of the other is left alone.
    assert!(driver.set_project(first, ProjectData {
        prelude: prelude_of(&["project", "other", "Int"]),
        ..ProjectData::default()
    }));

    let after_main = driver.lower(main).expect("the file to be lowered");
    let after_lib = driver.lower(lib).expect("the file to be lowered");

    assert!(
        !Arc::ptr_eq(&before_main, &after_main),
        "the slot was not dropped"
    );
    assert!(Arc::ptr_eq(&before_lib, &after_lib), "the slot was dropped");
    assert_eq!(import_path(&after_main, "Int"), "project::other::Int");
}

#[test]
fn a_module_that_changes_project_is_read_with_the_other_project() {
    let mut driver = Driver::new();
    let project = ProjectId::new("the-project");

    assert!(driver.set_project(project.clone(), ProjectData {
        prelude: prelude_of(&["project", "core", "Int"]),
        ..ProjectData::default()
    }));

    // A module the graph does not know is read with the prelude of the language, and one
    // recorded as a module of a project is read with the prelude of it.
    driver.set_file_text(
        path("lib.mlk"),
        Some("fun size(): Int =\n    1\n".to_string()),
    );
    let lib = driver
        .file_id(&path("lib.mlk"))
        .expect("the file to have an id");
    let before = driver.lower(lib).expect("the file to be lowered");

    assert_eq!(import_path(&before, "Int"), "std::prelude::Int");

    assert!(driver.set_module_project(ModuleId(lib), project.clone()));
    assert!(!driver.set_module_project(ModuleId(lib), project));

    let after = driver.lower(lib).expect("the file to be lowered");

    assert!(!Arc::ptr_eq(&before, &after), "the slot was not dropped");
    assert_eq!(import_path(&after, "Int"), "project::core::Int");
}

#[test]
fn dropping_a_project_returns_its_modules_to_the_prelude_of_the_language() {
    let (mut driver, file) = driver_with("main.mlk", "fun main(): Int =\n    1\n");
    let project = ProjectId::new("the-project");

    driver.set_project(project.clone(), ProjectData {
        prelude: Prelude::none(),
        ..ProjectData::default()
    });
    assert!(driver.set_module_project(ModuleId(file), project.clone()));

    let before = driver.lower(file).expect("the file to be lowered");
    assert_eq!(
        before
            .item_tree()
            .scope()
            .anchor(&Name::new("Int"), Namespace::Ty),
        PathAnchor::Unresolved,
    );

    assert!(driver.remove_project(&project).is_some());
    assert!(driver.remove_project(&project).is_none());

    let after = driver.lower(file).expect("the file to be lowered");

    assert!(!Arc::ptr_eq(&before, &after), "the slot was not dropped");
    assert_eq!(import_path(&after, "Int"), "std::prelude::Int");
    assert_eq!(driver.project_graph().project_of(ModuleId(file)), None);
}

/// The standard library of the language, as a host records it, and the files it was handed.
fn with_std() -> (Driver, Vec<StdFile>) {
    let mut driver = Driver::new();
    let files = driver.use_std();

    (driver, files)
}

/// The id of a module of the library in a driver that holds it.
fn std_file(driver: &Driver, files: &[StdFile], name: &str) -> FileId {
    let at = mlkc_stdlib::modules()
        .iter()
        .position(|module| module.name == name)
        .expect("a module the library has");

    driver
        .file_id(&files[at].path)
        .expect("a module of the library to be pushed")
}

#[test]
fn the_standard_library_is_the_project_the_compiler_names() {
    let (driver, files) = with_std();
    let id = ProjectId::new(mlkc_stdlib::PROJECT);
    let project = driver
        .project_graph()
        .project(&id)
        .expect("the library to be recorded");

    for module in mlkc_stdlib::modules() {
        let module = ModuleId(std_file(&driver, &files, module.name));

        assert_eq!(driver.project_graph().project_of(module), Some(&id));
    }

    // A project is what its modules are read with, and the library gives its own modules
    // the names of the library.
    assert_eq!(project.prelude, mlkc_stdlib::prelude());
}

#[test]
fn the_standard_library_compiles_without_diagnostics() {
    let (mut driver, files) = with_std();

    for module in mlkc_stdlib::modules() {
        let file = std_file(&driver, &files, module.name);
        let diagnostics = driver
            .diagnostics(file)
            .expect("a module of the library to be parsed");

        assert!(diagnostics.is_empty(), "{}: {diagnostics:?}", module.name);
    }
}

#[test]
fn recording_the_standard_library_twice_changes_nothing() {
    let (mut driver, files) = with_std();
    let core = std_file(&driver, &files, mlkc_stdlib::CORE);
    let before = driver
        .lower(core)
        .expect("the module of the library to be lowered");

    let again = driver.use_std();

    assert_eq!(again, files, "the library lands where it landed");
    let after = driver
        .lower(core)
        .expect("the module of the library to be lowered");

    assert!(
        Arc::ptr_eq(&before, &after),
        "a library that did not change is not read again",
    );
}

/// The prelude of the language is a list of names the library exports: the language names
/// them (`std::prelude::Int`), the library declares and re-exports them, and a test is what
/// keeps the two sides of that fact from drifting apart ([ADR-0011]).
///
/// [ADR-0011]: ../../docs/adr/0011-module-prelude.md
#[test]
fn the_prelude_of_the_language_names_what_the_library_exports() {
    let (mut driver, files) = with_std();
    let prelude = driver
        .lower(std_file(&driver, &files, "prelude"))
        .expect("the module that re-exports the names to be lowered");
    let scope = prelude.item_tree().scope();

    for import in Prelude::standard().imports() {
        let anchor = scope.anchor(import.name(), Namespace::Ty);

        assert!(
            matches!(anchor, PathAnchor::Use(_)),
            "the library to export {}: {anchor:?}",
            import.name(),
        );
    }
}

#[test]
fn a_name_of_another_module_resolves_in_the_middle_of_a_project() {
    let (mut driver, main, data) = reader_and_data(DATA, READER);

    let resolution = driver
        .resolution(ModuleId(main))
        .expect("the module to resolve");

    assert!(
        resolution.diagnostics().is_empty(),
        "{:?}",
        resolution.diagnostics(),
    );

    let point = resolution
        .scope()
        .get(&Name::new("Point"))
        .expect("`Point` to be a name of the reader");
    let (entity, _) = point.ty.clone().expect("`Point` to be a type");

    assert_eq!(entity.module, ModuleId(data));
}

#[test]
fn a_body_edit_leaves_the_interface_of_a_module_where_it_was() {
    let (mut driver, _, data) = reader_and_data(DATA, READER);

    let module = ModuleId(data);
    let before = driver
        .interface(module)
        .expect("the module to have an interface");

    driver.set_file_text(path("data.mlk"), Some(DATA_EDITED.to_owned()));
    let after = driver
        .interface(module)
        .expect("the module to have an interface");

    // What a module shows is a function of its own text, and a body is not what it shows:
    // the interface a person reads is the value the driver already held.
    assert!(
        Arc::ptr_eq(&before, &after),
        "a body edit changed the interface"
    );
}

#[test]
fn a_body_edit_of_a_module_leaves_the_modules_that_read_it_where_they_were() {
    let (mut driver, main, _) = reader_and_data(DATA, READER);

    let before = driver
        .resolution(ModuleId(main))
        .expect("the module to resolve");

    driver.set_file_text(path("data.mlk"), Some(DATA_EDITED.to_owned()));

    let after = driver
        .resolution(ModuleId(main))
        .expect("the module to resolve");

    // The interface of `data` is the value the driver holds, so the walk of `main` reaches
    // it again: what `main` resolved to is what it had, and nothing of it was paid for.
    assert!(
        Arc::ptr_eq(&before, &after),
        "a body edit re-resolved a reader"
    );
}

#[test]
fn a_read_of_a_value_the_driver_holds_is_a_hit() {
    let (mut driver, file) = driver_with("main.mlk", MODULE);
    let unit = Unit::File(file);

    let first = driver.parse(file).expect("the file to be parsed");
    let taken = driver.take_stats();

    // Nothing was held, so the pass had to run, and the counters say why.
    assert_eq!(counters(&taken, Pass::Parse, &unit), Tally {
        misses: 1,
        ..Tally::default()
    });

    let second = driver.parse(file).expect("the file to be parsed");
    let taken = driver.take_stats();

    assert!(Arc::ptr_eq(&first, &second));
    assert_eq!(counters(&taken, Pass::Parse, &unit), Tally {
        hits: 1,
        ..Tally::default()
    });

    // Taking the counters is what clears them: what a host reads is what happened since.
    assert!(driver.take_stats().is_empty());
}

/// The counters of one pass for one unit, which is what a test about them reads.
fn counters(taken: &Stats, pass: Pass, unit: &Unit) -> Tally {
    taken
        .of(pass, unit)
        .unwrap_or_else(|| panic!("counters for {pass} of {unit:?}"))
}

/// A clock that stands a second still per reading, so that what a pass cost is what it read.
///
/// The readings are per thread, so what one test reads is not what another one moves.
fn ticking_clock() -> f64 {
    thread_local! {
        static NOW: Cell<f64> = const { Cell::new(0.0) };
    }

    NOW.with(|now| {
        let at = now.get();

        now.set(at + 1000.0);

        at
    })
}

#[test]
fn an_edit_of_a_body_reads_the_surface_of_its_module_again_and_keeps_it() {
    let (mut driver, _, data) = reader_and_data(DATA, READER);
    let module = ModuleId(data);

    let _ = driver
        .interface(module)
        .expect("the module to have an interface");
    let _ = driver.take_stats();

    driver.set_file_text(path("data.mlk"), Some(DATA_EDITED.to_owned()));
    let _ = driver
        .interface(module)
        .expect("the module to have an interface");
    let taken = driver.take_stats();

    // The text changed, so what is derived from it is read again --- and the surface of the
    // module came out of it unchanged, which is what the counters call a keep. The parse is a
    // value of the file and the interface one of the module, which is what the units say.
    assert_eq!(counters(&taken, Pass::Parse, &Unit::File(data)).stales, 1);
    assert_eq!(counters(&taken, Pass::Lower, &Unit::File(data)).stales, 1);
    assert_eq!(
        counters(&taken, Pass::Interface, &Unit::Module(module)).stales,
        1
    );
    assert_eq!(
        counters(&taken, Pass::Interface, &Unit::Module(module)).kept,
        1
    );
}

#[test]
fn an_edit_of_a_body_costs_its_module_and_nothing_of_its_readers() {
    let fixture = format!("//- /data.mlk\n{DATA}\n//- /main.mlk\n{READER}");
    let mut driver = std_project_of(&fixture);
    let data = file(&driver, "data.mlk");
    let main = file(&driver, "main.mlk");

    // Both modules are read once, so what the edit costs is what follows.
    let _ = driver.diagnostics(data).expect("the diagnostics of data");
    let _ = driver.diagnostics(main).expect("the diagnostics of main");
    let _ = driver.take_stats();

    driver.set_file_text(path("data.mlk"), Some(DATA_EDITED.to_owned()));
    let _ = driver.diagnostics(data).expect("the diagnostics of data");
    let taken = driver.take_stats();

    // A body edited is a check read again: the checks of the module's bodies are stale reads
    // --- what they are keyed by is a new value --- and the ones whose body did not change came
    // out the same, which is what the counters call a keep. Nothing here is a miss, and what
    // the module shows did not move either.
    assert_eq!(taken.total(Pass::Check).misses, 0);
    assert!(taken.total(Pass::Check).stales >= 2);
    assert!(taken.total(Pass::Check).kept >= 1);
    assert_eq!(taken.total(Pass::Signatures).kept, 1);

    // The module that reads `data` is read again with nothing to do: what it reads --- the
    // interface of `data` --- is the value the driver kept, so every pass is a hit. The hits
    // of the library are the many of them, since every one of its parses is handed over again.
    let _ = driver.diagnostics(main).expect("the diagnostics of main");
    let taken = driver.take_stats();

    assert!(
        taken
            .iter()
            .all(|(_, _, tally)| tally.misses == 0 && tally.stales == 0),
        "a reader of an edited body was read again: {taken:?}",
    );
    assert!(
        taken.total(Pass::Check).hits >= 1,
        "the body of the reader to be checked from a slot: {taken:?}",
    );
}

#[test]
fn a_change_of_a_project_drops_what_was_read_under_it() {
    let fixture = format!("//- /data.mlk\n{DATA}\n//- /main.mlk\n{READER}");
    let mut driver = project_of(&fixture);
    let data = ModuleId(file(&driver, "data.mlk"));

    let _ = driver.resolution(data).expect("the module to resolve");
    let _ = driver.take_stats();

    // The project gains the library, which is what its modules are read under: what was read
    // under the old prelude goes, rather than being read again.
    let mut library = ProjectData::default();
    library.dependencies.insert(
        Name::new(mlkc_stdlib::PROJECT),
        ProjectId::new(mlkc_stdlib::PROJECT),
    );
    driver.set_project(project(), library);

    let taken = driver.take_stats();

    assert!(taken.total(Pass::Lower).dropped >= 1);
    assert!(taken.total(Pass::Interface).dropped >= 1);
    assert!(taken.total(Pass::Resolution).dropped >= 1);

    // What went is named by the unit it belonged to, and a body goes by its own name.
    assert_eq!(
        counters(&taken, Pass::Lower, &Unit::File(data.0)).dropped,
        1
    );
    assert_eq!(
        counters(&taken, Pass::Resolution, &Unit::Module(data)).dropped,
        1
    );
    assert!(counters(&taken, Pass::ModuleIndex, &Unit::Project(project())).dropped >= 1);
}

#[test]
fn the_counters_name_the_unit_a_pass_was_asked_for() {
    let fixture = format!("//- /data.mlk\n{DATA}\n//- /main.mlk\n{READER}");
    let mut driver = std_project_of(&fixture);
    let data = file(&driver, "data.mlk");

    let _ = driver.diagnostics(data).expect("the diagnostics of data");
    let taken = driver.take_stats();

    // A check is a value of a body: what was read is counted for the body it was read for, and
    // the counters name both of them --- the two bodies of the module.
    let mut checked: Vec<String> = Vec::new();

    for (pass, unit, tally) in taken.iter() {
        if pass != Pass::Check || tally.is_empty() {
            continue;
        }

        let Unit::Body(owner) = unit else {
            panic!("a check to be a value of a body, not of {unit:?}");
        };

        assert_eq!(owner.module(), ModuleId(data), "a check of another module");

        checked.push(driver.unit_name(unit));
    }

    checked.sort_unstable();

    assert_eq!(checked.len(), 2, "the module has two bodies: {checked:?}");
    assert!(checked[0].ends_with(": helper"), "{}", checked[0]);
    assert!(checked[1].ends_with(": origin"), "{}", checked[1]);
}

#[test]
fn a_pass_that_runs_is_measured_by_the_clock_of_the_host() {
    let (mut driver, file) = driver_with("main.mlk", MODULE);

    driver.set_clock(ticking_clock);

    let _ = driver.parse(file).expect("the file to be parsed");
    let taken = driver.take_stats();

    // The pass was read off the clock before it ran and once after it, and the clock stands a
    // second still per reading: what it cost is one second, whatever it really took.
    assert_eq!(
        counters(&taken, Pass::Parse, &Unit::File(file)).took,
        Duration::from_secs(1),
    );

    // A hit is not timed: the pass did not run, so the clock was not read for it.
    let _ = driver.parse(file).expect("the file to be parsed");
    let taken = driver.take_stats();

    assert_eq!(
        counters(&taken, Pass::Parse, &Unit::File(file)).took,
        Duration::ZERO,
    );
}

#[test]
fn a_name_added_to_a_module_does_not_move_what_its_readers_resolved_to() {
    let (mut driver, main, data) = reader_and_data(DATA, READER);

    let module = ModuleId(data);
    let before = driver
        .resolution(ModuleId(main))
        .expect("the module to resolve");
    let interface = driver
        .interface(module)
        .expect("the module to have an interface");

    driver.set_file_text(path("data.mlk"), Some(DATA_EXTENDED.to_owned()));

    let after = driver
        .resolution(ModuleId(main))
        .expect("the module to resolve");
    let current = driver
        .interface(module)
        .expect("the module to have an interface");

    // A name added is a different interface, so the readers of `data` walk again --- and
    // the walk ends at the same entity, so the resolution it came to is the one it had.
    assert!(
        !Arc::ptr_eq(&interface, &current),
        "the interface to change"
    );
    assert!(
        Arc::ptr_eq(&before, &after),
        "a name added re-resolved a reader to something else",
    );
}

#[test]
fn the_index_of_a_project_holds_the_modules_its_files_are() {
    let (mut driver, main, data) = reader_and_data(DATA, READER);

    let index = driver
        .module_index(&project())
        .expect("the project to have an index");

    assert_eq!(index.get(&[Name::new("data")]), Some(ModuleId(data)));
    assert_eq!(index.get(&[Name::new("main")]), Some(ModuleId(main)));
    assert_eq!(index.len(), 2);
}

#[test]
fn the_def_map_of_a_project_holds_the_scopes_of_its_modules() {
    let (mut driver, main, _) = reader_and_data(DATA, READER);

    let map = driver
        .def_map(&project())
        .expect("the project to have a def map");
    let resolution = driver
        .resolution(ModuleId(main))
        .expect("the module to resolve");

    assert_eq!(map.len(), 2);
    assert!(
        Arc::ptr_eq(
            map.get(ModuleId(main))
                .expect("the reader to be in the map"),
            resolution.scope(),
        ),
        "the map to hold the scope of the resolution",
    );
}

#[test]
fn a_name_a_module_keeps_to_itself_is_told_about_as_kept() {
    let mut driver = project_of(
        "\
//- /data.mlk
type Hidden

//- /main.mlk
use project::data::Hidden

fun get() = 1
",
    );
    let main = file(&driver, "main.mlk");

    let diagnostics = driver.diagnostics(main).expect("the file to be diagnosed");

    // The walk of the reader finds no name where it ends, and telling a reader why is a
    // look at the module the name belongs to --- which is read only for such a name.
    assert_eq!(diagnostics.resolution().len(), 1, "{diagnostics:?}");

    let diagnostic = &diagnostics.resolution()[0];

    assert_eq!(diagnostic.category, Category::Resolver);
    assert_eq!(diagnostic.code, "08");
    assert_eq!(
        diagnostic.message,
        "the module `project::data` holds the name `Hidden` and does not show it",
    );
}

#[test]
fn what_a_resolution_found_travels_with_the_diagnostics_of_the_file() {
    const SOURCE: &str = "use project::data::Nope\n\nfun get() = 1\n";

    let (mut driver, main, _) = reader_and_data(DATA, SOURCE);

    let diagnostics = driver.diagnostics(main).expect("the file to be diagnosed");

    assert_eq!(diagnostics.resolution().len(), 1, "{diagnostics:?}");

    let diagnostic = &diagnostics.resolution()[0];
    let label = diagnostic.labels.first().expect("a label");

    // A resolution reports a place in the HIR, and the driver turns it into the span of the
    // text the place is written at.
    assert_eq!(diagnostic.category, Category::Resolver);
    assert_eq!(diagnostic.code, "04");
    assert_eq!(
        diagnostic.message,
        "the module `project::data` exports no name `Nope`"
    );
    assert_eq!(
        covered(SOURCE, Some(label.span.range)),
        Some("use project::data::Nope")
    );
}

#[test]
fn a_module_added_under_a_prefix_makes_the_module_that_named_it_resolve_again() {
    let mut driver = project_of(
        "\
//- /main.mlk
use project::data

use project::data::utils::Point

fun get(): Point = get()
",
    );
    let main = file(&driver, "main.mlk");

    let before = driver
        .resolution(ModuleId(main))
        .expect("the module to resolve");

    // No module of `data` is there yet, so the paths that read it name nothing.
    assert!(!before.diagnostics().is_empty());
    assert!(before.scope().get(&Name::new("Point")).is_none());

    let path = path("data/utils.mlk");
    driver.set_file_text(path.clone(), Some("pub type Point\n".to_owned()));

    let utils = driver.file_id(&path).expect("the file to have an id");
    driver.set_module_project(ModuleId(utils), project());

    let after = driver
        .resolution(ModuleId(main))
        .expect("the module to resolve");

    // A module that appeared under the prefix is a module the walk reads: the resolution of
    // a module that named the prefix is read again ([ADR-0016]).
    //
    // [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md
    assert!(after.diagnostics().is_empty(), "{:?}", after.diagnostics());
    assert_eq!(
        after
            .scope()
            .get(&Name::new("Point"))
            .and_then(|per_ns| per_ns.ty.clone())
            .map(|(entity, _)| entity.module),
        Some(ModuleId(utils)),
    );
}

#[test]
fn the_names_of_the_language_resolve_through_the_library() {
    let (mut driver, file) = driver_with_std("main.mlk", MODULE);

    let resolution = driver
        .resolution(ModuleId(file))
        .expect("the module to resolve");

    assert!(
        resolution.diagnostics().is_empty(),
        "{:?}",
        resolution.diagnostics(),
    );

    let unit = resolution
        .scope()
        .get(&Name::new("Unit"))
        .expect("`Unit` to be a name of the module");
    let (entity, _) = unit.ty.clone().expect("`Unit` to be a type");

    // The name is the one the library declares: the prelude of the language names a path of
    // `std::prelude`, and the walk follows the re-export it wrote ([ADR-0011]).
    //
    // [ADR-0011]: ../../docs/adr/0011-module-prelude.md
    assert_eq!(entity.module, std_module(&driver, mlkc_stdlib::CORE));
    assert!(
        driver
            .diagnostics(file)
            .expect("the file to be diagnosed")
            .is_empty()
    );
}

/// A path in a prelude, as the paths of a project are written.
fn prelude_of(path: &[&str]) -> Prelude {
    Prelude::from_paths([PlainPath::from_segments(
        path.iter().map(|segment| Name::new(segment)),
    )])
}

#[test]
fn the_types_of_a_module_are_resolved_from_the_signatures_it_writes() {
    let (mut driver, file) = driver_with_std("main.mlk", "fun double(value: Int): Int = value\n");
    let module = ModuleId(file);
    let types = driver
        .module_types(module)
        .expect("the signatures of the module to resolve");
    let lowered = driver.lower(file).expect("the file to be lowered");
    let double = lowered
        .item_tree()
        .entities()
        .find(|(item, _)| item.name() == Some(&Name::new("double")))
        .map(|(item, _)| EntityLoc { module, item })
        .expect("the module to declare `double`");

    // The signature is a value: `Int` is the class the library declares, and the type of the
    // function reads as the types it writes ([ADR-0017]).
    //
    // [ADR-0017]: ../../docs/adr/0017-resolved-types.md
    assert_eq!(
        types.get(&double).map(ToString::to_string),
        Some("(Int) -> Int".to_owned()),
    );
}

#[test]
fn a_body_is_checked_against_the_signatures_of_its_module() {
    let (mut driver, file) =
        driver_with_std("main.mlk", "fun double(value: Int): Int = value + 1\n");
    let owner = body_of(&mut driver, file, "double");
    let lowered = driver.lower(file).expect("the file to be lowered");
    let body = lowered
        .bodies()
        .iter()
        .find(|body| body.owner() == &owner)
        .expect("the body of `double`");
    let checked = driver.check(&owner).expect("the body to check");

    assert_eq!(
        checked
            .expr_type(body.body().body.root())
            .map(ToString::to_string),
        Some("Int".to_owned()),
    );
}

#[test]
fn a_type_mistake_of_a_body_travels_with_the_diagnostics_of_the_file() {
    const SOURCE: &str = "fun main(): Unit =\n    \"text\"\n";

    let (mut driver, file) = driver_with_std("main.mlk", SOURCE);
    let diagnostics = driver.diagnostics(file).expect("the file to be diagnosed");

    assert_eq!(diagnostics.types().len(), 1, "{diagnostics:?}");

    let diagnostic = &diagnostics.types()[0];
    let label = diagnostic.labels.first().expect("a label");

    // A check reports a place in the HIR, and the driver turns it into the span of the text
    // the place is written at, with the words that belong under it ([ADR-0009]).
    //
    // [ADR-0009]: ../../docs/adr/0009-pass-contract.md
    assert_eq!(diagnostic.category, Category::TypeChecker);
    assert_eq!(diagnostic.code, "09");
    assert_eq!(
        diagnostic.message,
        "a value of type `String` is where a value of type `Unit` belongs",
    );
    assert_eq!(label.message, "expected `Unit`, found `String`");
    assert!(label.primary);
    assert_eq!(covered(SOURCE, Some(label.span.range)), Some("\"text\""));
}

#[test]
fn a_signature_that_is_not_written_is_reported_at_its_declaration() {
    const SOURCE: &str = "fun helper() = 1\n";

    let (mut driver, file) = driver_with_std("main.mlk", SOURCE);
    let diagnostics = driver.diagnostics(file).expect("the file to be diagnosed");

    assert_eq!(diagnostics.types().len(), 1, "{diagnostics:?}");

    let diagnostic = &diagnostics.types()[0];
    let label = diagnostic.labels.first().expect("a label");

    // The declaration writes no result type, so there is no type to point at and the
    // declaration itself is what is marked.
    assert_eq!(diagnostic.category, Category::TypeChecker);
    assert_eq!(diagnostic.code, "01");
    assert_eq!(
        diagnostic.message,
        "`fun helper` does not declare the type of its result",
    );
    assert_eq!(label.message, "the type of this result is not written");
    assert_eq!(
        covered(SOURCE, Some(label.span.range)),
        Some("fun helper() = 1")
    );
    assert!(!diagnostic.notes.is_empty(), "the deferral is explained");
}

#[test]
fn a_name_a_module_keeps_to_itself_is_told_about_as_kept_from_a_body() {
    const MAIN: &str = "\
//- /data.mlk
fun hidden(): Int = 1

//- /main.mlk
fun get(): Int = project::data::hidden()
";

    let mut driver = std_project_of(MAIN);
    let main = file(&driver, "main.mlk");
    let diagnostics = driver.diagnostics(main).expect("the file to be diagnosed");

    // The paths of a body are walked by the check and never by the resolution ([ADR-0016]),
    // so the walk that ends at a name a module keeps to itself is the check's --- and the
    // look that tells why is the one the rendering of a resolution takes.
    //
    // [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md
    assert_eq!(diagnostics.resolution().len(), 0, "{diagnostics:?}");
    assert_eq!(diagnostics.types().len(), 1, "{diagnostics:?}");

    let diagnostic = &diagnostics.types()[0];

    assert_eq!(diagnostic.category, Category::TypeChecker);
    assert_eq!(diagnostic.code, "11");
    assert_eq!(
        diagnostic.message,
        "the module `project::data` holds the name `hidden` and does not show it",
    );
}

#[test]
fn a_body_edit_leaves_the_signatures_and_the_other_bodies_where_they_were() {
    const SOURCE: &str = "\
//- /main.mlk
fun first(): Int = 1

fun second(): Int = 2
";

    let mut driver = std_project_of(SOURCE);
    let main = file(&driver, "main.mlk");
    let module = ModuleId(main);

    let before_types = driver
        .module_types(module)
        .expect("the signatures of the module to resolve");
    let owner = body_of(&mut driver, main, "second");
    let before = driver.check(&owner).expect("the body to check");

    driver.set_file_text(
        path("main.mlk"),
        Some("fun first(): Int = 42\n\nfun second(): Int = 2\n".to_owned()),
    );

    let after_types = driver
        .module_types(module)
        .expect("the signatures of the module to resolve");
    let owner = body_of(&mut driver, main, "second");
    let after = driver.check(&owner).expect("the body to check");

    // A body edit cannot change what a module shows, and it does not change the check of a
    // body it did not edit: a surface is resolved from signatures alone, and a check that
    // ends up equal to the one the driver held is the one it held ([ADR-0017]).
    //
    // [ADR-0017]: ../../docs/adr/0017-resolved-types.md
    assert!(
        Arc::ptr_eq(&before_types, &after_types),
        "a body edit moved the types of the module",
    );
    assert!(
        Arc::ptr_eq(&before, &after),
        "a body edit re-checked a body that did not change",
    );
}

#[test]
fn the_mir_of_a_body_is_the_cfg_form_of_it() {
    let (mut driver, file) =
        driver_with_std("main.mlk", "fun double(value: Int): Int = value + 1\n");
    let owner = body_of(&mut driver, file, "double");
    let mir = driver.mir(&owner).expect("the body to have MIR");

    // The construction is one walk of the checked body: the body is the one the entity
    // owns, a parameter is a value the body is entered with, and the CFG form holds the
    // slots the SSA construction will give definitions of their own ([ADR-0019]).
    //
    // [ADR-0019]: ../../docs/adr/0019-mir.md
    assert_eq!(mir.owner, owner);
    assert_eq!(mir.params.len(), 1);
    assert_eq!(mir.validate_cfg(), Ok(()));
}

#[test]
fn a_second_pull_of_mir_is_the_value_the_first_one_returned() {
    let (mut driver, file) =
        driver_with_std("main.mlk", "fun double(value: Int): Int = value + 1\n");
    let owner = body_of(&mut driver, file, "double");

    let first = driver.mir(&owner).expect("the body to have MIR");
    let second = driver.mir(&owner).expect("the body to have MIR");

    assert!(Arc::ptr_eq(&first, &second), "the slot was built twice");
}

#[test]
fn a_body_that_does_not_check_clean_is_not_lowered() {
    let (mut driver, file) = driver_with_std("main.mlk", "fun main(): Unit = 1\n");
    let owner = body_of(&mut driver, file, "main");

    // A body whose check reported a mistake has no MIR: there is no meaning to lower, and
    // codegen never meets an expression whose meaning is a mistake ([ADR-0019]).
    //
    // [ADR-0019]: ../../docs/adr/0019-mir.md
    assert!(driver.check(&owner).is_some(), "the body to be checked");
    assert!(
        driver.mir(&owner).is_none(),
        "a body with a mistake to be lowered"
    );
    assert!(driver.mir_ssa(&owner).is_none(), "an SSA form of a mistake");
}

#[test]
fn a_literal_out_of_the_range_of_int_is_the_checks_mistake() {
    const SOURCE: &str = "fun big(): Int = 1099511627776\n";

    let (mut driver, file) = driver_with_std("main.mlk", SOURCE);
    let owner = body_of(&mut driver, file, "big");
    let diagnostics = driver.diagnostics(file).expect("the file to be diagnosed");

    // The range of a literal is the meaning of the type it is written with, so the check is
    // what reports it, and the construction of MIR reads it as an invariant ([ADR-0018]).
    //
    // [ADR-0018]: ../../docs/adr/0018-values-as-words.md
    assert_eq!(diagnostics.types().len(), 1, "{diagnostics:?}");

    let diagnostic = &diagnostics.types()[0];
    let label = diagnostic.labels.first().expect("a label");

    assert_eq!(diagnostic.category, Category::TypeChecker);
    assert_eq!(diagnostic.code, "13");
    assert_eq!(
        diagnostic.message,
        "the integer literal `1099511627776` is outside the 31-bit range of `Int`",
    );
    assert!(label.primary);
    assert_eq!(
        covered(SOURCE, Some(label.span.range)),
        Some("1099511627776")
    );

    // A body with a mistake has no MIR at all ([ADR-0019]).
    //
    // [ADR-0019]: ../../docs/adr/0019-mir.md
    assert!(
        driver.mir(&owner).is_none(),
        "a mistaken body to be lowered"
    );
}

#[test]
fn a_file_that_did_not_parse_has_no_mir() {
    // The body of a module that broke the parser may hold an expression that is not there,
    // and MIR has no meaning for one: the driver lowers nothing for such a file.
    let (mut driver, file) = driver_with_std("main.mlk", "fun main(): Unit =\n");
    let owner = body_of(&mut driver, file, "main");

    assert!(driver.parse(file).is_some_and(|parse| parse.has_errors()));
    assert!(driver.mir(&owner).is_none(), "MIR of a file that broke");
    assert!(driver.mir_ssa(&owner).is_none(), "SSA of a file that broke");
}

#[test]
fn the_ssa_form_of_a_body_is_reached_by_a_pass() {
    let (mut driver, file) =
        driver_with_std("main.mlk", "fun double(value: Int): Int = value + 1\n");
    let owner = body_of(&mut driver, file, "double");

    let cfg = driver.mir(&owner).expect("the body to have MIR");
    let ssa = driver
        .mir_ssa(&owner)
        .expect("the body to have an SSA form");
    let again = driver
        .mir_ssa(&owner)
        .expect("the body to have an SSA form");

    assert!(Arc::ptr_eq(&ssa, &again), "the slot was built twice");
    assert!(
        !Arc::ptr_eq(&cfg, &ssa),
        "the SSA form is a pass over the CFG form",
    );
    assert_eq!(cfg.validate_cfg(), Ok(()));
    assert_eq!(ssa.validate_ssa(), Ok(()));
}

#[test]
fn a_body_edit_leaves_the_mir_of_the_bodies_that_did_not_change_where_they_were() {
    const SOURCE: &str = "\
//- /main.mlk
fun first(): Int = 1

fun second(): Int = 2
";

    let mut driver = std_project_of(SOURCE);
    let main = file(&driver, "main.mlk");

    let owner = body_of(&mut driver, main, "second");
    let before = driver.mir(&owner).expect("the body to have MIR");
    let before_ssa = driver
        .mir_ssa(&owner)
        .expect("the body to have an SSA form");

    driver.set_file_text(
        path("main.mlk"),
        Some("fun first(): Int = 3\n\nfun second(): Int = 2\n".to_owned()),
    );

    let owner = body_of(&mut driver, main, "second");
    let after = driver.mir(&owner).expect("the body to have MIR");
    let after_ssa = driver
        .mir_ssa(&owner)
        .expect("the body to have an SSA form");

    // A body edit that does not move a body cannot change its MIR: the lowering of a body
    // that ends up equal to the one the driver held is the one it held, spans included
    // ([ADR-0019]).
    //
    // [ADR-0019]: ../../docs/adr/0019-mir.md
    assert!(
        Arc::ptr_eq(&before, &after),
        "a body edit re-lowered a body that did not move",
    );
    assert!(
        Arc::ptr_eq(&before_ssa, &after_ssa),
        "a body edit rebuilt the SSA form of a body that did not move",
    );
}

#[test]
fn a_driver_without_the_library_checks_no_types() {
    // The classes of the language are the library's declaration, and a driver whose host
    // recorded no library has none: what a literal is is nothing the checker can say.
    let (mut driver, file) = driver_with("main.mlk", "fun main(): Int = 1\n");

    assert!(driver.module_types(ModuleId(file)).is_none());
    assert!(
        driver
            .diagnostics(file)
            .expect("the file to be diagnosed")
            .types()
            .is_empty()
    );
}

/// A project a test records.
fn project() -> ProjectId {
    ProjectId::new("the-project")
}

/// The first token of a kind in a parse, which is what shows what the parses shared.
fn first_token(parse: &Parse, kind: SyntaxKind) -> SyntaxToken {
    parse
        .syntax()
        .descendants_tokens(Direction::Next)
        .find(|token| token.kind() == kind)
        .expect("the tree to hold a token of that kind")
}

#[test]
fn a_revision_of_a_file_shares_the_tokens_it_did_not_edit() {
    let (mut driver, file) = driver_with("main.mlk", MODULE);
    let before = driver.parse(file).expect("the file to be parsed");

    // A function written after the module: every token of the module is what it was.
    let text = format!("{MODULE}\nfun added(): Unit =\n    1\n");
    assert!(driver.set_file_text(path("main.mlk"), Some(text.clone())));

    let after = driver.parse(file).expect("the file to be parsed");

    assert_eq!(after.syntax().to_string(), text);
    assert!(
        first_token(&before, FUN_KW).key() == first_token(&after, FUN_KW).key(),
        "the parse was not built through the nodes the parse before it left"
    );
}

#[test]
fn the_parses_of_two_files_share_no_nodes() {
    // The files are written the same way, and each is parsed through the nodes of its own
    // parses: what one file shares is with the revision before it, and not with another
    // file. That is the price of parsing the files of a project at once.
    let (mut driver, first) = driver_with("one.mlk", MODULE);
    driver.set_file_text(path("two.mlk"), Some(MODULE.to_string()));

    let second = driver
        .file_id(&path("two.mlk"))
        .expect("the file to have an id");

    let one = driver.parse(first).expect("the file to be parsed");
    let two = driver.parse(second).expect("the file to be parsed");

    assert!(first_token(&one, FUN_KW).key() != first_token(&two, FUN_KW).key());
}

#[test]
fn the_driver_and_the_trees_it_hands_out_cross_threads() {
    let (mut driver, file) = driver_with("main.mlk", MODULE);
    let before = driver.parse(file).expect("the file to be parsed");

    // A tree is immutable and shared, so it is read anywhere: a host answers the requests
    // that only read from the thread that asked, and the parsing to the thread that owns
    // the driver.
    let text = std::thread::spawn({
        let before = before.clone();

        move || before.syntax().to_string()
    })
    .join()
    .expect("the thread not to panic");

    assert_eq!(text, MODULE);

    // The driver owns the green nodes of every file it parsed, and they move with it: a
    // host may hand it to another thread, which is what lets the files of a project be
    // parsed at once.
    let (mut driver, text) = std::thread::spawn(move || {
        let parse = driver.parse(file).expect("the file to be parsed");

        (driver, parse.syntax().to_string())
    })
    .join()
    .expect("the thread not to panic");

    assert_eq!(text, MODULE);

    // The driver comes back as it was: the parse of the file is the value it already was.
    let after = driver.parse(file).expect("the file to be parsed");

    assert!(Arc::ptr_eq(&before, &after), "the slot was built again");
}

#[test]
fn a_changed_file_is_parsed_again_and_the_old_value_keeps_its_text() {
    let (mut driver, file) = driver_with("main.mlk", MODULE);
    let before = driver.parse(file).expect("the file to be parsed");

    driver.set_file_text(path("main.mlk"), Some(BROKEN.to_string()));
    let after = driver.parse(file).expect("the file to be parsed");

    assert!(!Arc::ptr_eq(&before, &after), "the slot was not rebuilt");
    assert_eq!(after.syntax().to_string(), BROKEN);
    assert_eq!(
        before.syntax().to_string(),
        MODULE,
        "a value is immutable, so it keeps the text it was parsed from"
    );
}

#[test]
fn a_deleted_file_has_no_parse() {
    let (mut driver, file) = driver_with("main.mlk", MODULE);

    driver.set_file_text(path("main.mlk"), None);

    assert_eq!(driver.file_state(file), FileState::Deleted);
    assert!(driver.parse(file).is_none());
}

#[test]
fn a_file_that_is_not_text_has_no_parse() {
    let mut driver = Driver::new();
    let path = path("binary.mlk");

    driver.set_file_contents(path.clone(), Some(vec![0xFF, 0xFE]));
    let file = driver.file_id(&path).expect("the file to have an id");

    assert_eq!(driver.file_state(file), FileState::Unreadable);
    assert!(driver.parse(file).is_none());
}

#[test]
fn the_diagnostics_of_a_broken_module_travel_with_the_parse() {
    let (mut driver, file) = driver_with("main.mlk", BROKEN);

    let parse = driver.parse(file).expect("the file to be parsed");

    assert!(
        parse.has_errors(),
        "a missing `in` is a mistake the parser reports"
    );

    let diagnostics = driver.diagnostics(file).expect("the file to be parsed");
    let diagnostic = diagnostics
        .iter()
        .next()
        .expect("the parse to report something");

    assert_eq!(diagnostic.level, Level::Error);
    assert_eq!(diagnostic.category, Category::Parser);
    assert_eq!(diagnostic.labels.first().expect("a label").span.file, file);
}

#[test]
fn the_diagnostics_are_converted_once() {
    let (mut driver, file) = driver_with("main.mlk", BROKEN);

    let first = driver.diagnostics(file).expect("the file to be parsed");
    let second = driver.diagnostics(file).expect("the file to be parsed");

    // Nothing is rendered again: a part of the diagnostics is the value the stage left.
    assert!(
        Arc::ptr_eq(first.parse(), second.parse())
            && Arc::ptr_eq(first.lowering(), second.lowering())
            && Arc::ptr_eq(first.resolution(), second.resolution()),
        "the conversion ran twice",
    );
}

#[test]
fn the_diagnostics_follow_the_text_and_the_old_ones_stay_as_they_were() {
    let (mut driver, file) = driver_with_std("main.mlk", BROKEN);
    let broken = driver.diagnostics(file).expect("the file to be parsed");

    assert!(!broken.is_empty());

    driver.set_file_text(path("main.mlk"), Some(MODULE.to_string()));
    let fixed = driver.diagnostics(file).expect("the file to be parsed");

    assert!(fixed.is_empty(), "the module parses cleanly now");
    assert!(
        !broken.is_empty(),
        "a value does not change under the one that holds it"
    );
}

#[test]
fn the_line_index_follows_the_text() {
    let (mut driver, file) = driver_with("main.mlk", MODULE);
    let index = driver.line_index(file).expect("the file to have an index");

    assert!(Arc::ptr_eq(
        &index,
        &driver.line_index(file).expect("the file to have an index")
    ));
    assert_eq!(
        index.line_count(),
        7,
        "six lines of source and the line the trailing break makes"
    );
    assert_eq!(
        &MODULE[index.line_range(5).expect("a sixth line")],
        "    println-int(x + 20)"
    );
    assert_eq!(index.line_col(MODULE.text_len()), LineCol {
        line: 6,
        col: 0
    });

    driver.set_file_text(path("main.mlk"), Some("x\n".to_string()));
    let rebuilt = driver.line_index(file).expect("the file to have an index");

    assert!(!Arc::ptr_eq(&index, &rebuilt), "the index was not rebuilt");
    assert_eq!(rebuilt.line_count(), 2);
}

#[test]
fn take_changes_reports_the_net_effect_since_the_last_call() {
    let mut driver = Driver::new();
    let path = path("main.mlk");
    let others = ["one", "two"];

    driver.set_file_text(path.clone(), Some(others[0].to_string()));

    let created = driver.take_changes();
    assert_eq!(created.len(), 1);
    assert_eq!(created[0].change, Change::Create);
    assert!(driver.take_changes().is_empty(), "a drain drains");

    driver.set_file_text(path.clone(), Some(others[0].to_string()));
    assert!(
        driver.take_changes().is_empty(),
        "the same contents are not a change"
    );

    driver.set_file_text(path.clone(), Some(others[1].to_string()));
    assert_eq!(driver.take_changes()[0].change, Change::Modify);

    driver.set_file_text(path, None);
    assert_eq!(driver.take_changes()[0].change, Change::Delete);
}
