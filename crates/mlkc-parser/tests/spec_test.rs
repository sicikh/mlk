//! The harness of the spec tests: parse a fixture, and compare the parse with the snapshot
//! next to it.
//!
//! A snapshot holds what a person reads when a rule changes: the source, the AST that a
//! reader of the tree sees, the tree the parser produced, and the diagnostics. Every
//! fixture has one, and the snapshot is part of changing a rule:
//! `INSTA_UPDATE=always cargo test -p mlkc-parser` rewrites them, and the diff of the
//! snapshots is what the review reads.
//!
//! The directory of a fixture tells what its parse is expected to be: the snapshots under
//! `specs/valid` must be diagnostics-free and hold no bogus or incomplete node at all,
//! while the ones under `specs/invalid` are a mistake and where the parser got to after it.

use std::{
    fmt::Write,
    fs,
    path::{Path, PathBuf},
};

use mlkc_parser_core::AnyParse;
use mlkc_rowan::SyntaxKind as SyntaxKindTrait;
use mlkc_syntax::{MODULE_ROOT, MlkLanguage, ModuleRoot};

/// The directory of the tests of this crate.
///
/// The fixtures are read from here rather than from the working directory of the test, so
/// that a test does not depend on where it was started from.
const TESTS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests");

/// The directory of the fixtures, relative to [TESTS_DIR].
///
/// A snapshot is written next to the fixture it describes, and the path of a snapshot that
/// insta resolves is relative to the file that asserts it — here, `tests/spec_test.rs`.
/// The path handed to insta is therefore spelled relative to the directory of the tests,
/// while the fixtures themselves are read from [TESTS_DIR].
pub(crate) const SPECS_DIR: &str = "specs";

/// What the parse of a fixture is expected to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Outcome {
    /// The fixture is a module of the language: the parse reports no diagnostics, and the
    /// tree holds neither a bogus node nor a child the grammar requires but the source
    /// does not have.
    Valid,

    /// The fixture holds a mistake: the snapshot describes what the parser made of it.
    Invalid,
}

/// Runs the fixture at `fixture`, a path relative to [SPECS_DIR], and checks it against its
/// snapshot.
pub(crate) fn run(fixture: &str) {
    let path = fixture_path(fixture);
    let source = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));

    let parsed = mlkc_parser::parse(&source);
    let outcome = outcome_of(fixture);

    assert_lossless(&parsed, &source, &path);
    assert_outcome(&parsed, outcome, &path);

    let snapshot = snapshot(&source, &parsed);
    let file_name = path
        .file_name()
        .expect("the fixture to have a name")
        .to_str()
        .expect("the name of the fixture to be UTF-8");
    let directory = Path::new(fixture)
        .parent()
        .expect("the fixture to be in a directory");

    insta::with_settings!({
        prepend_module_to_snapshot => false,
        snapshot_path => Path::new(SPECS_DIR).join(directory),
    }, {
        insta::assert_snapshot!(file_name, snapshot);
    });
}

/// The directory the fixtures live in.
fn fixtures_dir() -> PathBuf {
    Path::new(TESTS_DIR).join(SPECS_DIR)
}

/// The path of a fixture, given as a path relative to [SPECS_DIR].
pub(crate) fn fixture_path(fixture: &str) -> PathBuf {
    fixtures_dir().join(fixture)
}

/// The outcome the directory of the fixture promises.
fn outcome_of(fixture: &str) -> Outcome {
    let directory = Path::new(fixture)
        .components()
        .next()
        .expect("a fixture to be inside a directory")
        .as_os_str()
        .to_str()
        .expect("the directory of a fixture to be UTF-8");

    match directory {
        "valid" => Outcome::Valid,
        "invalid" => Outcome::Invalid,
        other => panic!("the fixture lives in `{other}`: move it to `valid` or `invalid`"),
    }
}

/// The tree of a parse always holds the whole source: nothing the parser walks over is
/// dropped, not even the tokens of a mistake.
fn assert_lossless(parsed: &AnyParse, source: &str, path: &Path) {
    let text = parsed.syntax::<MlkLanguage>().to_string();

    assert_eq!(
        text,
        source,
        "the tree of {} does not hold the source it was parsed from",
        path.display()
    );
}

