//! The spec tests of a project: the diagnostics the driver reports for its modules.
//!
//! Every fixture of the corpus has a test of its own, declared in `project_specs.rs`, and a
//! snapshot next to it that holds what the driver reported about every module of it: the
//! messages, their places, and the notes they carry.
//!
//! The dumps of the passes are not here: what a stage makes of a project is read by the suite
//! of the crate that owns the stage, which drives the same corpus through this same driver.
//! What is left to the driver is what it adds --- the diagnostics it renders for a host, and
//! the check that the pipeline ran without an internal exception.
//!
//! A fixture is a file in the `mlkc-fixture` format or a directory of modules, and the corpus
//! is shared by the suites of the pipeline ([`mlkc_fixture::projects_dir`]), so a project
//! added there is asserted by every stage at once.

// The file is a harness of the suite, and cargo compiles it as a test target of its own as
// well: in that target nothing calls it, which is what the allowance is for. An `allow`
// rather than an `expect` because the target that includes the file does call it, and an
// expectation that is not fulfilled is an error too.
#![allow(
    dead_code,
    reason = "the harness is a test target of its own, where nothing calls it"
)]

use std::fmt::Write as _;

use mlkc_diagnostics::Diagnostic;
use mlkc_driver::{Diagnostics, Driver};
use mlkc_fixture::Module;
use mlkc_hir_def::{ModuleId, Name, ProjectData, ProjectId};
use mlkc_vfs::VfsPath;

/// The directory of the snapshots, relative to the file that asserts them.
///
/// A snapshot is written next to the file that asserts it, and the path handed to insta is
/// relative to `tests/project_spec.rs`.
pub(crate) const SPECS_DIR: &str = "specs";

/// The name of the project every fixture is compiled as.
///
/// A module of the project calls it by the keyword `project` and by nothing else: the name a
/// manifest declares a project under is not a name a module of it may write ([ADR-0016]). The
/// standard library is the other project of the world, which every fixture depends on.
///
/// [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md
const PROJECT: &str = "app";

/// Runs the fixture at `fixture`, a project of the corpus, and checks it against its snapshot.
pub(crate) fn run(fixture: &str) {
    let modules = mlkc_fixture::read(fixture);
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

/// The projects the corpus holds, by the names the tests name them by.
///
/// A project that is a directory is named by the directory, and one that is a file by the file
/// without the extension of a module.
pub(crate) fn fixtures() -> Vec<String> {
    mlkc_fixture::projects()
}
