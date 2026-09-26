//! The imports a project gives every module without the module writing them.
//!
//! A prelude is what lets a module use `Int` without writing `use std.prelude.Int`: lowering
//! declares the imports of the prelude into the item tree of a module that does not refuse
//! them, so that everything after lowering reads a prelude import as it reads any other import
//! ([ADR-0011][adr-0011]).
//!
//! The language has a standard prelude ([`Prelude::standard`]), and a project may replace it
//! with one of its own: `std` itself is compiled with a prelude that names its own modules,
//! and a module that declares the prelude's names refuses the prelude as a whole ([`@no-prelude`]).
//!
//! [adr-0011]: ../../docs/adr/0011-module-prelude.md
//! [`@no-prelude`]: crate::item_data::ModuleAttributes::no_prelude

use std::sync::OnceLock;

use crate::{
    name::Name,
    path::{PlainPath, PlainPathId},
};

/// One import a prelude brings into every module of a project.
///
/// The shape is the one a written import has: a path, and the name it is brought in under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreludeImport {
    /// The name the import brings in.
    name: Name,
    /// The path of the entity it names.
    path: PlainPathId,
}

impl PreludeImport {
    /// The import a path is written as: the name it brings in is the last segment of the path.
    ///
    /// A path that has no last segment --- a path of one name, which is where a path starts
    /// and nothing else --- brings in a name that is not there; a caller that declares it
    /// records nothing, since a name that is not there is not a name a module has.
    pub fn of(path: PlainPath) -> Self {
        let name = path.last().cloned().unwrap_or_else(Name::missing);

        Self {
            name,
            path: PlainPathId::new(path),
        }
    }

    /// An import of `path`, under `name`.
    pub fn new(name: Name, path: PlainPathId) -> Self {
        Self { name, path }
    }

    /// The name the import brings in.
    pub fn name(&self) -> &Name {
        &self.name
    }

    /// The path of the entity it names.
    pub fn path(&self) -> &PlainPathId {
        &self.path
    }
}

/// The imports a project gives every module without the module writing them.
///
/// The prelude is a fixed list of named imports, never a glob: a glob would make the names of
/// a module depend on the contents of another one, which the module system rejects
/// ([ADR-0004][adr-0004]).
///
/// [adr-0004]: ../../docs/adr/0004-module-system.md
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prelude {
    imports: Vec<PreludeImport>,
}

impl Prelude {
    /// The prelude of the language: what every module of every project gets.
    ///
    /// A project that declares no prelude of its own is compiled with this one, and a project
    /// that wants these names and others writes the list out: a prelude replaces the standard
    /// one rather than adding to it.
    ///
    /// The standard prelude is one value of the compiler rather than a value per caller, so a
    /// caller is handed a reference to it.
    pub fn standard() -> &'static Prelude {
        static STANDARD: OnceLock<Prelude> = OnceLock::new();

        STANDARD.get_or_init(|| {
            Self::from_paths([
                PlainPath::from_segments([
                    Name::new("std"),
                    Name::new("prelude"),
                    Name::new("Int"),
                ]),
                PlainPath::from_segments([
                    Name::new("std"),
                    Name::new("prelude"),
                    Name::new("Unit"),
                ]),
            ])
        })
    }

    /// A prelude of the paths, in the order they are given.
    pub fn from_paths(paths: impl IntoIterator<Item = PlainPath>) -> Self {
        Self {
            imports: paths.into_iter().map(PreludeImport::of).collect(),
        }
    }

    /// A prelude that brings nothing in.
    pub fn none() -> Self {
        Self {
            imports: Vec::new(),
        }
    }

    /// The imports of the prelude, in the order they are given.
    pub fn imports(&self) -> &[PreludeImport] {
        &self.imports
    }

    /// Whether the prelude brings nothing in.
    pub fn is_empty(&self) -> bool {
        self.imports.is_empty()
    }
}

impl Default for Prelude {
    /// The prelude of a project that declares none: a copy of [`Prelude::standard`].
    fn default() -> Self {
        Self::standard().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_import_brings_a_path_in_under_its_last_segment() {
        let import = PreludeImport::of(PlainPath::from_segments([
            Name::new("std"),
            Name::new("prelude"),
            Name::new("Int"),
        ]));

        assert_eq!(import.name(), &Name::new("Int"));
        assert_eq!(import.path().to_string(), "std.prelude.Int");
    }

    #[test]
    fn a_path_that_names_nothing_brings_in_a_name_that_is_not_there() {
        // A path of one name is the root of a path and nothing else: there is no name after it
        // for an import to bring in.
        let import = PreludeImport::of(PlainPath::from_segments([Name::new("Int")]));

        assert!(import.name().is_missing());
    }

    #[test]
    fn the_standard_prelude_is_the_two_names_of_the_language() {
        let prelude = Prelude::standard();
        let names: Vec<_> = prelude
            .imports()
            .iter()
            .map(|import| import.name().as_str().to_owned())
            .collect();

        assert_eq!(names, ["Int", "Unit"]);
        assert_eq!(prelude.imports()[0].path().to_string(), "std.prelude.Int");
    }

    #[test]
    fn a_project_that_says_nothing_gets_the_prelude_of_the_language() {
        assert_eq!(Prelude::default(), *Prelude::standard());
        assert!(Prelude::none().is_empty());
        assert!(!Prelude::standard().is_empty());
    }
}
