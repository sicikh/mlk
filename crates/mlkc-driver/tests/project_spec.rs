//! The harness of the project spec tests.
//!
//! A fixture is a project, written one of two ways. A fixture that is a file holds its modules
//! in the fixture format ([`mlkc_fixture`]): each module is headed by the place it stands at,
//! `//- /main.mlk`, and what follows is its source. A fixture that is a directory holds a file
//! per module, and the place of a module is the path of its file under the directory: the file
//! `data/utils.mlk` is the module `project::data::utils` either way.
//!
//! A fixture is compiled by a driver that holds the standard library, and the project depends on
//! it, so the names of the language resolve.
//!
//! The snapshot holds, for every module of the project, the surface it was lowered to, the scope
//! it resolved to, the types its signatures and bodies were checked to, the MIR of every body
//! the front end read clean, in both of its forms, and what the stages reported. A snapshot is
//! part of changing how a project is read:
//! `INSTA_UPDATE=always cargo test -p mlkc-driver` rewrites them, and the diff of the snapshots
//! is what a review reads ([ADR-0006], [ADR-0016], [ADR-0017], [ADR-0019]).
//!
//! [ADR-0006]: ../../docs/adr/0006-snapshot-testing.md
//! [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md
//! [ADR-0017]: ../../docs/adr/0017-resolved-types.md
//! [ADR-0019]: ../../docs/adr/0019-mir.md

use std::{
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
};

use mlkc_diagnostics::Diagnostic;
use mlkc_driver::{Diagnostics, Driver};
use mlkc_fixture::Module;
use mlkc_hir_def::{ModuleId, Name, ProjectData, ProjectId};
use mlkc_vfs::VfsPath;

/// The directory of the tests of this crate.
///
/// The fixtures are read from here rather than from the working directory of the test, so that
/// a test does not depend on where it was started from.
const TESTS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests");

/// The directory of the fixtures, relative to [`TESTS_DIR`].
///
/// A snapshot is written next to the fixture it describes, and the path of a snapshot that
/// insta resolves is relative to the file that asserts it --- here, `tests/project_spec.rs`.
/// The path handed to insta is therefore spelled relative to the directory of the tests, while
/// the fixtures themselves are read from [`TESTS_DIR`].
pub(crate) const SPECS_DIR: &str = "specs";

/// The name of the project every fixture is compiled as.
///
/// A module of the project calls it by the keyword `project` and by nothing else: the name a
/// manifest declares a project under is not a name a module of it may write ([ADR-0016]). The
/// standard library is the other project of the world, which every fixture depends on.
///
/// [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md
const PROJECT: &str = "app";

/// The extension of a fixture that holds its own modules, and of a module of a directory.
const MODULE_EXTENSION: &str = "mlk";

