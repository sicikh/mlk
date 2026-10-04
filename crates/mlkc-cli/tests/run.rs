//! Running a build: a project compiled into modules and a manifest runs, whether the build is
//! read from a directory of files or from the archive the editor hands over, and what it
//! printed is what it computed.

use std::{
    fs,
    path::{Path, PathBuf},
};

use mlkc_driver::{Driver, Manifest};
use mlkc_hir_def::{ModuleId, Name, ProjectData, ProjectId};
use mlkc_vfs::VfsPath;

/// What the project of the test is: one module, with an entry point that prints what it
/// computes.
const SOURCE: &str = "#[entry]\npub fun main(): Unit =\n    print-int(6 * 7)\n";

/// The project the test builds.
const PROJECT: &str = "app";

#[test]
fn a_build_written_as_files_runs() {
    let directory = write("files");
    let printed = mlkc_cli::run::run(&directory).expect("the build to run");

    assert_eq!(printed, ["42"]);
}

#[test]
fn a_build_read_from_an_archive_runs() {
    let directory = write("archive");
    let mut archive = Zip::default();

    for (name, bytes) in files_of(&directory) {
        archive.add(&name, &bytes);
    }

    let path = std::env::temp_dir().join(format!("mlkc-run-{}.zip", std::process::id()));

    fs::write(&path, archive.finish()).expect("the archive to be written");

    let printed = mlkc_cli::run::run(&path).expect("the build to run");

    assert_eq!(printed, ["42"]);
}

/// Builds the project, writes the modules and the manifest into a directory of their own, and
/// hands the directory back. Each test writes under a name of its own: tests run at once.
fn write(tag: &str) -> PathBuf {
    let mut driver = Driver::new();

    driver.use_std();

    let mut data = ProjectData::default();
    data.dependencies.insert(
        Name::new(mlkc_stdlib::PROJECT),
        ProjectId::new(mlkc_stdlib::PROJECT),
    );
    driver.set_project(ProjectId::new(PROJECT), data);

    let path = VfsPath::new_virtual_path("/main.mlk".to_string());

    driver.set_file_text(path.clone(), Some(SOURCE.to_string()));

    let file = driver.file_id(&path).expect("the file to have an id");

    driver.set_module_project(ModuleId(file), ProjectId::new(PROJECT));

    let plan = driver
        .link(&ProjectId::new(PROJECT))
        .expect("the project to link");
    let manifest = Manifest::of(PROJECT, &plan, Some("host.wasm"));
    let directory = std::env::temp_dir().join(format!("mlkc-run-{}-{tag}", std::process::id()));

    if directory.exists() {
        fs::remove_dir_all(&directory).expect("the directory of the last run to be dropped");
    }

    fs::create_dir_all(&directory).expect("the directory of a build to be made");

    for id in &plan.order {
        let path = directory.join(Manifest::file_of(&plan.names[id]));

        fs::create_dir_all(path.parent().expect("a file to stand in a directory"))
            .expect("the directory of a module to be made");
        fs::write(&path, &plan.modules[id].bytes).expect("the module to be written");
    }

    // The shim the manifest names is not written: a host that makes GC values itself does not
    // need it, and a build without it is one this host still runs.
    fs::write(
        directory.join(Manifest::FILE),
        serde_json::to_string_pretty(&manifest).expect("the manifest to serialize"),
    )
    .expect("the manifest to be written");

    directory
}

/// The files under `directory`, by the path each stands under inside it.
fn files_of(directory: &Path) -> Vec<(String, Vec<u8>)> {
    let mut files = Vec::new();

    collect(directory, directory, &mut files);
    files.sort_by(|left, right| left.0.cmp(&right.0));

    files
}

/// Collects every file under `at`, named by its path under `root`.
fn collect(root: &Path, at: &Path, into: &mut Vec<(String, Vec<u8>)>) {
    for entry in fs::read_dir(at).expect("the directory of a build to be read") {
        let path = entry.expect("an entry to be read").path();

        if path.is_dir() {
            collect(root, &path, into);
            continue;
        }

        let name = path
            .strip_prefix(root)
            .expect("a path under the root")
            .to_string_lossy()
            .replace(std::path::MAIN_SEPARATOR, "/");

        into.push((
            name,
            fs::read(&path).expect("a file of the build to be read"),
        ));
    }
}

/// A writer of the archive the editor writes: a build handed over as one file.
///
/// Every file is stored as it is, and the directory at the end names where each of them
/// stands. The checksum of a file is zero: a reader of a build does not check it, and the test
/// is of what such a reader reads.
#[derive(Default)]
struct Zip {
    bytes: Vec<u8>,
    files: Vec<Held>,
}

/// One file of that archive: its path, where its header stands, and how long it is.
struct Held {
    name: String,
    at: u32,
    size: u32,
}

impl Zip {
    /// Adds a file to the archive, stored as it is.
    fn add(&mut self, name: &str, bytes: &[u8]) {
        let at = self.bytes.len() as u32;
        let size = bytes.len() as u32;

        self.u32(0x0403_4B50);
        self.u16(20);
        self.u16(0x0800);
        self.u16(0);
        self.u16(0);
        self.u16(0);
        self.u32(0);
        self.u32(size);
        self.u32(size);
        self.u16(name.len() as u16);
        self.u16(0);
        self.bytes.extend_from_slice(name.as_bytes());
        self.bytes.extend_from_slice(bytes);

        self.files.push(Held {
            name: name.to_owned(),
            at,
            size,
        });
    }

    /// Closes the archive with the directory of the files, and hands the bytes back.
    fn finish(mut self) -> Vec<u8> {
        let directory = self.bytes.len() as u32;
        let files = std::mem::take(&mut self.files);

        for held in &files {
            self.u32(0x0201_4B50);
            self.u16(20);
            self.u16(20);
            self.u16(0x0800);
            self.u16(0);
            self.u16(0);
            self.u16(0);
            self.u32(0);
            self.u32(held.size);
            self.u32(held.size);
            self.u16(held.name.len() as u16);
            self.u16(0);
            self.u16(0);
            self.u16(0);
            self.u16(0);
            self.u32(0);
            self.u32(held.at);
            self.bytes.extend_from_slice(held.name.as_bytes());
        }

        let took = self.bytes.len() as u32 - directory;

        self.u32(0x0605_4B50);
        self.u16(0);
        self.u16(0);
        self.u16(files.len() as u16);
        self.u16(files.len() as u16);
        self.u32(took);
        self.u32(directory);
        self.u16(0);

        self.bytes
    }

    fn u16(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }
}
