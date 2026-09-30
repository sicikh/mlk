//! The standard library of the language: the sources of it, and the values that describe it.
//!
//! The library is written in MLK like any other code and lives in the repository under
//! `library`, one directory per project of it: a person reads and reviews it there, and this
//! crate is the compiler's copy of it, taken when the compiler is built ([`modules`]).
//!
//! What the library *is* --- the project it forms, the imports it gives its own modules without
//! them writing them, and the path the compiler keeps a module under --- is written here once.
//!
//! Nothing here drives a compiler: the driver depends on this crate, records the library when
//! its host asks for it, and hands the files over. A host of the compiler reads none of what is
//! written here, and cannot spell the library another way.

use mlkc_hir_def::{Name, PlainPath, Prelude, ProjectData};
use mlkc_vfs::VfsPath;

/// The name of the project the standard library is.
///
/// It is the name a path of another project roots at --- the prelude of the language names
/// `std::prelude::Int` --- while the modules of the library itself name their project with the
/// keyword `project`, the one a module knows of itself without a manifest ([`PlainPath`]).
pub const PROJECT: &str = "std";

/// The name of the module that declares the names of the language.
///
/// The other module of the library re-exports what this one declares, and the prelude of the
/// language names it: `std::prelude::Int` is a name of this module, re-exported ([`prelude`]).
pub const CORE: &str = "core";

/// One module of the standard library.
#[derive(Debug, Clone, Copy)]
pub struct Module {
    /// The name of the module within its project: `core`, `prelude`.
    ///
    /// A module is the file of that name with `.mlk` after it, under the directory of the
    /// library: the name is what a path of the library writes, and the file is where a person
    /// reads it.
    pub name: &'static str,

    /// The source of the module, as it stands in the repository.
    pub source: &'static str,
}

/// The modules of the standard library, in the order they are read.
pub fn modules() -> &'static [Module] {
    &[
        Module {
            name: CORE,
            source: include_str!("../../../library/std/core.mlk"),
        },
        Module {
            name: "prelude",
            source: include_str!("../../../library/std/prelude.mlk"),
        },
    ]
}

/// The imports the library gives its own modules without them writing them.
///
/// The names of the language are declared *by* the library, so the library cannot be compiled
/// with the prelude of the language: it is read with a prelude that names its own modules
/// ([ADR-0011]). The two modules of it today refuse any prelude with `#[no-prelude]` --- one
/// declares the names of the language, the other re-exports them --- so what this prelude
/// holds is what a module of the library that uses the names will be given, and the list is
/// the prelude of the library all the same.
///
/// [ADR-0011]: ../../docs/adr/0011-module-prelude.md
pub fn prelude() -> Prelude {
    Prelude::from_paths([
        PlainPath::from_segments([Name::new("project"), Name::new(CORE), Name::new("Int")]),
        PlainPath::from_segments([Name::new("project"), Name::new(CORE), Name::new("Unit")]),
        PlainPath::from_segments([Name::new("project"), Name::new(CORE), Name::new("String")]),
        PlainPath::from_segments([Name::new("project"), Name::new(CORE), Name::new("Bool")]),
    ])
}

/// What the library is as a project.
pub fn project() -> ProjectData {
    ProjectData {
        prelude: prelude(),
        ..ProjectData::default()
    }
}

/// The path a module of the library is kept under in a driver.
///
/// The library is not a directory of a repository that a compiler reads: it is the compiler's
/// own, so the path is a name the compiler gives --- one directory for the library, and one file
/// per module of it --- and every host sees the library under the same one.
pub fn path(module: &Module) -> VfsPath {
    VfsPath::new_virtual_path(format!("/{PROJECT}/{}.mlk", module.name))
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use super::*;

    /// The modules of the library are the files of it: a file a person adds to `library/std` is
    /// a module of the library, and a module that names a file nothing writes is not one.
    ///
    /// The list of modules is written by hand --- it is where a module is named --- and this is
    /// what keeps it and the directory of the library from drifting apart.
    #[test]
    fn the_library_is_the_files_of_the_library() {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../library")
            .join(PROJECT);

        let mut files: Vec<String> = fs::read_dir(&directory)
            .expect("the directory of the library to be readable")
            .map(|entry| {
                entry
                    .expect("an entry of the library to be readable")
                    .file_name()
                    .into_string()
                    .expect("a module to be named by text")
            })
            .collect();
        files.sort();

        let mut named: Vec<String> = modules()
            .iter()
            .map(|module| format!("{}.mlk", module.name))
            .collect();
        named.sort();

        assert_eq!(files, named, "the files of {}", directory.display());
    }

    #[test]
    fn a_module_of_the_library_is_a_file_of_a_directory_of_its_own() {
        let named: Vec<_> = modules()
            .iter()
            .map(|module| path(module).to_string())
            .collect();

        assert_eq!(named, ["/std/core.mlk", "/std/prelude.mlk"]);
    }

    #[test]
    fn the_prelude_of_the_library_names_the_modules_of_the_library() {
        let prelude = prelude();
        let imports = prelude.imports();
        let named: Vec<_> = imports
            .iter()
            .map(|import| import.name().as_str().to_owned())
            .collect();

        assert_eq!(named, ["Int", "Unit", "String", "Bool"]);
        assert_eq!(imports[0].path().to_string(), "project::core::Int");
        assert_eq!(imports[1].path().to_string(), "project::core::Unit");
        assert_eq!(imports[2].path().to_string(), "project::core::String");
        assert_eq!(imports[3].path().to_string(), "project::core::Bool");
    }
}
