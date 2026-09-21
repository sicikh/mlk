//! # Virtual File System
//!
//! The VFS owns the current state of every file the compiler knows about:
//! the path it was interned under, its contents, and a [version](FileVersion)
//! that changes exactly when the contents do.
//!
//! Files are identified by [`FileId`]s -- interned paths.
//! The notion of the path, [`VfsPath`], is somewhat abstract:
//! at the moment, it is represented as an [`std::path::PathBuf`] internally,
//! but this is an implementation detail.
//!
//! Updates are pushed to the VFS with [`set_file_contents`],
//! and the VFS does not care where they come from:
//!
//! - natively, the [`loader`] reads the files from disk and watches them;
//! - an editor sends the text of documents it opens and edits;
//! - a WASM host feeds the sources it was given.
//!
//! Pushing the same contents twice changes nothing:
//! the version stays the same and no change is reported,
//! so a watcher can re-read a whole project without invalidating a single cache.
//!
//! The compiler observes the VFS in two ways:
//!
//! - [`take_changes`] returns the changes accumulated since the last call,
//!   which is the work list of the driver;
//! - [`file_text`] returns the contents as a shared [`Arc<str>`],
//!   and [`file_version`] returns the version they were read at.
//!
//! Because the contents of a version are immutable, a worker thread
//! can hold on to them and parse them in parallel
//! while other files are being updated.
//! Caches keyed by [`FileVersion`] are therefore self-validating:
//! a value derived from `v1` of a file stays valid
//! as long as [`file_version`] still returns `v1`.
//!
//! [`set_file_contents`]: Vfs::set_file_contents
//! [`take_changes`]: Vfs::take_changes
//! [`file_text`]: Vfs::file_text
//! [`file_version`]: Vfs::file_version

mod anchored_path;
pub mod file_set;
pub mod loader;
mod path_interner;
mod vfs_path;

use std::{fmt, hash::BuildHasherDefault, mem, sync::Arc};

use indexmap::{IndexMap, map::Entry};
pub use mlkc_paths::{AbsPath, AbsPathBuf};
use rustc_hash::FxHasher;
use tracing::{Level, span};

use crate::path_interner::PathInterner;
pub use crate::{
    anchored_path::{AnchoredPath, AnchoredPathBuf},
    vfs_path::VfsPath,
};

/// Handle to a file in [`Vfs`]
///
/// Ids are interned by path and never reused, so the same path always maps to the same id.
/// It has no meaning without the [`Vfs`] that handed it out.
#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct FileId(u32);

impl FileId {
    pub const DUMMY: FileId = FileId(Self::MAX);

    const MAX: u32 = 0x7FFF_FFFF;

    #[inline]
    pub const fn from_raw(raw: u32) -> FileId {
        assert!(raw <= Self::MAX);
        FileId(raw)
    }

    #[inline]
    pub const fn index(self) -> u32 {
        self.0
    }
}

/// safe because `FileId` is a newtype of `u32`
impl nohash_hasher::IsEnabled for FileId {}

/// The version of the contents of a file.
///
/// The version is bumped whenever the contents or the existence of a file change,
/// and only then: pushing the same text twice leaves the version alone.
/// This makes it a sound cache key:
/// a value derived from the version `v` of a file stays valid
/// as long as the file is still at version `v`.
///
/// Versions are local to a file and are not comparable across files.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FileVersion(u64);

impl FileVersion {
    /// The version of a file whose contents the VFS has never seen.
    pub const INITIAL: Self = Self(0);

    /// The version as a plain number, for logging and compact cache keys.
    pub const fn raw(self) -> u64 {
        self.0
    }

    fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

/// What the [`Vfs`] knows about a file.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum FileState {
    /// The file exists and its contents are known: see [`Vfs::file_text`].
    Exists,
    /// The file exists, but its contents are not valid UTF-8, so they cannot be compiled.
    ///
    /// The state is kept instead of the file being treated as missing,
    /// so that the driver can report the file
    /// rather than silently ignoring it.
    Unreadable,
    /// The file does not exist: it was deleted, or it was never pushed to the VFS.
    Deleted,
    /// The file was specifically excluded by the user. We still include excluded files
    /// when they're opened (without their contents).
    Excluded,
}

/// Kind of change of a file, see [`Vfs::take_changes`].
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Change {
    /// The file did not exist before, and it does now.
    Create,
    /// The file existed before with different contents.
    Modify,
    /// The file existed before, and it does not exist now.
    Delete,
}

/// A file that changed, as reported by [`Vfs::take_changes`].
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ChangedFile {
    /// Id of the changed file
    pub file_id: FileId,
    /// The [version](FileVersion) the file has after the change.
    pub version: FileVersion,
    /// Kind of change
    pub change: Change,
}

impl ChangedFile {
    /// Returns `true` if the change is not [`Delete`](ChangeKind::Delete).
    pub fn exists(&self) -> bool {
        !matches!(self.change, Change::Delete)
    }

    /// Returns `true` if the change is [`Create`](ChangeKind::Create) or
    /// [`Delete`](ChangeKind::Delete).
    pub fn is_created_or_deleted(&self) -> bool {
        matches!(self.change, Change::Create | Change::Delete)
    }

    /// Returns `true` if the change is [`Create`](ChangeKind::Create).
    pub fn is_created(&self) -> bool {
        matches!(self.change, Change::Create)
    }

    /// Returns `true` if the change is [`Modify`](ChangeKind::Modify).
    pub fn is_modified(&self) -> bool {
        matches!(self.change, Change::Modify)
    }

    pub fn kind(&self) -> ChangeKind {
        match self.change {
            Change::Create => ChangeKind::Create,
            Change::Modify => ChangeKind::Modify,
            Change::Delete => ChangeKind::Delete,
        }
    }
}

/// Kind of [file change](ChangedFile).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ChangeKind {
    /// The file was (re-)created
    Create,
    /// The file was modified
    Modify,
    /// The file was deleted
    Delete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileExcluded {
    Yes,
    No,
}

/// The contents of a file, as known to the [`Vfs`].
///
/// Whether the file exists and what it contains are one value,
/// so a file that cannot be read cannot be mistaken for a file that has text.
#[derive(Clone, Debug, PartialEq, Eq)]
enum FileContents {
    /// The file exists, and its text is known.
    Text(Arc<str>),
    /// The file exists, but its contents are not valid UTF-8.
    Unreadable,
    /// The file does not exist: it was deleted, or it was never pushed to the VFS.
    Deleted,
    /// The file was specifically excluded by the user.
    Excluded,
}

impl FileContents {
    /// How this value is reported to the outside: see [`FileState`].
    fn state(&self) -> FileState {
        match self {
            Self::Text(_) => FileState::Exists,
            Self::Unreadable => FileState::Unreadable,
            Self::Deleted => FileState::Deleted,
            Self::Excluded => FileState::Excluded,
        }
    }

    fn text(&self) -> Option<&Arc<str>> {
        match self {
            Self::Text(text) => Some(text),
            Self::Unreadable | Self::Deleted | Self::Excluded => None,
        }
    }
}

/// A file tracked by the [`Vfs`].
#[derive(Clone, Debug, PartialEq, Eq)]
struct File {
    /// What the file contains, and whether it exists at all.
    contents: FileContents,
    /// Bumped on every change of the contents.
    version: FileVersion,
}

impl File {
    fn deleted() -> Self {
        Self {
            contents: FileContents::Deleted,
            version: FileVersion::INITIAL,
        }
    }

    fn state(&self) -> FileState {
        self.contents.state()
    }

    fn text(&self) -> Option<&Arc<str>> {
        self.contents.text()
    }

