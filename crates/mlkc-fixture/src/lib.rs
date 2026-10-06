//! A project written in one file: the fixture format of the tests.
//!
//! A fixture holds the modules of a project, one after another, each of them headed by the
//! place it stands at:
//!
//! ```text
//! //- /main.mlk
//! use project::data::Point
//!
//! //- /data.mlk
//! pub type Point
//! ```
//!
//! The mark is the one the tests of rust-analyzer are written with --- `//-` and the path of
//! the file that follows --- so that a project of a handful of modules is a value a test holds
//! rather than a directory of files it reads. It is used where the modules are what a test is
//! about; a fixture that outgrows it is a directory of modules under `tests/specs`, which the
//! same tests read over the same code.
//!
//! The place of a module is written the way a host pushes a file: `/main.mlk` is the module a
//! project calls `main`, and `/data/utils.mlk` is the module `data::utils`.
//!
//! The projects the pipeline suites compile are the corpus under [`projects_dir`]: every suite
//! asserts the stage it owns over each of them, so a project added there is read by every stage
//! at once.

use std::{
    fs,
    path::{Path, PathBuf},
};

/// One module of a fixture: the place it stands at, and its source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Module {
    /// The place of the module, as the fixture wrote it: the path of its file in a project.
    pub place: String,
    /// The source of the module: the lines after the mark that names it.
    pub source: String,
}

/// The modules a fixture holds, in the order it writes them.
///
/// # Panics
///
/// Panics when the fixture holds no module, when a line before the first mark is not empty, or
/// when a mark names no place: a fixture is what a test wrote, and a mistake in it is the
/// test's rather than a value to hand on.
pub fn modules(fixture: &str) -> Vec<Module> {
    let mut modules: Vec<Module> = Vec::new();

    for line in fixture.lines() {
        if let Some(place) = line.strip_prefix(MARK) {
            let place = place.trim();

            assert!(
                !place.is_empty(),
                "a mark to name the place of the module it heads",
            );

            modules.push(Module {
                place: place.to_owned(),
                source: String::new(),
            });

            continue;
        }

        let Some(module) = modules.last_mut() else {
            // What may stand before the first mark is what a fixture says of itself: a comment,
            // and the blank lines around it. It belongs to no module, and to no source.
            assert!(
                line.trim().is_empty() || line.trim_start().starts_with(COMMENT),
                "a fixture to open with the mark of a module, found `{line}`",
            );

            continue;
        };

        // The source of a module is its lines, each of them ending in a break: a fixture is
        // written one line at a time, and a module of one is a module of the language.
        module.source.push_str(line);
        module.source.push('\n');
    }

    // A blank line between two modules is what separates them rather than a part of either:
    // what a module holds is its own lines, and a module of the language ends in one break.
    for module in &mut modules {
        if module.source.trim_end().is_empty() {
            module.source.clear();
            continue;
        }

        module.source = format!("{}\n", module.source.trim_end());
    }

    assert!(!modules.is_empty(), "a fixture to hold at least one module");

    modules
}

/// The mark a module of a fixture is headed by: a comment, so that a fixture reads as the code
/// it holds.
const MARK: &str = "//-";

/// What a comment starts with, which is what a fixture may open with.
const COMMENT: &str = "//";

/// The extension of a module of a project.
const EXTENSION: &str = "mlk";

/// The directory the projects the pipeline suites compile live in.
///
/// A project is a file in the format above, or a directory that holds one file per module.
/// The corpus is read by every spec suite of the pipeline; see [`projects`] and [`read`].
pub fn projects_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("projects")
}

