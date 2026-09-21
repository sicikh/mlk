//! The harness of the spec tests.
//!
//! A spec is a test that is written where the rule it covers lives:
//!
//! - next to the rule, as a comment holding the source of a module that parses cleanly,
//!   see [inline_specs];
//! - under `specs`, as a fixture whose parse is compared with the snapshot written next to
//!   it. A snapshot holds what a person reads when a rule changes: the source, the AST that
//!   a reader of the tree sees, the tree the parser produced, and the diagnostics. The
//!   snapshot is part of changing a rule: `INSTA_UPDATE=always cargo test -p mlkc-parser`
//!   rewrites them, and the diff of the snapshots is what the review reads.
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

    let problems = problems(parsed);

    assert!(
        problems.is_empty(),
        "{} is a valid module but its parse is not clean:\n- {}",
        path.display(),
        problems.join("\n- ")
    );
}

/// What is wrong with a parse that the language accepts.
///
/// The parser is the one that knows the grammar, so a module that the language accepts
/// must parse into a tree that holds every child the grammar requires, and holds nothing
/// the parser made up to keep parsing.
fn problems(parsed: &AnyParse) -> Vec<String> {
    let mut problems = Vec::new();

    if !parsed.diagnostics().is_empty() {
        problems.push(format!("the parse reported {:?}", parsed.diagnostics()));
    }

    let root = parsed.syntax::<MlkLanguage>();

    if let Some(bogus) = root.descendants().find(|node| node.kind().is_bogus()) {
        problems.push(format!(
            "the tree holds a bogus node, the parser made it up from a mistake:\n{bogus:#?}"
        ));
    }

    // A child the grammar requires and the source does not have is what a reader of the
    // tree sees as `missing (required)`, and `bogus node` is a child that cannot live
    // where it was written.
    match module_root(parsed) {
        Some(ast) => {
            for marker in ["missing (required)", "bogus node"] {
                if ast.contains(marker) {
                    problems.push(format!("the AST holds `{marker}`:\n{ast}"));
                }
            }
        },
        None => {
            problems.push(format!(
                "the root of the tree is {:?}, which is not the root of a module",
                root.kind()
            ))
        },
    }

    problems
}