    /// Replaces the contents of the file.
    ///
    /// Returns `true` if the file changed, in which case its version was bumped.
    fn update(&mut self, contents: FileContents) -> bool {
        if self.contents == contents {
            return false;
        }

        self.contents = contents;
        self.version = self.version.next();

        true
    }
}

/// Storage for the state of all files known to the compiler and the file id to path mapping.
///
/// For more information see the [crate-level](crate) documentation.
#[derive(Default)]
pub struct Vfs {
    interner: PathInterner,
    files: Vec<File>,
    changes: IndexMap<FileId, ChangedFile, BuildHasherDefault<FxHasher>>,
}

impl Vfs {
    /// Id of the given path if it exists in the `Vfs` and is not deleted.
    pub fn file_id(&self, path: &VfsPath) -> Option<(FileId, FileExcluded)> {
        let file_id = self.interner.get(path)?;
        match self.state(file_id) {
            FileState::Deleted => None,
            FileState::Excluded => Some((file_id, FileExcluded::Yes)),
            FileState::Exists | FileState::Unreadable => Some((file_id, FileExcluded::No)),
        }
    }

    /// File path corresponding to the given `file_id`.
    ///
    /// # Panics
    ///
    /// Panics if the id is not present in the `Vfs`.
    pub fn file_path(&self, file_id: FileId) -> &VfsPath {
        self.interner.lookup(file_id)
    }

    /// What the VFS knows about `file_id`.
    ///
    /// An id the VFS has never seen is reported as [`FileState::Deleted`].
    pub fn file_state(&self, file_id: FileId) -> FileState {
        self.state(file_id)
    }

    /// The contents of `file_id`, or `None` if the file does not exist,
    /// cannot be read as text, or was never pushed to the VFS.
    ///
    /// The returned handle can be cloned into a worker thread:
    /// the contents of a version never change,
    /// they are only replaced by the contents of the next version.
    pub fn file_text(&self, file_id: FileId) -> Option<Arc<str>> {
        self.text(file_id).cloned()
    }

    /// The [version](FileVersion) of the contents of `file_id`.
    ///
    /// An id the VFS has never seen is reported as [`FileVersion::INITIAL`].
    pub fn file_version(&self, file_id: FileId) -> FileVersion {
        self.file(file_id)
            .map_or(FileVersion::INITIAL, |file| file.version)
    }

