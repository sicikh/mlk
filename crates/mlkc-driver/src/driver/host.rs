//! What a host pushes, reads, and configures: the files, the projects, and the line index.

use std::sync::Arc;

use mlkc_hir_def::{ModuleId, ProjectData, ProjectGraph, ProjectId};
use mlkc_line_index::LineIndex;
use mlkc_vfs::{ChangedFile, FileId, FileState, FileVersion, RelPathBuf, VfsPath};

use super::{Driver, Pass, options::Options};

/// One file of the standard library: the path a driver keeps a module under, and its source.
///
/// A host that records the library is handed these ([`Driver::use_std`]), and a host that shows
/// it to a person --- the editor --- shows the same files and writes in none of them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StdFile {
    /// The path the file is known by in a driver.
    pub path: VfsPath,

    /// The source of the file, as the compiler was built with it.
    pub text: &'static str,
}

/// The root of the file system: what the place of a file is read against
/// ([`Driver::module_path`]).
fn root() -> VfsPath {
    VfsPath::new_virtual_path("/".to_owned())
}

/// The path of a file under the directory it is written in: the name of the file itself.
///
/// This is what a module is called by when the place of its file says nothing of it: a file of
/// the host's own file system, and a file written at the root of the file system. The extension
/// is kept --- it is what says that the file is a file of the language, and it is lowering that
/// leaves it out of the name of the module --- and a path that names no file names no module.
fn file_name(path: &VfsPath) -> Option<RelPathBuf> {
    let name = match path.name_and_extension() {
        Some((stem, Some(extension))) => format!("{stem}.{extension}"),
        Some((stem, None)) => stem.to_owned(),
        None => return None,
    };

    Some(RelPathBuf::try_from(name.as_str()).expect("the name of a file to be a relative path"))
}

impl Driver {
    /// A driver that knows nothing: the host pushes what it wants compiled.
    ///
    /// The standard library of the language is the one thing a host does not have to know:
    /// a host asks for it ([`Driver::use_std`]) and is handed the files of the library the
    /// compiler was built with.
    pub fn new() -> Self {
        Self::default()
    }

    // Inputs: the driver never reaches for any of this.

    /// Feeds the contents of a file into the driver; `None` means the file is gone.
    ///
    /// Returns whether the contents changed,
    /// and pushing the same contents again changes nothing.
    pub fn set_file_contents(&mut self, path: VfsPath, contents: Option<Vec<u8>>) -> bool {
        self.vfs.set_file_contents(path, contents)
    }

    /// A convenience for a host that already holds text:
    /// an editor, a WASM shim, a test.
    pub fn set_file_text(&mut self, path: VfsPath, text: Option<String>) -> bool {
        self.vfs.set_file_text(path, text)
    }

    /// Configures the pipeline: how much debug information a module carries, and how hard the
    /// passes optimize ([ADR-0023][adr-0023]).
    ///
    /// Returns whether the options changed; pushing them again changes nothing, and a value
    /// built under them stays where it was
    /// ([ADR-0008][adr-0008]).
    ///
    /// [adr-0008]: ../../docs/adr/0008-compiler-driver.md
    /// [adr-0023]: ../../docs/adr/0023-debug-information.md
    pub fn set_options(&mut self, options: Options) -> bool {
        if options == self.options {
            return false;
        }

        self.options = options;
        self.options_version += 1;

        true
    }

    /// What the host last configured.
    pub fn options(&self) -> Options {
        self.options
    }

    /// Records a project: what it depends on, and the prelude its modules are given without
    /// writing them.
    ///
    /// Returns whether the graph changed. A project says what its modules are read under, so
    /// a change to one drops the HIR of the modules that belong to it ([ADR-0008]) --- today
    /// the prelude is the part of a project that lowering reads; the parses are kept, since a
    /// parse is a function of the text and of nothing else.
    ///
    /// A project holds no module it starts from: which modules are its own is recorded one by
    /// one with [`Driver::set_module_project`].
    ///
    /// [ADR-0008]: ../../docs/adr/0008-compiler-driver.md
    pub fn set_project(&mut self, project: ProjectId, data: ProjectData) -> bool {
        if self.projects.project(&project) == Some(&data) {
            return false;
        }

        Arc::make_mut(&mut self.projects).insert(project.clone(), data);
        self.invalidate_project(&project);

        true
    }

