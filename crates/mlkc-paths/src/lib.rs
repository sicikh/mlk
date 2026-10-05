//! Thin wrappers around [`camino`], distinguishing between absolute and
//! relative paths.

use std::{
    borrow::Borrow,
    ffi::OsStr,
    fmt, ops,
    path::{Path, PathBuf},
};

pub use camino::{Utf8Component, Utf8Components, Utf8Path, Utf8PathBuf, Utf8Prefix};

/// A [`Utf8PathBuf`] that is guaranteed to be absolute.
#[derive(Debug, Clone, Ord, PartialOrd, Eq, Hash)]
pub struct AbsPathBuf(Utf8PathBuf);

impl From<AbsPathBuf> for Utf8PathBuf {
    fn from(AbsPathBuf(path_buf): AbsPathBuf) -> Utf8PathBuf {
        path_buf
    }
}

impl From<AbsPathBuf> for PathBuf {
    fn from(AbsPathBuf(path_buf): AbsPathBuf) -> PathBuf {
        path_buf.into()
    }
}

impl ops::Deref for AbsPathBuf {
    type Target = AbsPath;
    fn deref(&self) -> &AbsPath {
        self.as_path()
    }
}

impl AsRef<Utf8Path> for AbsPathBuf {
    fn as_ref(&self) -> &Utf8Path {
        self.0.as_path()
    }
}

impl AsRef<OsStr> for AbsPathBuf {
    fn as_ref(&self) -> &OsStr {
        self.0.as_ref()
    }
}

impl AsRef<Path> for AbsPathBuf {
    fn as_ref(&self) -> &Path {
        self.0.as_ref()
    }
}

impl AsRef<AbsPath> for AbsPathBuf {
    fn as_ref(&self) -> &AbsPath {
        self.as_path()
    }
}

impl Borrow<AbsPath> for AbsPathBuf {
    fn borrow(&self) -> &AbsPath {
        self.as_path()
    }
}

impl TryFrom<Utf8PathBuf> for AbsPathBuf {
    type Error = Utf8PathBuf;
    fn try_from(path_buf: Utf8PathBuf) -> Result<AbsPathBuf, Utf8PathBuf> {
        if !path_buf.is_absolute() {
            return Err(path_buf);
        }
        Ok(AbsPathBuf(path_buf))
    }
}

impl TryFrom<&str> for AbsPathBuf {
    type Error = Utf8PathBuf;
    fn try_from(path: &str) -> Result<AbsPathBuf, Utf8PathBuf> {
        AbsPathBuf::try_from(Utf8PathBuf::from(path))
    }
}

impl<P: AsRef<Path> + ?Sized> PartialEq<P> for AbsPathBuf {
    fn eq(&self, other: &P) -> bool {
        self.0.as_std_path() == other.as_ref()
    }
}

impl AbsPathBuf {
    /// Wrap the given absolute path in `AbsPathBuf`
    ///
    /// # Panics
    ///
    /// Panics if `path` is not absolute.
    pub fn assert(path: Utf8PathBuf) -> AbsPathBuf {
        AbsPathBuf::try_from(path)
            .unwrap_or_else(|path| panic!("expected absolute path, got {path}"))
    }

    /// Wrap the given absolute path in `AbsPathBuf`
    ///
    /// # Panics
    ///
    /// Panics if `path` is not absolute.
    pub fn assert_utf8(path: PathBuf) -> AbsPathBuf {
        AbsPathBuf::assert(
            Utf8PathBuf::from_path_buf(path)
                .unwrap_or_else(|path| panic!("expected utf8 path, got {}", path.display())),
        )
    }

    /// Coerces to an `AbsPath` slice.
    ///
    /// Equivalent of [`Utf8PathBuf::as_path`] for `AbsPathBuf`.
    pub fn as_path(&self) -> &AbsPath {
        AbsPath::assert(self.0.as_path())
    }

    /// Equivalent of [`Utf8PathBuf::pop`] for `AbsPathBuf`.
    ///
    /// Note that this won't remove the root component, so `self` will still be
    /// absolute.
    pub fn pop(&mut self) -> bool {
        self.0.pop()
    }

    /// Equivalent of [`PathBuf::push`] for `AbsPathBuf`.
    ///
    /// Extends `self` with `path`.
    ///
    /// If `path` is absolute, it replaces the current path.
    ///
    /// On Windows:
    ///
    /// * if `path` has a root but no prefix (e.g., `\windows`), it
    ///   replaces everything except for the prefix (if any) of `self`.
    /// * if `path` has a prefix but no root, it replaces `self`.
    /// * if `self` has a verbatim prefix (e.g. `\\?\C:\windows`)
    ///   and `path` is not empty, the new path is normalized: all references
    ///   to `.` and `..` are removed.
    pub fn push<P: AsRef<Utf8Path>>(&mut self, suffix: P) {
        self.0.push(suffix)
    }