    /// Returns an iterator over the stored ids and their corresponding paths.
    ///
    /// Skips the files the compiler cannot work with:
    /// the deleted ones and the ones that cannot be read as text.
    pub fn iter(&self) -> impl Iterator<Item = (FileId, &VfsPath)> + '_ {
        (0..self.files.len())
            .map(|index| FileId::from_raw(index as u32))
            .filter(move |&file_id| self.state(file_id) == FileState::Exists)
            .map(move |file_id| {
                let path = self.interner.lookup(file_id);
                (file_id, path)
            })
    }

    /// Takes a [snapshot](Snapshot) of the current state of the VFS.
    ///
    /// A revision of the compiler --
    /// from draining the changes to producing the diagnostics for them --
    /// reads the files through a single snapshot.
    pub fn snapshot(&self) -> Snapshot<'_> {
        Snapshot { vfs: self }
    }

    /// Update the `path` with the given `contents`. `None` means the file was deleted.
    ///
    /// Returns `true` if the contents of the file changed, and records the change for
    /// [`take_changes`](Vfs::take_changes).
    /// Pushing the same contents again returns `false` and changes nothing.
    ///
    /// Contents that are not valid UTF-8 make the file [`FileState::Unreadable`]:
    /// MLK sources are text, so such a file cannot be compiled,
    /// but the VFS remembers it instead of pretending that it does not exist.
    ///
    /// If the path does not currently exists in the `Vfs`, allocates a new
    /// [`FileId`] for it.
    pub fn set_file_contents(&mut self, path: VfsPath, contents: Option<Vec<u8>>) -> bool {
        let _p = span!(Level::INFO, "Vfs::set_file_contents").entered();
        let file_id = self.alloc_file_id(path);
        let index = file_id.index() as usize;

        // Excluded files are deliberately not tracked: an update must not resurrect them.
        if self.files[index].state() == FileState::Excluded {
            return false;
        }

        let (state, contents) = match contents {
            None => (FileState::Deleted, FileContents::Deleted),
            Some(contents) => match String::from_utf8(contents) {
                Ok(text) => (FileState::Exists, FileContents::Text(Arc::from(text))),
                Err(_) => (FileState::Unreadable, FileContents::Unreadable),
            },
        };

        let old_state = self.files[index].state();

        if !self.files[index].update(contents) {
            return false;
        }

        let version = self.files[index].version;
        self.record_change(file_id, old_state, state, version);

        true
    }

    /// A convenience wrapper around [`set_file_contents`](Vfs::set_file_contents)
    /// for the callers that already hold text:
    /// an editor, a WASM host, a test.
    pub fn set_file_text(&mut self, path: VfsPath, text: Option<String>) -> bool {
        self.set_file_contents(path, text.map(String::into_bytes))
    }

    /// Drains and returns all the changes in the `Vfs`.
    ///
    /// A change is a net effect, not an event:
    /// at most one change per file is reported,
    /// and its [`version`](ChangedFile::version) is the version of the file at this point.
    pub fn take_changes(&mut self) -> IndexMap<FileId, ChangedFile, BuildHasherDefault<FxHasher>> {
        mem::take(&mut self.changes)
    }

    /// Provides a panic-less way to verify file_id validity.
    ///
    /// Returns `true` if the file exists, whether or not its contents can be read:
    /// use [`file_text`](Vfs::file_text) to get the contents themselves.
    pub fn exists(&self, file_id: FileId) -> bool {
        self.state(file_id) != FileState::Deleted
    }

    /// Returns the id associated with `path`
    ///
    /// - If `path` does not exist in the `Vfs`, allocate a new id for it, associated with a
    ///   deleted file;
    /// - Else, returns `path`'s id.
    ///
    /// Does not record a change.
    fn alloc_file_id(&mut self, path: VfsPath) -> FileId {
        let file_id = self.interner.intern(path);
        let index = file_id.index() as usize;
        let len = self.files.len().max(index + 1);
        self.files.resize(len, File::deleted());
        file_id
    }

    /// The record of `file_id`, if the VFS has ever seen it.
    fn file(&self, file_id: FileId) -> Option<&File> {
        self.files.get(file_id.index() as usize)
    }

    /// The contents of `file_id`, if it exists and can be read as text.
    fn text(&self, file_id: FileId) -> Option<&Arc<str>> {
        self.file(file_id).and_then(File::text)
    }

    /// The state of `file_id`, which is [`FileState::Deleted`] if it was never seen.
    fn state(&self, file_id: FileId) -> FileState {
        self.file(file_id).map_or(FileState::Deleted, File::state)
    }

    /// Records the net effect of a transition from `old_state` to `new_state`.
    ///
    /// Several updates between two [`take_changes`](Vfs::take_changes) calls
    /// are coalesced into a single change per file.
    fn record_change(
        &mut self,
        file_id: FileId,
        old_state: FileState,
        new_state: FileState,
        version: FileVersion,
    ) {
        let change = match (
            old_state != FileState::Deleted,
            new_state != FileState::Deleted,
        ) {
            // The file neither existed nor exists: nothing happened.
            (false, false) => return,
            (false, true) => Change::Create,
            (true, false) => Change::Delete,
            (true, true) => Change::Modify,
        };

        match self.changes.entry(file_id) {
            Entry::Occupied(mut entry) => {
                let merged = match (entry.get().change, change) {
                    // The file appeared and disappeared again between two drains:
                    // as far as the consumer is concerned, nothing happened.
                    (Change::Create, Change::Delete) => {
                        entry.shift_remove();
                        return;
                    },
                    // The file was here before the drain, so from the consumer's point of
                    // view this is still the same file, with different contents.
                    (Change::Delete, Change::Create) => Change::Modify,
                    (Change::Create, _) => Change::Create,
                    // The remaining combinations cannot happen:
                    // `Create` requires the file to have been absent and `Delete` requires
                    // it to have been present, so they contradict the recorded change.
                    (_, change) => change,
                };

                let changed_file = entry.get_mut();
                changed_file.change = merged;
                changed_file.version = version;
            },
            Entry::Vacant(entry) => {
                entry.insert(ChangedFile {
                    file_id,
                    version,
                    change,
                });
            },
        }
    }

    /// We cannot ignore excluded files, because this will lead to errors when the client
    /// requests semantic information for them, so we instead mark them specially.
    ///
    /// Exclusion is a property of the configuration of the loader, not of the contents of
    /// the file, so it is not reported by [`take_changes`](Vfs::take_changes):
    /// the driver is the one that excluded the file, and it knows about it.
    pub fn insert_excluded_file(&mut self, path: VfsPath) {
        let file_id = self.alloc_file_id(path);
        let index = file_id.index() as usize;
        self.files[index].update(FileContents::Excluded);
    }
}