/// Checks what the directory of the fixture promises about its parse.
fn assert_outcome(parsed: &AnyParse, outcome: Outcome, path: &Path) {
    if outcome == Outcome::Invalid {
        return;
    }

    assert!(
        parsed.diagnostics().is_empty(),
        "{} is a valid module but the parse reported {:?}",
        path.display(),
        parsed.diagnostics()
    );

    let root = parsed.syntax::<MlkLanguage>();
    assert!(
        !root.descendants().any(|node| node.kind().is_bogus()),
        "{} is a valid module but the tree holds a bogus node:\n{:#?}",
        path.display(),
        root
    );

    // A child the grammar requires and the source does not have is what a reader of the
    // tree sees as `missing (required)`, and `bogus node` is a child that cannot live
    // where it was written.
    let ast = format!("{:#?}", parsed.tree::<ModuleRoot>());

    for missing in ["missing (required)", "bogus node"] {
        assert!(
            !ast.contains(missing),
            "{} is a valid module but the tree holds `{missing}`:\n{ast}",
            path.display(),
        );
    }
}

/// The text of the snapshot of a parse.
fn snapshot(source: &str, parsed: &AnyParse) -> String {
    let mut snapshot = String::new();

    snapshot.push_str("## Input\n\n```mlk\n");
    snapshot.push_str(source);
    snapshot.push_str("\n```\n\n");

    snapshot.push_str("## AST\n\n```\n");
    snapshot.push_str(&ast(parsed));
    snapshot.push_str("\n```\n\n");

    snapshot.push_str("## CST\n\n```\n");
    write!(snapshot, "{:#?}", parsed.syntax::<MlkLanguage>())
        .expect("writing to a string to never fail");
    snapshot.push_str("\n```\n\n");

    snapshot.push_str("## Diagnostics\n\n");
    snapshot.push_str(&diagnostics(parsed));

    snapshot
}

/// The AST of a parse, as the typed facade of the tree reads it.
fn ast(parsed: &AnyParse) -> String {
    let root = parsed.syntax::<MlkLanguage>();

    // A mistake may break the module apart, and then there is no module root to cast the
    // tree to. The CST below is the one that shows what happened.
    if root.kind() != MODULE_ROOT {
        return format!("The root of the tree is {:?}.", root.kind());
    }

    format!("{:#?}", parsed.tree::<ModuleRoot>())
}

/// The diagnostics of a parse, one line each.
fn diagnostics(parsed: &AnyParse) -> String {
    let mut text = String::new();

    for diagnostic in parsed.diagnostics() {
        match diagnostic.range() {
            Some(range) => {
                writeln!(text, "{range:?}: {}", diagnostic.message)
                    .expect("writing to a string to never fail");
            },
            None => {
                writeln!(text, "no range: {}", diagnostic.message)
                    .expect("writing to a string to never fail");
            },
        }

        for advice in diagnostic.advices() {
            writeln!(
                text,
                "    {:?} {:?}: {}",
                advice.kind, advice.range, advice.message
            )
            .expect("writing to a string to never fail");
        }
    }

    if text.is_empty() {
        text.push_str("No diagnostics.\n");
    }

    text
}

/// The extension of a fixture. The snapshots live next to them and are not fixtures.
const FIXTURE_EXTENSION: &str = "mlk";

/// The fixtures the spec tests cover, as paths relative to [SPECS_DIR].
pub(crate) fn fixtures() -> Vec<PathBuf> {
    let mut fixtures = Vec::new();
    collect_fixtures(&fixtures_dir(), &mut fixtures);
    fixtures.sort();

    fixtures
}

fn collect_fixtures(directory: &Path, fixtures: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", directory.display()));

    for entry in entries {
        let path = entry.expect("the entry to be readable").path();

        if path.is_dir() {
            collect_fixtures(&path, fixtures);
        } else if path.extension() == Some(FIXTURE_EXTENSION.as_ref()) {
            fixtures.push(path);
        }
    }
}
