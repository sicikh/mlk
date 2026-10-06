//! The harness of the spec tests of the resolution.
//!
//! A fixture of the shared corpus (`mlkc-fixture`) is compiled by the driver --- a
//! dev-dependency of this crate --- and the snapshot holds what this stage made of it: the scope
//! every module of the project resolved to ([ADR-0016]). No other stage is in the snapshot;
//! every one of them has a suite in the crate that owns it.
//!
//! [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md

// The file is a harness of the suite, and cargo compiles it as a test target of its own as
// well: in that target nothing calls it, which is what the allowance is for. An `allow`
// rather than an `expect` because the target that includes the file does call it, and an
// expectation that is not fulfilled is an error too.
#![allow(
    dead_code,
    reason = "the harness is a test target of its own, where nothing calls it"
)]

use std::fmt::Write as _;

use mlkc_driver::Driver;
use mlkc_fixture::Module;
use mlkc_hir_def::{ModuleId, Name, ProjectData, ProjectId};
use mlkc_vfs::VfsPath;

/// The directory of the snapshots, relative to the file that asserts them.
///
/// A snapshot is written next to the file that asserts it, and the path handed to insta is
/// relative to `tests/project_specs.rs`.
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

        let resolution = driver
            .resolution(module)
            .expect("a module of a fixture to resolve");

        snapshot.push_str("### Resolution\n\n```\n");
        snapshot.push_str(&mlkc_hir_def::dump::resolution(resolution.scope()));
        snapshot.push_str("```\n\n");
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

/// The projects the corpus holds, by the names the tests name them by.
///
/// A project that is a directory is named by the directory, and one that is a file by the file
/// without the extension of a module.
pub(crate) fn fixtures() -> Vec<String> {
    mlkc_fixture::projects()
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