/// Runs the fixture at `fixture`, a path relative to [`SPECS_DIR`], and checks it against its
/// snapshot.
///
/// A fixture is a file that holds the modules of the project, or a directory that holds a file
/// per module: see the module documentation.
pub(crate) fn run(fixture: &str) {
    let modules = modules_of(fixture);
    let (mut driver, project) = project(&modules);

    let index = driver
        .module_index(&project)
        .expect("a fixture to be a project the driver holds");

    let mut snapshot = String::new();

    // The index reads in the order of the module paths, so a snapshot reads in that order
    // whatever order the files of the fixture were read in.
    for (path, module) in index.iter() {
        let path = path.iter().map(Name::as_str).collect::<Vec<_>>().join("::");

        writeln!(snapshot, "## `project::{path}`").expect("writing to a string to never fail");
        snapshot.push('\n');

        let lowered = driver
            .lower(module.0)
            .expect("a module of a fixture to be lowered");

        snapshot.push_str("### Item Tree\n\n```\n");
        snapshot.push_str(&mlkc_hir_def::dump::item_tree(lowered.item_tree()));
        snapshot.push_str("```\n\n");

        // A checked body is read by the positions of its nodes, and the HIR dump is what says
        // which node a position is: the ids of the two are the ids of one body.
        snapshot.push_str("### Bodies\n\n");

        if lowered.bodies().is_empty() {
            snapshot.push_str("No bodies.\n\n");
        } else {
            for body in lowered.bodies() {
                writeln!(snapshot, "`{:?}`", body.owner().item)
                    .expect("writing to a string to never fail");
                snapshot.push_str("\n```\n");
                snapshot.push_str(&mlkc_hir_def::dump::body(body.owner(), &body.body().body));
                snapshot.push_str("```\n\n");
            }
        }

        let resolution = driver
            .resolution(module)
            .expect("a module of a fixture to resolve");

        snapshot.push_str("### Resolution\n\n```\n");
        snapshot.push_str(&mlkc_hir_def::dump::resolution(resolution.scope()));
        snapshot.push_str("```\n\n");

        // The types of the module are resolved from the signatures it writes, before any body
        // is checked: the surface is what its readers read ([ADR-0017]).
        //
        // [ADR-0017]: ../../docs/adr/0017-resolved-types.md
        let types = driver
            .module_types(module)
            .expect("a module of a fixture to have its types resolved");

        snapshot.push_str("### Type Surface\n\n```\n");
        snapshot.push_str(&mlkc_hir_ty::dump::module_types(&types));
        snapshot.push_str("```\n\n");

        snapshot.push_str("### Checked Bodies\n\n");

        if lowered.bodies().is_empty() {
            snapshot.push_str("No bodies.\n\n");
        } else {
            for body in lowered.bodies() {
                let checked = driver
                    .check(body.owner())
                    .expect("a body of a fixture to be checked");

                writeln!(snapshot, "`{:?}`", body.owner().item)
                    .expect("writing to a string to never fail");
                snapshot.push_str("\n```\n");
                snapshot.push_str(&mlkc_hir_ty::dump::checked_body(&checked));
                snapshot.push_str("```\n\n");
            }
        }

        // The MIR of a body is read in the form the stage left it in: the CFG form the lowering
        // produces, and the SSA form the construction after it produces ([ADR-0019]).
        //
        // [ADR-0019]: ../../docs/adr/0019-mir.md
        snapshot.push_str("### MIR\n\n");

        if lowered.bodies().is_empty() {
            snapshot.push_str("No bodies.\n\n");
        } else {
            for body in lowered.bodies() {
                writeln!(snapshot, "`{:?}`", body.owner().item)
                    .expect("writing to a string to never fail");
                snapshot.push('\n');

                // A body the front end or the check reported a mistake about is not lowered,
                // and a host is told so by the diagnostics of the file ([ADR-0019]).
                let Some(mir) = driver.mir(body.owner()) else {
                    snapshot.push_str("Not lowered: the front end reported a mistake.\n\n");
                    continue;
                };

                snapshot.push_str("CFG form:\n\n```\n");
                snapshot.push_str(&mlkc_mir::dump::body(&mir));
                snapshot.push_str("```\n\n");

                let ssa = driver
                    .mir_ssa(body.owner())
                    .expect("a body that is lowered to have an SSA form");

                snapshot.push_str("SSA form:\n\n```\n");
                snapshot.push_str(&mlkc_mir::dump::body(&ssa));
                snapshot.push_str("```\n\n");

                // The LIR is what the WASM back end lowers the SSA form into: the target's own
                // instructions, and where every value that needs storage lives ([ADR-0022]).
                //
                // [adr-0022]: ../../docs/adr/0022-wasm-lir.md
                snapshot.push_str("LIR:\n\n");

                match driver.lir(body.owner()) {
                    Some(lir) => {
                        snapshot.push_str("```\n");
                        snapshot.push_str(&mlkc_lir_wasm::dump::body(&lir));
                        snapshot.push_str("```\n\n");
                    },
                    None => {
                        snapshot.push_str("Not lowered: the module of the body is not whole.\n\n");
                    },
                }
            }
        }

        snapshot.push_str("### Diagnostics\n\n");

        let diagnostics = driver
            .diagnostics(module.0)
            .expect("a module of a fixture to be diagnosed");

        write_diagnostics(&mut snapshot, &diagnostics);
    }

    // A compiler that bugged has no business writing a snapshot: the report says what was being
    // computed when the pass panicked, and failing here is how a review sees it.
    if let Some(report) = driver.ice() {
        panic!("the driver bugged:\n{report}");
    }

    insta::with_settings!({
        prepend_module_to_snapshot => false,
        snapshot_path => SPECS_DIR,
    }, {
        insta::assert_snapshot!(fixture, snapshot);
    });
}

/// The modules of a fixture, in the order they are read.
///
/// A fixture that is a file holds its modules in the fixture format; one that is a directory
/// holds a file per module, and the place of a module is the path of its file under the
/// directory.
fn modules_of(fixture: &str) -> Vec<Module> {
    let directory = fixtures_dir().join(fixture);

    if directory.is_dir() {
        let mut modules = Vec::new();
        collect_modules(&directory, &directory, &mut modules);
        modules.sort_by(|left, right| left.place.cmp(&right.place));

        return modules;
    }

    let path = directory.with_extension(MODULE_EXTENSION);
    let source = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));

    mlkc_fixture::modules(&source)
}

