//! The manifest of a build: what a host reads to run a program whose modules are files
//! ([ADR-0021]).
//!
//! A plan of the link stage is a value a host either instantiates as it stands --- a browser,
//! which holds the bytes in memory --- or writes out as files: the editor's archive, and the
//! CLI. A host that writes files cannot ask the plan what a module is, because the plan is on
//! the other side of the disk; what it can read is this manifest, which is written beside the
//! modules ([`Manifest::FILE`]) and says what a host needs:
//! which file every module is, what each imports and exports, and where the program begins.
//!
//! [adr-0021]: ../../docs/adr/0021-translation-units.md

use serde::{Deserialize, Serialize};

use crate::LinkPlan;

/// The manifest of a build ([ADR-0021]).
///
/// The shape is the one a person reads and a host parses: a module is a canonical name and the
/// file it is written as, an import says whether a module or the host provides it, and the
/// entry says what a host calls to run the program.
///
/// [adr-0021]: ../../docs/adr/0021-translation-units.md
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    /// The canonical name of the project the program is of: the project the plan was linked
    /// for, whose modules are the program and whose dependencies stand beside them.
    pub project: String,

    /// The modules of the program, providers before the modules that import them: the order a
    /// host instantiates them in.
    pub modules: Vec<ManifestModule>,

    /// The file the shim for a host that cannot make GC values is written as, when the build
    /// writes one: a host that makes them itself does not need it.
    pub host: Option<String>,

    /// Where the program begins: the module, and the name the entry is exported by. `None` when
    /// the project declares no entry point.
    pub entry: Option<ManifestEntry>,
}

/// One module of a build: where it is written, and what it needs and offers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestModule {
    /// The canonical name of the module: the name the modules after it import it by, and the
    /// name a stack trace shows.
    pub name: String,

    /// The path of the file the module is written as, relative to the build.
    pub file: String,

    /// The functions the module imports, in the order of their indices. An external one is
    /// what a host implements, and the others are what the modules before it export.
    pub imports: Vec<ManifestImport>,

    /// The functions the module exports, in the order it declares them.
    pub exports: Vec<ManifestExport>,
}

/// One function a module of a build imports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestImport {
    /// The canonical name of the module the function belongs to.
    pub module: String,

    /// The name of the function inside its module.
    pub name: String,

    /// Whether the function is declared `#[extern]`: a host implements it, not a module.
    pub external: bool,

    /// How many words the function takes; it gives back one.
    pub arity: u32,
}

/// One function a module of a build exports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestExport {
    /// The name of the function inside its module.
    pub name: String,

    /// How many words the function takes; it gives back one.
    pub arity: u32,
}

/// Where a build begins: the module the entry is in, and the name it is exported by.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestEntry {
    /// The canonical name of the module the entry is in.
    pub module: String,

    /// The name the entry is exported by.
    pub name: String,
}

impl Manifest {
    /// The name a build writes the manifest under, and the name a host reads it by.
    pub const FILE: &str = "manifest.json";

    /// The manifest of `plan`: every module of it in the order it is instantiated in, the file
    /// each is written as, and the entry the project declares ([ADR-0021]).
    ///
    /// `host` is the file the shim for a host that cannot make GC values is written as, when
    /// the build writes one ([`Manifest::host`]).
    ///
    /// [adr-0021]: ../../docs/adr/0021-translation-units.md
    pub fn of(project: &str, plan: &LinkPlan, host: Option<&str>) -> Self {
        let modules = plan
            .order
            .iter()
            .map(|id| {
                let module = &plan.modules[id];

                ManifestModule {
                    name: plan.names[id].clone(),
                    file: Self::file_of(&plan.names[id]),
                    imports: module
                        .imports
                        .iter()
                        .map(|import| {
                            ManifestImport {
                                module: import.module.clone(),
                                name: import.name.clone(),
                                external: import.external,
                                arity: import.arity,
                            }
                        })
                        .collect(),
                    exports: module
                        .exports
                        .iter()
                        .map(|export| {
                            ManifestExport {
                                name: export.name.clone(),
                                arity: export.arity,
                            }
                        })
                        .collect(),
                }
            })
            .collect();
        let entry = plan.entry.as_ref().map(|(module, name)| {
            ManifestEntry {
                module: plan.names[module].clone(),
                name: name.clone(),
            }
        });

        Manifest {
            project: project.to_owned(),
            modules,
            host: host.map(str::to_owned),
            entry,
        }
    }

    /// The file a module of `name` is written as: `app::main` is `app/main.wasm`.
    ///
    /// A canonical name is the project and the path of the module inside it, and a file a host
    /// writes is a path under a directory. Anything a path may not hold becomes a dash, and so
    /// does a part of dots only, which would name the directory itself or the one above it:
    /// what a name names is a file of the build, and nothing else.
    pub fn file_of(name: &str) -> String {
        let mut file = String::with_capacity(name.len() + 5);

        for (at, part) in name.split("::").enumerate() {
            if at > 0 {
                file.push('/');
            }

            if part.is_empty() || part.chars().all(|it| it == '.') {
                file.push_str(&"-".repeat(part.len().max(1)));
                continue;
            }

            for it in part.chars() {
                file.push(
                    if it.is_ascii_alphanumeric() || matches!(it, '.' | '_' | '-') {
                        it
                    } else {
                        '-'
                    },
                );
            }
        }

        file.push_str(".wasm");

        file
    }
}

#[cfg(test)]
mod tests {
    use super::Manifest;

    #[test]
    fn a_canonical_name_becomes_a_path_under_a_directory() {
        assert_eq!(Manifest::file_of("app::main"), "app/main.wasm");
        assert_eq!(Manifest::file_of("std::core"), "std/core.wasm");
        assert_eq!(Manifest::file_of("main"), "main.wasm");
        assert_eq!(Manifest::file_of("app::lib::a.b"), "app/lib/a.b.wasm");
    }

    #[test]
    fn a_name_a_path_may_not_hold_becomes_a_file_of_the_build() {
        // A part of dots only would name the directory itself or the one above it.
        assert_eq!(Manifest::file_of("app::.."), "app/--.wasm");
        assert_eq!(Manifest::file_of("app::."), "app/-.wasm");
        assert_eq!(Manifest::file_of("::"), "-/-.wasm");
        assert_eq!(Manifest::file_of("app::a b"), "app/a-b.wasm");
        assert_eq!(Manifest::file_of("app::a/b"), "app/a-b.wasm");
    }
}