/// The AST of a parse, as the typed facade of the tree reads it.
fn module_root(parsed: &AnyParse) -> Option<String> {
    // A mistake may break the module apart, and then there is no module root to cast the
    // tree to. The CST of the snapshot is the one that shows what happened.
    if parsed.syntax::<MlkLanguage>().kind() != MODULE_ROOT {
        return None;
    }

    Some(format!("{:#?}", parsed.tree::<ModuleRoot>()))
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
    module_root(parsed).unwrap_or_else(|| {
        format!(
            "The root of the tree is {:?}.",
            parsed.syntax::<MlkLanguage>().kind()
        )
    })
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

/// The directive that starts a spec written next to a rule: `// test mlk <name>`.
///
/// The comment lines that follow the directive hold the source of the spec, up to the first
/// line that is not a comment. The source is a whole module, so a spec reads as the file a
/// person would write to see the rule at work.
const SPEC_DIRECTIVE: &str = "// test mlk";

/// A test written as a comment next to the rule it covers.
pub(crate) struct InlineSpec {
    /// The file the spec is written in, relative to the root of the crate.
    pub(crate) file: String,
    /// The line the directive is on, counted from one.
    pub(crate) line: usize,
    /// The name the directive gives the spec.
    pub(crate) name: String,
    /// The module the spec holds.
    pub(crate) source: String,
}

/// Runs the specs written next to the rules, and reports every one that broke.
///
/// The sources are read when the test runs, so a spec that is written is a spec that runs:
/// there is nothing to register, and no list to keep in step with the rules.
pub(crate) fn run_inline_specs() {
    let specs = inline_specs();

    assert!(
        !specs.is_empty(),
        "no spec was found in the sources of the parser: a spec is a `{SPEC_DIRECTIVE} <name>` \
         comment that holds a module"
    );

    let mut failures = Vec::new();

    for spec in &specs {
        let parsed = mlkc_parser::parse(&spec.source);

        let mut problems = problems(&parsed);

        if parsed.syntax::<MlkLanguage>().to_string() != spec.source {
            problems.push("the tree does not hold the source it was parsed from".to_string());
        }

        if !problems.is_empty() {
            failures.push(format!(
                "{}:{} `{}`:\n  {}",
                spec.file,
                spec.line,
                spec.name,
                problems.join("\n  ")
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "{} of the {} specs written next to the rules failed:\n\n{}",
        failures.len(),
        specs.len(),
        failures.join("\n\n")
    );
}

/// The specs written in the sources of the parser, in the order they are written in.
pub(crate) fn inline_specs() -> Vec<InlineSpec> {
    let mut sources = Vec::new();
    collect_sources(&src_dir(), &mut sources);
    sources.sort();

    let mut specs = Vec::new();

    for path in sources {
        let text = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        let file = path
            .strip_prefix(crate_dir())
            .unwrap_or(path.as_path())
            .display()
            .to_string();

        specs.extend(specs_of(&file, &text));
    }

    specs
}

/// The specs of a single source file.
fn specs_of(file: &str, text: &str) -> Vec<InlineSpec> {
    let mut specs = Vec::new();
    let mut lines = text.lines().enumerate().peekable();

    while let Some((index, line)) = lines.next() {
        let Some(name) = spec_directive(line) else {
            continue;
        };

        let mut source = Vec::new();

        while let Some((_, next)) = lines.peek() {
            // A directive ends the spec before it, and anything that is not a comment ends
            // the spec as well: the module is written in comments, and the code of the
            // parser is not one.
            if spec_directive(next).is_some() {
                break;
            }

            let Some(text) = spec_comment(next) else {
                break;
            };

            source.push(text);
            lines.next();
        }

        specs.push(InlineSpec {
            file: file.to_string(),
            line: index + 1,
            name: name.to_string(),
            source: module_text(&source),
        });
    }

    specs
}

/// The name a directive line gives a spec, if the line is a directive.
fn spec_directive(line: &str) -> Option<&str> {
    let name = line.trim_start().strip_prefix(SPEC_DIRECTIVE)?.trim();

    (!name.is_empty()).then_some(name)
}

/// The source a line of a spec holds, if the line belongs to a spec.
///
/// A doc comment documents the code of the parser instead of holding a test, so it ends the
/// spec rather than joining it.
fn spec_comment(line: &str) -> Option<&str> {
    let comment = line.trim_start().strip_prefix("//")?;

    if comment.starts_with('/') || comment.starts_with('!') {
        return None;
    }

    Some(comment.strip_prefix(' ').unwrap_or(comment))
}

/// The module a spec holds, without the empty lines that only keep the comment readable.
fn module_text(lines: &[&str]) -> String {
    let empty = |line: &&str| line.trim().is_empty();
    let first = lines.iter().position(|line| !empty(line)).unwrap_or(0);
    let last = lines
        .iter()
        .rposition(|line| !empty(line))
        .map_or(first, |last| last + 1);

    let mut text = lines[first..last].join("\n");

    if !text.is_empty() {
        text.push('\n');
    }

    text
}

/// The directory of the sources of the parser.
fn src_dir() -> PathBuf {
    Path::new(crate_dir()).join("src")
}

/// The directory of the crate, which is the root of the paths of the sources.
fn crate_dir() -> &'static str {
    env!("CARGO_MANIFEST_DIR")
}

fn collect_sources(directory: &Path, sources: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", directory.display()));

    for entry in entries {
        let path = entry.expect("the entry to be readable").path();

        if path.is_dir() {
            collect_sources(&path, sources);
        } else if path.extension() == Some(SOURCE_EXTENSION.as_ref()) {
            sources.push(path);
        }
    }
}

/// The extension of a source of the parser. The specs are written in these files.
const SOURCE_EXTENSION: &str = "rs";