/// The projects the corpus holds, in name order: the name of the file without the extension of
/// a module, or the name of the directory that holds one file per module.
///
/// # Panics
///
/// Panics when the directory of the corpus is not readable: the projects ship with the crate,
/// and a build that lost them is broken rather than one that has nothing to say.
pub fn projects() -> Vec<String> {
    let directory = projects_dir();
    let mut projects = Vec::new();

    let entries = fs::read_dir(&directory)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", directory.display()));

    for entry in entries {
        let path = entry
            .expect("a project of the corpus to be readable")
            .path();

        if path.is_dir() {
            let name = path
                .file_name()
                .and_then(|it| it.to_str())
                .expect("a project to be named");

            projects.push(name.to_owned());
        } else if path.extension() == Some(EXTENSION.as_ref()) {
            let name = path
                .file_stem()
                .and_then(|it| it.to_str())
                .expect("a project to be named");

            projects.push(name.to_owned());
        }
    }

    projects.sort();
    projects
}

/// The modules of the project `name`: what its file writes, or the files under its directory,
/// each named by the path of the file under the directory.
///
/// # Panics
///
/// Panics when the project cannot be read, or when a directory of one holds no module: a
/// project of the corpus is what a test compiles, and a missing one fails the test that read it.
pub fn read(name: &str) -> Vec<Module> {
    let path = projects_dir().join(name);

    if path.is_dir() {
        let mut modules = Vec::new();

        collect(&path, &path, &mut modules);
        modules.sort_by(|left, right| left.place.cmp(&right.place));

        assert!(!modules.is_empty(), "the project `{name}` to hold a module",);

        return modules;
    }

    let path = path.with_extension(EXTENSION);
    let source = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));

    modules(&source)
}

/// The modules of a project that is a directory: the place of each file under it, and its source.
///
/// The place of a file is the path of it under the directory of the project, which is what the
/// module is called by: the file `data/utils.mlk` is the module `project::data::utils`.
fn collect(root: &Path, directory: &Path, modules: &mut Vec<Module>) {
    let entries = fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", directory.display()));

    for entry in entries {
        let path = entry.expect("the entry to be readable").path();

        if path.is_dir() {
            collect(root, &path, modules);
        } else if path.extension() == Some(EXTENSION.as_ref()) {
            let place = path
                .strip_prefix(root)
                .expect("a file of a project to be under the project")
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

#[cfg(test)]
mod tests {
    use super::{Module, modules};

    #[test]
    fn a_fixture_is_the_modules_it_writes() {
        let modules = modules(
            "\
//- /main.mlk
use project::data::Point

//- /data/utils.mlk
pub type Point
",
        );

        assert_eq!(modules, [
            Module {
                place: "/main.mlk".to_owned(),
                source: "use project::data::Point\n".to_owned(),
            },
            Module {
                place: "/data/utils.mlk".to_owned(),
                source: "pub type Point\n".to_owned(),
            },
        ],);
    }

    #[test]
    fn a_module_of_a_fixture_may_hold_nothing() {
        let modules = modules("//- /empty.mlk\n//- /main.mlk\nfun main() = 1\n");

        assert_eq!(modules[0].source, "");
        assert_eq!(modules[1].source, "fun main() = 1\n");
    }

    #[test]
    fn a_comment_of_a_module_is_not_a_mark() {
        let modules = modules("//- /main.mlk\n// what a module says of itself\nfun main() = 1\n");

        assert_eq!(
            modules[0].source,
            "// what a module says of itself\nfun main() = 1\n"
        );
    }

    #[test]
    fn a_fixture_may_say_what_it_is_before_the_first_module() {
        let modules = modules("// What this fixture is.\n\n//- /main.mlk\nfun main() = 1\n");

        assert_eq!(modules[0].source, "fun main() = 1\n");
    }

    #[test]
    #[should_panic(expected = "a fixture to hold at least one module")]
    fn a_fixture_that_holds_no_module_is_a_mistake() {
        modules("");
    }

    #[test]
    #[should_panic(expected = "a mark to name the place")]
    fn a_mark_that_names_no_place_is_a_mistake() {
        modules("//-  \n");
    }

    #[test]
    #[should_panic(expected = "a fixture to open with the mark")]
    fn a_line_before_the_first_mark_is_a_mistake() {
        modules("fun main() = 1\n//- /main.mlk\n");
    }
}