    /// Records the standard library of the language: pushes its sources, and records the
    /// project they are.
    ///
    /// The library is part of the compiler ([`mlkc_stdlib`]) and not of a host, so nothing here
    /// is a host's to say: where the modules land, which project they are, and the imports the
    /// library gives its own modules are all decided by the compiler.
    ///
    /// A host says *when* the library goes in, and a driver whose host never calls this holds no
    /// library: the tests of this crate want that, and so does a host that compiles against
    /// another library, which records its own ([`Driver::set_project`]).
    ///
    /// Returns the files the library is made of, in the order of [`mlkc_stdlib::modules`]:
    /// the path each was recorded at, and its source, which is what a host shows. The library is
    /// the compiler's rather than a person's, and nothing in it is a person's to write.
    pub fn use_std(&mut self) -> Vec<StdFile> {
        let files: Vec<StdFile> = mlkc_stdlib::modules()
            .iter()
            .map(|module| {
                StdFile {
                    path: mlkc_stdlib::path(module),
                    text: module.source,
                }
            })
            .collect();

        for file in &files {
            self.set_file_text(file.path.clone(), Some(file.text.to_owned()));
        }

        let id = ProjectId::new(mlkc_stdlib::PROJECT);

        self.set_project(id.clone(), mlkc_stdlib::project());

        // Every module of the library is one of the project: a project holds no module it
        // starts from, so each of them is recorded.
        for file in &files {
            let module = ModuleId(
                self.file_id(&file.path)
                    .expect("a file of the library to have an id once it is pushed"),
            );

            self.set_module_project(module, id.clone());
        }

        files
    }

    /// Drops a project, returning what it was.
    ///
    /// The modules of the project belong to no project afterwards, so they are read with the
    /// prelude of the language again, and the HIR of them goes.
    pub fn remove_project(&mut self, project: &ProjectId) -> Option<ProjectData> {
        self.projects.project(project)?;
        self.invalidate_project(project);

        Arc::make_mut(&mut self.projects).remove(project)
    }

    /// Records which project a module belongs to, and returns whether the graph changed.
    ///
    /// A module is lowered with the prelude of its project, so a module that changes project
    /// drops the HIR that was read under the other one. A module may be recorded before the
    /// graph holds the project: it is read with the prelude of the language until it does.
    pub fn set_module_project(&mut self, module: ModuleId, project: ProjectId) -> bool {
        if self.projects.project_of(module) == Some(&project) {
            return false;
        }

        Arc::make_mut(&mut self.projects).set_module_project(module, project);
        self.invalidate_module(module);

        true
    }

    /// Forgets which project a module belongs to, and returns whether the graph changed.
    ///
    /// A module that leaves a project is read with the prelude of the language again, so what
    /// was read under the project goes, and so does the plan of the project: the plan is a value
    /// of the modules it is built from ([ADR-0021]).
    ///
    /// A host that drops a file says so here. The file goes in with
    /// [`Driver::set_file_text`], and the graph is told which project it is of by
    /// [`Driver::set_module_project`]: nothing here is inferred, because whether a file that is
    /// not there is a module that left the project or one that is missing is a host's to say.
    ///
    /// [ADR-0021]: ../../docs/adr/0021-translation-units.md
    pub fn remove_module_project(&mut self, module: ModuleId) -> bool {
        if self.projects.project_of(module).is_none() {
            return false;
        }

        // What was read under the project goes before the module does: a value of the project
        // names the modules it was built from.
        self.invalidate_module(module);

        Arc::make_mut(&mut self.projects)
            .remove_module(module)
            .is_some()
    }

    // Reads: no computation, and none of them takes `&mut self`.

    /// The id of a path the driver knows, if the file is there.
    pub fn file_id(&self, path: &VfsPath) -> Option<FileId> {
        self.vfs.file_id(path).map(|(file, _)| file)
    }

    /// The id of a path the driver has seen, whether or not the file is still there.
    ///
    /// [`Driver::file_id`] answers for a file that is there; a file that is gone keeps its id,
    /// and what a host reads of it afterwards is the state it went away with
    /// ([`Driver::file_state`]). A host that drops a file says so by this id: the module it was
    /// is a value of the driver, and the path is the only name it has left for it.
    pub fn path_id(&self, path: &VfsPath) -> Option<FileId> {
        self.vfs.path_id(path)
    }

    /// The contents of a file, or `None` if it is missing, unreadable, or excluded.
    pub fn file_text(&self, file: FileId) -> Option<Arc<str>> {
        self.vfs.file_text(file)
    }

    /// The version of the contents of a file, which is the identity of its state.
    pub fn file_version(&self, file: FileId) -> FileVersion {
        self.vfs.file_version(file)
    }

    /// What the driver knows about a file: read or not, text or not, there or not.
    pub fn file_state(&self, file: FileId) -> FileState {
        self.vfs.file_state(file)
    }