    pub fn join(&self, path: impl AsRef<Utf8Path>) -> Self {
        Self(self.0.join(path))
    }
}

impl fmt::Display for AbsPathBuf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

/// Wrapper around an absolute [`Utf8Path`].
#[derive(Debug, Ord, PartialOrd, Eq, Hash)]
#[repr(transparent)]
pub struct AbsPath(Utf8Path);

impl<P: AsRef<Path> + ?Sized> PartialEq<P> for AbsPath {
    fn eq(&self, other: &P) -> bool {
        self.0.as_std_path() == other.as_ref()
    }
}

impl AsRef<Utf8Path> for AbsPath {
    fn as_ref(&self) -> &Utf8Path {
        &self.0
    }
}

impl AsRef<Path> for AbsPath {
    fn as_ref(&self) -> &Path {
        self.0.as_ref()
    }
}

impl AsRef<OsStr> for AbsPath {
    fn as_ref(&self) -> &OsStr {
        self.0.as_ref()
    }
}

impl ToOwned for AbsPath {
    type Owned = AbsPathBuf;

    fn to_owned(&self) -> Self::Owned {
        AbsPathBuf(self.0.to_owned())
    }
}

impl<'a> TryFrom<&'a Utf8Path> for &'a AbsPath {
    type Error = &'a Utf8Path;
    fn try_from(path: &'a Utf8Path) -> Result<&'a AbsPath, &'a Utf8Path> {
        if !path.is_absolute() {
            return Err(path);
        }
        Ok(AbsPath::assert(path))
    }
}

impl AbsPath {
    /// Wrap the given absolute path in `AbsPath`
    ///
    /// # Panics
    ///
    /// Panics if `path` is not absolute.
    pub fn assert(path: &Utf8Path) -> &AbsPath {
        assert!(path.is_absolute(), "{path} is not absolute");
        unsafe { &*(path as *const Utf8Path as *const AbsPath) }
    }

    /// Equivalent of [`Utf8Path::parent`] for `AbsPath`.
    pub fn parent(&self) -> Option<&AbsPath> {
        self.0.parent().map(AbsPath::assert)
    }

    /// Equivalent of [`Utf8Path::join`] for `AbsPath` with an additional normalize step afterwards.
    pub fn absolutize(&self, path: impl AsRef<Utf8Path>) -> AbsPathBuf {
        self.join(path).normalize()
    }

    /// Equivalent of [`Utf8Path::join`] for `AbsPath`.
    pub fn join(&self, path: impl AsRef<Utf8Path>) -> AbsPathBuf {
        Utf8Path::join(self.as_ref(), path).try_into().unwrap()
    }

    /// Normalize the given path:
    /// - Removes repeated separators: `/a//b` becomes `/a/b`
    /// - Removes occurrences of `.` and resolves `..`.
    /// - Removes trailing slashes: `/a/b/` becomes `/a/b`.
    ///
    /// # Example
    /// ```ignore
    /// # use paths::AbsPathBuf;
    /// let abs_path_buf = AbsPathBuf::assert("/a/../../b/.//c//".into());
    /// let normalized = abs_path_buf.normalize();
    /// assert_eq!(normalized, AbsPathBuf::assert("/b/c".into()));
    /// ```
    pub fn normalize(&self) -> AbsPathBuf {
        AbsPathBuf(normalize_path(&self.0))
    }

    /// Equivalent of [`Utf8Path::to_path_buf`] for `AbsPath`.
    pub fn to_path_buf(&self) -> AbsPathBuf {
        AbsPathBuf::try_from(self.0.to_path_buf()).unwrap()
    }

    /// Equivalent of [`Utf8Path::strip_prefix`] for `AbsPath`.
    ///
    /// Returns a relative path.
    pub fn strip_prefix(&self, base: &AbsPath) -> Option<&RelPath> {
        self.0.strip_prefix(base).ok().map(RelPath::new_unchecked)
    }
    pub fn starts_with(&self, base: &AbsPath) -> bool {
        self.0.starts_with(&base.0)
    }
    pub fn ends_with(&self, suffix: &RelPath) -> bool {
        self.0.ends_with(&suffix.0)
    }

    pub fn name_and_extension(&self) -> Option<(&str, Option<&str>)> {
        Some((self.file_stem()?, self.extension()))
    }

