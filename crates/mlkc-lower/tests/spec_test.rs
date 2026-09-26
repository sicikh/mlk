//! The harness of the spec tests.
//!
//! A spec is a fixture under [`SPECS_DIR`], and a snapshot next to it that holds what the
//! lowering made of the module: the source, the item tree, the bodies, and the diagnostics.
//! The directory of a fixture tells what the module is expected to be:
//!
//! - the snapshots under `specs/valid` are modules the language accepts: the parse reports
//!   nothing, the lowering reports nothing, and the HIR holds no missing node;
//! - the ones under `specs/invalid` hold a mistake, and the snapshot is what the lowering
//!   read around it. A mistake itself is what the parser reports, and the parser's own spec
//!   tests are where its diagnostics are read: the ones here are the summary of a parse that
//!   the HIR below was lowered from.
//!
//! A snapshot is part of changing the lowering: `INSTA_UPDATE=always cargo test -p mlkc-lower`
//! rewrites them, and the diff of the snapshots is what the review reads.
//!
//! The fixtures are lowered with the standard prelude, which is what a module of a project
//! that declares no prelude of its own is compiled with: a fixture that uses `Int` without
//! importing it shows what the prelude is worth, and one that declares the name itself shows
//! that the module's own text is what its names denote ([ADR-0011]).
//!
//! [ADR-0011]: ../../docs/adr/0011-module-prelude.md

use std::{
    fmt::Write,
    fs,
    path::{Path, PathBuf},
};

use mlkc_hir_def::{ItemLoc, ModuleId, Prelude};
use mlkc_lower::{LoweredBody, LoweredModule, lower_body, lower_module, syntax_at};
use mlkc_parser_core::AnyParse;
use mlkc_rowan::AstNode;
use mlkc_syntax::{FunDecl, ModuleRoot};
use mlkc_vfs::FileId;

/// The directory of the tests of this crate.
///
/// The fixtures are read from here rather than from the working directory of the test, so
/// that a test does not depend on where it was started from.
const TESTS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests");

/// The directory of the fixtures, relative to [`TESTS_DIR`].
///
/// A snapshot is written next to the fixture it describes, and the path of a snapshot that
/// insta resolves is relative to the file that asserts it --- here, `tests/spec_test.rs`.
/// The path handed to insta is therefore spelled relative to the directory of the tests,
/// while the fixtures themselves are read from [`TESTS_DIR`].
pub(crate) const SPECS_DIR: &str = "specs";

/// The file a fixture is lowered as.
///
/// One module per fixture: what is lowered of a module is what the module's own text says,
/// and the file it is read from is what the names inside it are names in.
const FIXTURE_FILE: FileId = FileId::from_raw(0);

/// What the lowering of a fixture is expected to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Outcome {
    /// The fixture is a module of the language: the parse and the lowering report nothing,
    /// and the HIR holds no missing node.
    Valid,

    /// The fixture holds a mistake: the snapshot describes what the lowering made of it.
    Invalid,
}

/// Runs the fixture at `fixture`, a path relative to [`SPECS_DIR`], and checks it against its
/// snapshot.
pub(crate) fn run(fixture: &str) {
    let path = fixture_path(fixture);
    let source = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));

    let parsed = mlkc_parser::parse(&source);
    let root = parsed.tree::<ModuleRoot>();

    let lowered = lower_module(ModuleId(FIXTURE_FILE), &root, Prelude::standard());
    let bodies = bodies(&lowered);

    assert_outcome(&parsed, &lowered, &bodies, outcome_of(fixture), &path);
    assert_positions(&root, &lowered);

    let snapshot = snapshot(&source, &parsed, &lowered, &bodies);
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

/// The bodies of a module, lowered one at a time, from the declarations they are written in.
fn bodies(lowered: &LoweredModule) -> Vec<LoweredBody> {
    lowered
        .bodies
        .iter()
        .map(|decl| {
            lower_body(&lowered.item_tree, &decl.decl)
                .expect("a declaration of the work list to declare a body")
        })
        .collect()
}

/// The declaration the item tree points at is the one the work list holds.
///
/// The walk back is what a caller that holds an item tree and the syntax it was lowered from
/// does: the position an entity is recorded at is where its declaration is, and a body is
/// lowered from there.
fn assert_positions(root: &ModuleRoot, lowered: &LoweredModule) {
    for decl in &lowered.bodies {
        let loc = ItemLoc::from(decl.owner.item().clone());
        let position = lowered
            .item_tree
            .syntax_loc(loc)
            .expect("an entity of the item tree to be recorded at a position");
        let syntax = syntax_at(root, position).expect("a declaration to be where it was recorded");
        let declaration = FunDecl::cast(syntax).expect("the declaration of a function");

        assert_eq!(
            declaration.syntax().text_trimmed().to_string(),
            decl.decl.syntax().text_trimmed().to_string(),
            "the declaration of {:?} is not the one it was lowered from",
            decl.owner.item(),
        );
    }
}