/// A read-only view of the state of the [`Vfs`] at the moment it was taken.
///
/// A [`Span`](mlkc_span::Span) is a file id and a range, with no version attached:
/// the range belongs to the text the file had when the span was created.
/// A snapshot is what makes that text reachable again:
///
/// - the snapshot holds a shared borrow of the VFS,
///   so nothing can be updated while a span is being resolved;
/// - the contents of a version never change,
///   so a handle taken from a snapshot
///   stays the text of that version, whoever holds it;
/// - a worker thread clones the handle and parses it,
///   while the driver works with the rest of the files.
///
/// The borrow is as strong as the place the [`Vfs`] lives in:
/// if it ends up behind a lock, the guard has to be held for the whole revision.
#[derive(Copy, Clone, Debug)]
pub struct Snapshot<'vfs> {
    vfs: &'vfs Vfs,
}

impl<'vfs> Snapshot<'vfs> {
    /// The contents of `file_id` as of this snapshot,
    /// or `None` if the file does not exist or cannot be read as text.
    ///
    /// The handle outlives the snapshot value itself and can be cloned into a worker thread.
    pub fn file_text(&self, file_id: FileId) -> Option<&'vfs Arc<str>> {
        self.vfs.text(file_id)
    }

    /// The [version](FileVersion) of `file_id` as of this snapshot.
    pub fn file_version(&self, file_id: FileId) -> FileVersion {
        self.vfs.file_version(file_id)
    }

    /// What the VFS knows about `file_id` as of this snapshot.
    pub fn file_state(&self, file_id: FileId) -> FileState {
        self.vfs.file_state(file_id)
    }

    /// The path of `file_id`.
    ///
    /// # Panics
    ///
    /// Panics if the id is not present in the `Vfs`.
    pub fn file_path(&self, file_id: FileId) -> &'vfs VfsPath {
        self.vfs.file_path(file_id)
    }

    /// The files the compiler can work with, as of this snapshot.
    pub fn iter(&self) -> impl Iterator<Item = (FileId, &'vfs VfsPath)> + 'vfs {
        self.vfs.iter()
    }
}