    // Note that we deliberately don't implement `Deref<Target = Utf8Path>` here.
    //
    // The problem with `Utf8Path` is that it directly exposes convenience IO-ing
    // methods. For example, `Utf8Path::exists` delegates to `fs::metadata`.
    //
    // For `AbsPath`, we want to make sure that this is a POD type, and that all
    // IO goes via `fs`. That way, it becomes easier to mock IO when we need it.

    pub fn file_name(&self) -> Option<&str> {
        self.0.file_name()
    }
    pub fn extension(&self) -> Option<&str> {
        self.0.extension()
    }
    pub fn file_stem(&self) -> Option<&str> {
        self.0.file_stem()
    }
    pub fn as_os_str(&self) -> &OsStr {
        self.0.as_os_str()
    }
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn components(&self) -> Utf8Components<'_> {
        self.0.components()
    }
}

impl fmt::Display for AbsPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

/// Wrapper around a relative [`Utf8PathBuf`].
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct RelPathBuf(Utf8PathBuf);

impl From<RelPathBuf> for Utf8PathBuf {
    fn from(RelPathBuf(path_buf): RelPathBuf) -> Utf8PathBuf {
        path_buf
    }
}

impl ops::Deref for RelPathBuf {
    type Target = RelPath;
    fn deref(&self) -> &RelPath {
        self.as_path()
    }
}

impl AsRef<Utf8Path> for RelPathBuf {
    fn as_ref(&self) -> &Utf8Path {
        self.0.as_path()
    }
}

impl AsRef<Path> for RelPathBuf {
    fn as_ref(&self) -> &Path {
        self.0.as_ref()
    }
}

impl TryFrom<Utf8PathBuf> for RelPathBuf {
    type Error = Utf8PathBuf;
    fn try_from(path_buf: Utf8PathBuf) -> Result<RelPathBuf, Utf8PathBuf> {
        if !path_buf.is_relative() {
            return Err(path_buf);
        }
        Ok(RelPathBuf(path_buf))
    }
}

impl TryFrom<&str> for RelPathBuf {
    type Error = Utf8PathBuf;
    fn try_from(path: &str) -> Result<RelPathBuf, Utf8PathBuf> {
        RelPathBuf::try_from(Utf8PathBuf::from(path))
    }
}

impl RelPathBuf {
    /// Coerces to a `RelPath` slice.
    ///
    /// Equivalent of [`Utf8PathBuf::as_path`] for `RelPathBuf`.
    pub fn as_path(&self) -> &RelPath {
        RelPath::new_unchecked(self.0.as_path())
    }
}

/// Wrapper around a relative [`Utf8Path`].
#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
#[repr(transparent)]
pub struct RelPath(Utf8Path);

impl AsRef<Utf8Path> for RelPath {
    fn as_ref(&self) -> &Utf8Path {
        &self.0
    }
}

impl AsRef<Path> for RelPath {
    fn as_ref(&self) -> &Path {
        self.0.as_ref()
    }
}

impl RelPath {
    /// Creates a new `RelPath` from `path`, without checking if it is relative.
    pub fn new_unchecked(path: &Utf8Path) -> &RelPath {
        unsafe { &*(path as *const Utf8Path as *const RelPath) }
    }

    /// Equivalent of [`Utf8Path::to_path_buf`] for `RelPath`.
    pub fn to_path_buf(&self) -> RelPathBuf {
        RelPathBuf::try_from(self.0.to_path_buf()).unwrap()
    }

