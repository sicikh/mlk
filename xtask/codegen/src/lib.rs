//! Codegen tools for generating Syntax and AST definitions. Derived from Rust analyzer's codegen

// The generator is a tool: the modules below are its own API, and the notes it prints are
// the report a person reads after running it.
#![expect(
    unreachable_pub,
    reason = "the generator is a tool whose modules are its own API; only its entry points are named"
)]
#![expect(
    clippy::print_stdout,
    reason = "the generator reports every file it updated by printing it"
)]

mod ast;
mod generate_macros;
mod generate_node_factory;
mod generate_nodes;
mod generate_nodes_mut;
mod generate_syntax_factory;
mod generate_syntax_kinds;
mod mlk_kinds_src;

mod kind_src;
mod language_kind;
mod termcolorful;

use std::path::Path;

use bpaf::Bpaf;
use xtask_glue::{Mode, Result, glue::fs2};

pub use self::ast::generate_ast;

pub enum UpdateResult {
    NotUpdated,
    Updated,
}

/// Updates the file at `path` with `contents` when the file on disk differs from them.
///
/// In [`Mode::Verify`] a file that differs makes the call fail instead of being written;
/// in [`Mode::Overwrite`] the file is written,
/// creating the parent directories if they are missing.
///
/// # Errors
///
/// Fails when [`Mode::Verify`] is passed and the file does not hold `contents`,
/// including when it does not exist or cannot be read,
/// or when the parent directories cannot be created or the file cannot be written.
pub fn update(path: &Path, contents: &str, mode: &Mode) -> Result<UpdateResult> {
    match fs2::read_to_string(path) {
        Ok(old_contents) if old_contents == contents => {
            return Ok(UpdateResult::NotUpdated);
        },
        _ => (),
    }

    if *mode == Mode::Verify {
        anyhow::bail!("`{}` is not up-to-date", path.display());
    }

    eprintln!("updating {}", path.display());
    if let Some(parent) = path.parent()
        && !parent.exists()
    {
        fs2::create_dir_all(parent)?;
    }
    fs2::write(path, contents)?;
    Ok(UpdateResult::Updated)
}

pub fn to_capitalized(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

#[derive(Debug, Clone, Bpaf)]
#[bpaf(options)]
pub enum TaskCommand {
    /// Transforms ungram files into AST
    #[bpaf(command)]
    Grammar(Vec<String>),
    /// Runs ALL the codegen
    #[bpaf(command)]
    All,
    /// Checks that the generated files are what the generators make of their sources
    #[bpaf(command)]
    Check,
}

#[cfg(test)]
mod tests {
    use std::{
        env, fs,
        path::{Path, PathBuf},
        sync::atomic::{AtomicUsize, Ordering},
    };

    use super::{Mode, UpdateResult, to_capitalized, update};

    struct TestDir(PathBuf);

    impl TestDir {
        fn new() -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);

            let path = env::temp_dir().join(format!(
                "xtask-codegen-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed),
            ));
            // A recycled pid can find the directory of a crashed earlier run.
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();

            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn update_reports_not_updated_for_a_file_that_already_has_the_contents() {
        for mode in [Mode::Verify, Mode::Overwrite] {
            let dir = TestDir::new();
            let path = dir.path().join("file.txt");
            fs::write(&path, "contents").unwrap();

            let result = update(&path, "contents", &mode).unwrap();

            assert!(matches!(result, UpdateResult::NotUpdated), "mode {mode:?}");
            assert_eq!(fs::read_to_string(&path).unwrap(), "contents");
        }
    }

    #[test]
    fn update_in_verify_mode_refuses_a_stale_file_and_leaves_it_untouched() {
        let dir = TestDir::new();
        let path = dir.path().join("file.txt");
        fs::write(&path, "old contents").unwrap();

        let error = update(&path, "new contents", &Mode::Verify)
            .err()
            .expect("verify mode to refuse a stale file");

        assert!(error.to_string().contains("not up-to-date"), "{error}");
        assert_eq!(fs::read_to_string(&path).unwrap(), "old contents");
    }

    #[test]
    fn update_in_verify_mode_refuses_a_file_that_does_not_exist() {
        let dir = TestDir::new();
        let path = dir.path().join("missing.txt");

        let error = update(&path, "contents", &Mode::Verify)
            .err()
            .expect("verify mode to refuse a missing file");

        assert!(error.to_string().contains("not up-to-date"), "{error}");
        assert!(!path.exists());
    }

    #[test]
    fn update_in_overwrite_mode_replaces_a_stale_file() {
        let dir = TestDir::new();
        let path = dir.path().join("file.txt");
        fs::write(&path, "old contents").unwrap();

        let result = update(&path, "new contents", &Mode::Overwrite).unwrap();

        assert!(matches!(result, UpdateResult::Updated));
        assert_eq!(fs::read_to_string(&path).unwrap(), "new contents");
    }

    #[test]
    fn update_in_overwrite_mode_creates_the_parent_directories_of_a_new_file() {
        let dir = TestDir::new();
        let path = dir.path().join("nested").join("file.txt");

        let result = update(&path, "contents", &Mode::Overwrite).unwrap();

        assert!(matches!(result, UpdateResult::Updated));
        assert_eq!(fs::read_to_string(&path).unwrap(), "contents");
    }

    #[test]
    fn to_capitalized_returns_an_empty_string_for_empty_input() {
        assert_eq!(to_capitalized(""), "");
    }

    #[test]
    fn to_capitalized_uppercases_the_first_character_only() {
        assert_eq!(to_capitalized("foo"), "Foo");
        assert_eq!(to_capitalized("Foo"), "Foo");
        assert_eq!(to_capitalized("fooBar"), "FooBar");
        assert_eq!(to_capitalized("f"), "F");
    }

    #[test]
    fn to_capitalized_uppercases_a_non_ascii_first_character() {
        assert_eq!(to_capitalized("éclair"), "Éclair");
    }
}