impl fmt::Debug for Vfs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Vfs")
            .field("n_files", &self.files.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path(name: &str) -> VfsPath {
        VfsPath::new_virtual_path(format!("/{name}"))
    }

    fn contents(text: &str) -> Option<Vec<u8>> {
        Some(text.as_bytes().to_vec())
    }

    #[test]
    fn contents_are_stored_and_versioned() {
        let mut vfs = Vfs::default();
        let path = path("main.mlk");

        assert!(vfs.set_file_contents(path.clone(), contents("let x = 1;")));

        let (file_id, excluded) = vfs.file_id(&path).expect("the file exists");
        assert_eq!(excluded, FileExcluded::No);
        assert_eq!(vfs.file_state(file_id), FileState::Exists);
        assert_eq!(vfs.file_text(file_id).as_deref(), Some("let x = 1;"));
        assert_eq!(vfs.file_version(file_id).raw(), 1);

        // The same contents again: no work for the compiler, no invalidation.
        assert!(!vfs.set_file_contents(path.clone(), contents("let x = 1;")));
        assert_eq!(vfs.file_version(file_id).raw(), 1);

        assert!(vfs.set_file_contents(path, contents("let y = 2;")));
        assert_eq!(vfs.file_text(file_id).as_deref(), Some("let y = 2;"));
        assert_eq!(vfs.file_version(file_id).raw(), 2);
    }

    #[test]
    fn changes_are_reported_once_per_file() {
        let mut vfs = Vfs::default();
        let path = path("main.mlk");

        vfs.set_file_contents(path.clone(), contents("a"));
        vfs.set_file_contents(path.clone(), contents("ab"));
        vfs.set_file_contents(path.clone(), contents("abc"));

        let changes = vfs.take_changes();
        assert_eq!(changes.len(), 1);

        let change = changes.values().next().expect("the file changed");
        assert_eq!(change.change, Change::Create);
        assert_eq!(change.kind(), ChangeKind::Create);
        assert_eq!(change.version.raw(), 3);
        assert!(change.is_created());
        assert!(change.exists());

        // The changes were drained: nothing is reported twice.
        assert!(vfs.take_changes().is_empty());

        vfs.set_file_contents(path.clone(), contents("abcd"));
        let changes = vfs.take_changes();
        let change = changes.values().next().expect("the file changed");
        assert_eq!(change.change, Change::Modify);
        assert!(change.is_modified());

        vfs.set_file_contents(path, None);
        let changes = vfs.take_changes();
        let change = changes.values().next().expect("the file changed");
        assert_eq!(change.change, Change::Delete);
        assert!(change.is_created_or_deleted());
        assert!(!change.exists());
    }

    #[test]
    fn a_file_that_appears_and_disappears_is_not_a_change() {
        let mut vfs = Vfs::default();

        vfs.set_file_contents(path("scratch.mlk"), contents("..."));
        vfs.set_file_contents(path("scratch.mlk"), None);

        assert!(vfs.take_changes().is_empty());
    }

    #[test]
    fn a_recreated_file_is_a_modification() {
        let mut vfs = Vfs::default();
        let path = path("main.mlk");

        vfs.set_file_contents(path.clone(), contents("a"));
        vfs.take_changes();

        vfs.set_file_contents(path.clone(), None);
        vfs.set_file_contents(path, contents("b"));

        let changes = vfs.take_changes();
        let change = changes.values().next().expect("the file changed");
        assert_eq!(change.change, Change::Modify);
    }

    #[test]
    fn deleted_files_keep_their_ids_but_not_their_contents() {
        let mut vfs = Vfs::default();
        let path = path("main.mlk");

        vfs.set_file_contents(path.clone(), contents("let x = 1;"));
        let (file_id, _) = vfs.file_id(&path).expect("the file exists");

        vfs.set_file_contents(path.clone(), None);

        assert_eq!(vfs.file_id(&path), None);
        assert!(!vfs.exists(file_id));
        assert_eq!(vfs.file_state(file_id), FileState::Deleted);
        assert_eq!(vfs.file_text(file_id), None);
        assert_eq!(vfs.iter().count(), 0);
    }

    #[test]
    fn a_worker_keeps_the_contents_it_started_with() {
        let mut vfs = Vfs::default();
        let path = path("main.mlk");

        vfs.set_file_contents(path.clone(), contents("let x = 1;"));
        let (file_id, _) = vfs.file_id(&path).expect("the file exists");
        let old = vfs.file_text(file_id).expect("the file has contents");

        vfs.set_file_contents(path, contents("let y = 2;"));

        // The contents of a version never change under the reader's feet.
        assert_eq!(old.as_ref(), "let x = 1;");
        assert_eq!(vfs.file_text(file_id).as_deref(), Some("let y = 2;"));
    }

    #[test]
    fn a_snapshot_pins_the_revision() {
        let mut vfs = Vfs::default();
        let path = path("main.mlk");

        vfs.set_file_contents(path.clone(), contents("let x = 1;"));
        let (file_id, _) = vfs.file_id(&path).expect("the file exists");

        // A revision reads the state of the world through one snapshot.
        let (text, version) = {
            let snapshot = vfs.snapshot();

            assert_eq!(snapshot.file_state(file_id), FileState::Exists);
            assert_eq!(snapshot.file_path(file_id), &path);
            assert_eq!(snapshot.iter().count(), 1);

            (
                snapshot.file_text(file_id).cloned(),
                snapshot.file_version(file_id),
            )
        };

        // The file can only be updated because the snapshot is gone,
        // while the handle it handed out still points at the text of its version.
        vfs.set_file_contents(path, contents("let y = 2;"));

        assert_eq!(text.as_deref(), Some("let x = 1;"));
        assert_eq!(version.raw(), 1);
        assert!(version < vfs.file_version(file_id));
    }

    #[test]
    fn contents_that_are_not_text_make_the_file_unreadable() {
        let mut vfs = Vfs::default();
        let path = path("binary.mlk");

        vfs.set_file_contents(path.clone(), Some(vec![0xff, 0xfe]));

        let (file_id, excluded) = vfs.file_id(&path).expect("the file exists");
        assert_eq!(excluded, FileExcluded::No);
        assert_eq!(vfs.file_state(file_id), FileState::Unreadable);
        assert_eq!(vfs.file_text(file_id), None);
        assert!(vfs.exists(file_id));
        // The compiler can only work with the files it can read.
        assert_eq!(vfs.iter().count(), 0);

        let changes = vfs.take_changes();
        let change = changes.values().next().expect("the file changed");
        assert_eq!(change.change, Change::Create);

        // The file is then fixed, which is a change the compiler has to see.
        vfs.set_file_contents(path, contents("let x = 1;"));

        assert_eq!(vfs.file_state(file_id), FileState::Exists);
        assert_eq!(vfs.file_text(file_id).as_deref(), Some("let x = 1;"));
        assert_eq!(vfs.iter().count(), 1);

        let changes = vfs.take_changes();
        let change = changes.values().next().expect("the file changed");
        assert_eq!(change.change, Change::Modify);
    }

    #[test]
    fn excluded_files_are_never_updated() {
        let mut vfs = Vfs::default();
        let path = path("outside.mlk");

        vfs.insert_excluded_file(path.clone());

        let (file_id, excluded) = vfs.file_id(&path).expect("the file exists");
        assert_eq!(excluded, FileExcluded::Yes);
        assert_eq!(vfs.file_state(file_id), FileState::Excluded);
        assert!(!vfs.set_file_contents(path, contents("let x = 1;")));
        assert_eq!(vfs.file_text(file_id), None);
    }

    #[test]
    fn the_vfs_is_empty_until_something_is_pushed_to_it() {
        let mut vfs = Vfs::default();
        let path = path("unknown.mlk");

        assert_eq!(vfs.file_id(&path), None);
        assert!(vfs.take_changes().is_empty());
        assert_eq!(vfs.iter().count(), 0);

        // A path pushed as "deleted" is not an event either.
        assert!(!vfs.set_file_contents(path.clone(), None));
        assert!(vfs.take_changes().is_empty());
        assert_eq!(vfs.file_id(&path), None);
    }
}