/// The directory the fixtures live in.
fn fixtures_dir() -> PathBuf {
    Path::new(TESTS_DIR).join(SPECS_DIR)
}

/// The path of a fixture, given as a path relative to [`SPECS_DIR`].
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

/// Checks what the directory of the fixture promises about the lowering.
fn assert_outcome(
    parsed: &AnyParse,
    lowered: &LoweredModule,
    bodies: &[LoweredBody],
    outcome: Outcome,
    path: &Path,
) {
    if outcome == Outcome::Invalid {
        return;
    }

    let mut problems = Vec::new();

    if !parsed.diagnostics().is_empty() {
        problems.push(format!("the parse reported {:?}", parsed.diagnostics()));
    }

    for diagnostic in diagnostics(lowered, bodies) {
        problems.push(format!("the lowering reported {diagnostic}"));
    }

    for (what, dump) in dumps(lowered, bodies) {
        if dump.contains("missing") {
            problems.push(format!("the {what} holds a missing node:\n{dump}"));
        }
    }

    assert!(
        problems.is_empty(),
        "{} is a module the language accepts, but:\n- {}",
        path.display(),
        problems.join("\n- ")
    );
}

/// The diagnostics of lowering a module: the ones of the item tree, then the ones of a body.
fn diagnostics(lowered: &LoweredModule, bodies: &[LoweredBody]) -> Vec<String> {
    lowered
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.to_string())
        .chain(
            bodies
                .iter()
                .flat_map(|body| body.diagnostics.iter())
                .map(|diagnostic| diagnostic.to_string()),
        )
        .collect()
}

/// The item tree and the bodies of a module, as text.
fn dumps(lowered: &LoweredModule, bodies: &[LoweredBody]) -> Vec<(String, String)> {
    let mut dumps = vec![(
        "item tree".to_owned(),
        mlkc_hir_def::dump::item_tree(&lowered.item_tree),
    )];

    for (decl, body) in lowered.bodies.iter().zip(bodies) {
        dumps.push((
            format!("body of {:?}", decl.owner.item()),
            mlkc_hir_def::dump::body(&decl.owner, &body.body),
        ));
    }

    dumps
}

/// The text of the snapshot of a lowering.
fn snapshot(
    source: &str,
    parsed: &AnyParse,
    lowered: &LoweredModule,
    bodies: &[LoweredBody],
) -> String {
    let mut snapshot = String::new();

    snapshot.push_str("## Input\n\n```mlk\n");
    snapshot.push_str(source);
    snapshot.push_str("\n```\n\n");

    snapshot.push_str("## Item Tree\n\n```\n");
    snapshot.push_str(&mlkc_hir_def::dump::item_tree(&lowered.item_tree));
    snapshot.push_str("```\n\n");

    snapshot.push_str("## Bodies\n\n");
    if lowered.bodies.is_empty() {
        snapshot.push_str("No bodies.\n\n");
    } else {
        for (decl, body) in lowered.bodies.iter().zip(bodies) {
            snapshot.push_str("```\n");
            snapshot.push_str(&mlkc_hir_def::dump::body(&decl.owner, &body.body));
            snapshot.push_str("```\n\n");
        }
    }

    snapshot.push_str("## Lowering Diagnostics\n\n");
    write_diagnostics(
        &mut snapshot,
        diagnostics(lowered, bodies).into_iter(),
        "No diagnostics.",
    );

    snapshot.push_str("## Parse Diagnostics\n\n");
    write_diagnostics(
        &mut snapshot,
        parsed.diagnostics().iter().map(diagnostic_line),
        "No diagnostics.",
    );

    snapshot
}

/// The summary of a parse diagnostic, one line.
///
/// The detail of what a parse reported --- what it expected, and where else it looked --- is
/// what the parser's own snapshots hold; what lowering reads of a parse is where it broke.
fn diagnostic_line(diagnostic: &mlkc_parser_core::diagnostic::ParseDiagnostic) -> String {
    match diagnostic.range() {
        Some(range) => format!("{range:?}: {}", diagnostic.message),
        None => format!("no range: {}", diagnostic.message),
    }
}

/// The lines of a section of diagnostics, or a word for a section that holds none.
fn write_diagnostics(text: &mut String, lines: impl Iterator<Item = String>, empty: &str) {
    let mut any = false;

    for line in lines {
        writeln!(text, "{line}").expect("writing to a string to never fail");
        any = true;
    }

    if !any {
        text.push_str(empty);
        text.push('\n');
    }

    text.push('\n');
}

/// The extension of a fixture. The snapshots live next to them and are not fixtures.
const FIXTURE_EXTENSION: &str = "mlk";

/// The fixtures the spec tests cover, as paths relative to [`SPECS_DIR`].
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