/// The driver a fixture is compiled by: the standard library, the project that depends on it,
/// and the modules of the fixture.
fn project(modules: &[Module]) -> (Driver, ProjectId) {
    let mut driver = Driver::new();
    driver.use_std();

    let project = ProjectId::new(PROJECT);
    let mut data = ProjectData::default();
    data.dependencies.insert(
        Name::new(mlkc_stdlib::PROJECT),
        ProjectId::new(mlkc_stdlib::PROJECT),
    );
    driver.set_project(project.clone(), data);

    for module in modules {
        let path = VfsPath::new_virtual_path(module.place.clone());
        driver.set_file_text(path.clone(), Some(module.source.clone()));

        let file = driver
            .file_id(&path)
            .expect("a module of a fixture to be pushed");
        driver.set_module_project(ModuleId(file), project.clone());
    }

    (driver, project)
}

/// The lines of one section of diagnostics, or a word for a section that holds none.
fn write_diagnostics(text: &mut String, diagnostics: &Diagnostics) {
    if diagnostics.is_empty() {
        text.push_str("No diagnostics.\n\n");
        return;
    }

    for diagnostic in diagnostics.iter() {
        writeln!(text, "{}", diagnostic_line(diagnostic))
            .expect("writing to a string to never fail");
    }

    text.push('\n');
}

/// The summary of a diagnostic: what it is, what it says, where it points, and what it adds.
///
/// The detail of what a diagnostic renders --- the text of the line it marks, and the marks
/// under it --- is what a host shows a person; what a diff of a snapshot needs is which
/// diagnostic it is, where it is, and what it says there.
fn diagnostic_line(diagnostic: &Diagnostic) -> String {
    let mut line = format!(
        "{}[{}]: {}",
        diagnostic.category.as_str(),
        diagnostic.code,
        diagnostic.message,
    );

    for label in &diagnostic.labels {
        let place = format!(
            "{}..{}",
            u32::from(label.span.range.start()),
            u32::from(label.span.range.end()),
        );

        if label.message.is_empty() {
            write!(line, "  @{place}").expect("writing to a string to never fail");
        } else {
            write!(line, "  @{place} ({})", label.message)
                .expect("writing to a string to never fail");
        }
    }

    if diagnostic.labels.is_empty() {
        line.push_str("  @no place");
    }

    for note in &diagnostic.notes {
        write!(line, "\n  note: {note}").expect("writing to a string to never fail");
    }

    line
}

/// The directory the fixtures live in.
fn fixtures_dir() -> PathBuf {
    Path::new(TESTS_DIR).join(SPECS_DIR)
}

/// The path of a fixture, given as a path relative to [`SPECS_DIR`].
pub(crate) fn fixture_path(fixture: &str) -> PathBuf {
    fixtures_dir().join(fixture)
}

/// The modules of a fixture that is a directory: the place of each file in the project, and its
/// source.
///
/// The place of a file is the path of it under the directory of the fixture, which is what the
/// module is called by: the file `data/utils.mlk` is the module `project::data::utils`.
fn collect_modules(root: &Path, directory: &Path, modules: &mut Vec<Module>) {
    let entries = fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", directory.display()));

    for entry in entries {
        let path = entry.expect("the entry to be readable").path();

        if path.is_dir() {
            collect_modules(root, &path, modules);
        } else if path.extension() == Some(MODULE_EXTENSION.as_ref()) {
            let place = path
                .strip_prefix(root)
                .expect("a file of a fixture to be under the fixture")
                .to_str()
                .expect("the place of a module to be UTF-8")
                .replace('\\', "/");
            let source = fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));

            modules.push(Module {
                place: format!("/{place}"),
                source,
            });
        }
    }
}

/// The fixtures the spec tests cover, as the paths the tests name them by, relative to
/// [`SPECS_DIR`].
///
/// A fixture that is a directory is named by the directory, and one that is a file by the file
/// without the extension of a module: the fixture `imports.mlk` and the fixture `imports/` are
/// both named `imports`.
pub(crate) fn fixtures() -> Vec<PathBuf> {
    let mut fixtures = Vec::new();
    let entries = fs::read_dir(fixtures_dir())
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", fixtures_dir().display()));

    for entry in entries {
        let path = entry.expect("the entry to be readable").path();

        if path.is_dir() {
            fixtures.push(path);
        } else if path.extension() == Some(MODULE_EXTENSION.as_ref()) {
            fixtures.push(path.with_extension(""));
        }
    }

    fixtures.sort();
    fixtures
}