    pub fn as_utf8_path(&self) -> &Utf8Path {
        self.as_ref()
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// Taken from <https://github.com/rust-lang/cargo/blob/79c769c3d7b4c2cf6a93781575b7f592ef974255/src/cargo/util/paths.rs#L60-L85>
fn normalize_path(path: &Utf8Path) -> Utf8PathBuf {
    let mut components = path.components().peekable();
    let mut ret = if let Some(c @ Utf8Component::Prefix(..)) = components.peek().copied() {
        components.next();
        Utf8PathBuf::from(c.as_str())
    } else {
        Utf8PathBuf::new()
    };

    for component in components {
        match component {
            Utf8Component::Prefix(..) => unreachable!(),
            Utf8Component::RootDir => {
                ret.push(component.as_str());
            },
            Utf8Component::CurDir => {},
            Utf8Component::ParentDir => {
                ret.pop();
            },
            Utf8Component::Normal(c) => {
                ret.push(c);
            },
        }
    }
    ret
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    /// An absolute path for a test, which must be absolute for `assert` not to panic.
    fn abs(path: &str) -> &AbsPath {
        AbsPath::assert(Utf8Path::new(path))
    }

    /// An absolute path buffer for a test, which must be absolute for `assert` not to panic.
    fn abs_buf(path: &str) -> AbsPathBuf {
        AbsPathBuf::assert(Utf8PathBuf::from(path))
    }

    /// A relative path buffer for a test, which must be relative for `try_from` to accept it.
    fn rel_buf(path: &str) -> RelPathBuf {
        RelPathBuf::try_from(path).unwrap()
    }

    #[test]
    fn try_from_rejects_a_relative_path_and_returns_it() {
        let relative = Utf8PathBuf::from("a/b");

        assert_eq!(AbsPathBuf::try_from(relative.clone()), Err(relative));

        let rejected = AbsPathBuf::try_from("a/b").unwrap_err();
        assert_eq!(rejected, Utf8PathBuf::from("a/b"));
    }

    #[test]
    fn an_abs_path_borrows_from_an_absolute_utf8_path_only() {
        let absolute = Utf8Path::new("/a/b");
        assert_eq!(<&AbsPath>::try_from(absolute).unwrap(), absolute);

        let relative = Utf8Path::new("a/b");
        assert_eq!(<&AbsPath>::try_from(relative), Err(relative));
    }

    #[test]
    #[should_panic(expected = "expected absolute path")]
    fn assert_rejects_a_relative_path() {
        AbsPathBuf::assert(Utf8PathBuf::from("a/b"));
    }

    #[test]
    #[should_panic(expected = "is not absolute")]
    fn abs_path_assert_rejects_a_relative_path() {
        AbsPath::assert(Utf8Path::new("a/b"));
    }

    #[test]
    fn assert_utf8_wraps_an_absolute_os_path() {
        assert_eq!(AbsPathBuf::assert_utf8(PathBuf::from("/a/b")), "/a/b");
    }

    #[cfg(unix)]
    #[test]
    #[should_panic(expected = "expected utf8 path")]
    fn assert_utf8_rejects_a_path_that_is_not_utf8() {
        use std::os::unix::ffi::OsStringExt;

        let path = PathBuf::from(std::ffi::OsString::from_vec(vec![b'/', 0xFF]));
        AbsPathBuf::assert_utf8(path);
    }

    #[test]
    fn an_abs_path_buf_lends_an_abs_path_of_the_same_path() {
        let path = abs_buf("/a/b");

        assert_eq!(path.as_path(), abs("/a/b"));
        assert_eq!(path.file_name(), Some("b"), "the buffer derefs to its path");
    }

    #[test]
    fn pop_truncates_to_the_parent_and_stops_at_the_root() {
        let mut path = abs_buf("/a/b");

        assert!(path.pop());
        assert_eq!(path, "/a");

        assert!(path.pop());
        assert_eq!(path, "/");

        assert!(!path.pop(), "the root has no parent to pop to");
        assert_eq!(path, "/");
    }

    #[test]
    fn push_extends_the_path_and_an_absolute_suffix_replaces_it() {
        let mut path = abs_buf("/a");
        path.push("b");
        assert_eq!(path, "/a/b");

        path.push("/x/y");
        assert_eq!(path, "/x/y", "an absolute path starts over");
    }

    #[test]
    fn join_appends_without_normalizing_but_absolutize_resolves_the_result() {
        let base = abs_buf("/a/b");

        assert_eq!(base.join("c"), "/a/b/c");
        assert_eq!(base.join("../c"), "/a/b/../c");
        assert_eq!(base.join("/x"), "/x", "an absolute path replaces the base");

        assert_eq!(base.as_path().absolutize("../c"), "/a/c");
    }

    #[test]
    fn normalize_removes_cur_dir_repeated_separators_and_parent_dir() {
        assert_eq!(abs_buf("/a/../../b/.//c//").normalize(), "/b/c");
    }

    #[test]
    fn normalize_cannot_climb_above_the_root() {
        assert_eq!(abs_buf("/../a").normalize(), "/a");
    }

    #[test]
    fn parent_walks_up_to_the_root_and_stops() {
        assert_eq!(abs("/a/b").parent(), Some(abs("/a")));
        assert_eq!(abs("/a").parent(), Some(abs("/")));
        assert_eq!(abs("/").parent(), None);
    }

    #[test]
    fn strip_prefix_returns_the_rest_of_the_path() {
        let path = abs("/a/b/c");

        assert_eq!(
            path.strip_prefix(abs("/a")).map(RelPath::as_str),
            Some("b/c")
        );
        assert_eq!(
            path.strip_prefix(abs("/a/b/c")).map(RelPath::as_str),
            Some("")
        );
        assert_eq!(path.strip_prefix(abs("/a/bc")).map(RelPath::as_str), None);
        assert_eq!(path.strip_prefix(abs("/x")).map(RelPath::as_str), None);
    }

    #[test]
    fn starts_with_matches_whole_components() {
        let path = abs("/a/b/c");

        assert!(path.starts_with(abs("/a")));
        assert!(path.starts_with(abs("/a/b")));
        assert!(!path.starts_with(abs("/a/bc")));
        assert!(!path.starts_with(abs("/x")));
    }

    #[test]
    fn ends_with_matches_whole_components() {
        let path = abs("/a/b/c");

        assert!(path.ends_with(rel_buf("b/c").as_path()));
        assert!(path.ends_with(rel_buf("c").as_path()));
        assert!(!path.ends_with(rel_buf("bc").as_path()));
        assert!(!path.ends_with(rel_buf("x/c").as_path()));
    }

    #[test]
    fn name_and_extension_split_the_file_stem_from_its_extension() {
        assert_eq!(
            abs("/src/main.mlk").name_and_extension(),
            Some(("main", Some("mlk")))
        );
        assert_eq!(abs("/src/main").name_and_extension(), Some(("main", None)));
        assert_eq!(abs("/").name_and_extension(), None);
    }

    #[test]
    fn file_name_extension_and_stem_read_the_last_component() {
        let path = abs("/src/main.mlk");

        assert_eq!(path.file_name(), Some("main.mlk"));
        assert_eq!(path.extension(), Some("mlk"));
        assert_eq!(path.file_stem(), Some("main"));

        assert_eq!(abs("/src/main").extension(), None);
        assert_eq!(abs("/").file_name(), None);
    }

    #[test]
    fn display_writes_the_path_it_wraps() {
        assert_eq!(abs_buf("/a/b").to_string(), "/a/b");
        assert_eq!(abs("/a/b").to_string(), "/a/b");
    }

    #[test]
    fn conversions_hand_back_the_path_beneath_the_wrapper() {
        let path = abs_buf("/a/b");

        let utf8: Utf8PathBuf = path.clone().into();
        let std_path: PathBuf = path.clone().into();
        let as_utf8: &Utf8Path = path.as_ref();
        let as_std: &Path = path.as_ref();
        let as_os: &OsStr = path.as_ref();
        let as_abs: &AbsPath = path.as_ref();

        assert_eq!(utf8, "/a/b");
        assert_eq!(std_path, Path::new("/a/b"));
        assert_eq!(as_utf8, Utf8Path::new("/a/b"));
        assert_eq!(as_std, Path::new("/a/b"));
        assert_eq!(as_os, OsStr::new("/a/b"));
        assert_eq!(as_abs, abs("/a/b"));
    }

    #[test]
    fn an_abs_path_buf_is_found_under_its_abs_path_borrow() {
        let mut paths = HashMap::new();
        paths.insert(abs_buf("/a/b"), "value");

        assert_eq!(paths.get(abs("/a/b")), Some(&"value"));
    }

    #[test]
    fn to_path_buf_and_to_owned_copy_the_path() {
        let path = abs("/a/b");

        assert_eq!(path.to_path_buf().as_path(), path);

        let owned = <AbsPath as ToOwned>::to_owned(path);
        assert_eq!(owned.as_path(), path);
    }

    #[test]
    fn a_rel_path_buf_accepts_only_a_relative_path() {
        assert_eq!(
            rel_buf("a/b").as_str(),
            "a/b",
            "the buffer derefs to its path"
        );

        let rejected = RelPathBuf::try_from("/a/b").unwrap_err();
        assert_eq!(
            rejected,
            Utf8PathBuf::from("/a/b"),
            "the rejected path comes back unchanged"
        );
    }

    #[test]
    fn a_rel_path_can_borrow_a_relative_path_without_a_check() {
        let path = Utf8Path::new("a/b");
        let rel = RelPath::new_unchecked(path);

        assert_eq!(rel.as_str(), "a/b");
        assert_eq!(rel.as_utf8_path(), path);
        assert_eq!(rel.to_path_buf(), rel_buf("a/b"));
    }

    #[test]
    fn a_rel_path_buf_converts_into_a_utf8_path_buf() {
        let converted: Utf8PathBuf = rel_buf("a/b").into();

        assert_eq!(converted, Utf8PathBuf::from("a/b"));
    }
}