    /// The path a file was interned under.
    pub fn file_path(&self, file: FileId) -> &VfsPath {
        self.vfs.file_path(file)
    }

    /// The projects the compiler knows, and the project each module belongs to.
    pub fn project_graph(&self) -> &ProjectGraph {
        &self.projects
    }

    /// The projects a path of a module is read in: the project the module belongs to, and the
    /// projects that project depends on.
    ///
    /// A module names the project it belongs to by the keyword `project`, so what the names
    /// after the keyword are read against is the index of that project, and the names of the
    /// projects it depends on are read against their indexes. A module that belongs to no
    /// project is read in every project of the graph, which is all there is to declare a
    /// dependency on ([ADR-0016]).
    ///
    /// [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md
    pub(super) fn read_projects(&self, module: ModuleId) -> Vec<ProjectId> {
        match self.projects.project_of(module) {
            Some(project) => {
                match self.projects.project(project) {
                    Some(data) => {
                        std::iter::once(project.clone())
                            .chain(data.dependencies.values().cloned())
                            .collect()
                    },
                    None => Vec::new(),
                }
            },
            None => {
                self.projects
                    .projects()
                    .map(|(project, _)| project.clone())
                    .collect()
            },
        }
    }

    /// The projects a module may name.
    ///
    /// A module names the projects that the project it belongs to depends on; what it calls its
    /// own project by is the keyword `project`, and not a name ([ADR-0016]). A module that
    /// belongs to no project names the projects of the graph, each of them by its own name,
    /// which is all there is to declare a dependency on.
    ///
    /// The lowering is keyed by the list, so it is sorted: the same projects are the same input
    /// however a host declared them.
    ///
    /// [ADR-0016]: ../../docs/adr/0016-inter-module-resolution.md
    pub(super) fn named_projects(&self, module: ModuleId) -> Vec<ProjectId> {
        let mut projects: Vec<ProjectId> = match self.projects.project_of(module) {
            Some(project) => {
                match self.projects.project(project) {
                    Some(data) => data.dependencies.values().cloned().collect(),
                    None => Vec::new(),
                }
            },
            None => {
                self.projects
                    .projects()
                    .map(|(project, _)| project.clone())
                    .collect()
            },
        };

        projects.sort();
        projects
    }

    /// Where a module stands: the place of its file, which is what the module is called when it
    /// declares no path of its own.
    ///
    /// The place of a file is the path of it under the root of the file system it was pushed
    /// into: the module `lib/arith.mlk` is the module `project::lib::arith`, and `main.mlk`
    /// itself is `project::main`. A host lays a project out in that file system, so the paths
    /// it pushes are what says where the modules of the project stand.
    ///
    /// A file of the host's own file system is a path of the machine rather than a place in the
    /// tree a host laid out, and a module of either is called by the name of its file, the name
    /// being all there is to say which module it is.
    ///
    /// `None` for a file whose place names no file --- the root of the file system, pushed as
    /// if it were a file --- since a module is a file, and a place that names no file names no
    /// module: there is nothing to lower such a file to.
    pub(super) fn module_path(&self, module: ModuleId) -> Option<RelPathBuf> {
        let file = self.vfs.file_path(module.0);

        match file.strip_prefix(&root()) {
            // What stands under the root is the place of the module; the root itself is a place
            // that names no file.
            Some(relative) if relative.as_utf8_path().file_name().is_some() => {
                Some(relative.to_path_buf())
            },
            Some(_) => None,
            None => file_name(file),
        }
    }

    /// The line index of `file`: where its lines start and end.
    ///
    /// A person and a protocol read a position as a line and a column, a diagnostic points
    /// at a byte range, and the index is the mapping between the two.
    /// It is derived from the text like everything else here,
    /// so nothing computes it before somebody asks.
    pub fn line_index(&mut self, file: FileId) -> Option<Arc<LineIndex>> {
        let version = self.file_version(file);
        let text = self.file_text(file);

        Self::text_derived(
            &mut self.stats,
            self.clock,
            Pass::LineIndex,
            &mut self.line_indices,
            file,
            version,
            || text.map(|text| Arc::new(LineIndex::new(&text))),
        )
    }

    /// The net effect of the pushes a host made since its last call.
    ///
    /// The driver does not need this: a slot decides its own validity by comparing versions,
    /// and the driver knows which files were pushed.
    /// A host does, to know which documents to publish again —
    /// a file that appeared and disappeared between two calls is not one of them.
    pub fn take_changes(&mut self) -> Vec<ChangedFile> {
        self.vfs.take_changes().into_values().collect()
    }
}
