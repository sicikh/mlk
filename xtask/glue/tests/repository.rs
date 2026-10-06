//! What the repository itself has to hold: the index of the ADRs agrees with the records
//! beside it, no snapshot was left half-accepted, the map names the crates and the passes
//! that are there, and no source globs the module it is written in.
//!
//! These are checks rather than unit tests of a function: what they read is the tree as it
//! stands, so a record added to `docs/adr` without a line in the index, a status that drifts
//! from the record it belongs to, or a `*.snap.new` left behind fails `just test`.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use xtask_glue::project_root;

/// The records of `docs/adr`, by their four-digit number, with the file each is written in.
fn records() -> BTreeMap<String, PathBuf> {
    let dir = project_root().join("docs/adr");
    let mut records = BTreeMap::new();

    for entry in fs::read_dir(&dir).expect("docs/adr is not readable") {
        let path = entry.expect("docs/adr is not readable").path();

        if path.extension().and_then(|it| it.to_str()) != Some("md") {
            continue;
        }

        let name = path
            .file_name()
            .and_then(|it| it.to_str())
            .unwrap_or_default();

        if let Some((number, _)) = name.split_once('-')
            && number.len() == 4
            && number.bytes().all(|it| it.is_ascii_digit())
        {
            records.insert(number.to_owned(), path);
        }
    }

    records
}

/// The targets of the markdown links of a text: what stands between `](` and the parenthesis
/// that closes the link, with the parentheses inside a target counted.
fn links_in(text: &str) -> Vec<String> {
    let mut links = Vec::new();
    let mut rest = text;

    while let Some(at) = rest.find("](") {
        let after = &rest[at + 2..];
        let mut depth = 0;
        let mut end = None;

        for (index, character) in after.char_indices() {
            match character {
                '(' => depth += 1,
                ')' if depth == 0 => {
                    end = Some(index);
                    break;
                },
                ')' => depth -= 1,
                _ => (),
            }
        }

        let Some(end) = end else { break };

        links.push(after[..end].to_owned());
        rest = &after[end + 1..];
    }

    links
}

/// The status a record gives itself, as the text after `- Status:`.
fn status_of(record: &str) -> Option<String> {
    record
        .lines()
        .find_map(|line| line.strip_prefix("- Status: "))
        .map(|it| it.trim().to_owned())
}

/// The status the index gives the record numbered `number`.
fn status_in_index(index: &str, number: &str) -> Option<String> {
    index
        .lines()
        .find(|line| line.starts_with(&format!("| [{number}](")))
        .and_then(|line| line.trim_end_matches('|').rsplit('|').next())
        .map(|cell| cell.trim().to_owned())
}

/// The first word of a status: `accepted; supersedes ...` is `accepted`.
fn word(status: &str) -> String {
    status
        .chars()
        .take_while(|it| it.is_ascii_alphabetic())
        .collect()
}

/// The paths of the `.rs` files under `dir`, with the directories of the tree walked.
fn rust_files(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap_or_else(|_| panic!("{} is not readable", dir.display())) {
        let path = entry.expect("the tree is readable").path();

        if path.is_dir() {
            if path.file_name().and_then(|it| it.to_str()) != Some("target") {
                rust_files(&path, files);
            }
        } else if path.extension().and_then(|it| it.to_str()) == Some("rs") {
            files.push(path);
        }
    }
}

/// The `use` statements of a source, as `(line, text)`: each begins at a line whose first word
/// is `use` --- a visibility in front of it is dropped, and so is the `///` or `//!` of a
/// doctest example --- and ends at its semicolon.
fn use_statements(source: &str) -> Vec<(usize, String)> {
    let mut statements = Vec::new();
    let mut current: Option<(usize, String)> = None;

    for (number, line) in source.lines().enumerate() {
        let trimmed = line.trim_start();
        let trimmed = trimmed
            .strip_prefix("///")
            .or_else(|| trimmed.strip_prefix("//!"))
            .map_or(trimmed, str::trim_start);

        if current.is_none() {
            let import = trimmed
                .strip_prefix("pub ")
                .or_else(|| trimmed.strip_prefix("pub(crate) "))
                .unwrap_or(trimmed);

            if import.starts_with("use ") {
                current = Some((number + 1, String::new()));
            }
        }

        let mut done = false;

        if let Some((_, statement)) = current.as_mut() {
            match trimmed.find(';') {
                Some(end) => {
                    statement.push_str(&trimmed[..end]);
                    done = true;
                },
                None => {
                    statement.push_str(trimmed);
                    statement.push(' ');
                },
            }
        }

        if done {
            statements.push(current.take().expect("a statement is being built"));
        }
    }

    statements
}

/// The index names every record, and every link it makes to a record is one that is there.
#[test]
fn the_index_names_every_record_and_no_other_file() {
    let dir = project_root().join("docs/adr");
    let index = fs::read_to_string(dir.join("README.md")).expect("the ADR index is not readable");
    let mut named = Vec::new();

    for link in links_in(&index) {
        if link.starts_with('#') || link.contains("://") {
            continue;
        }

        let target = link.split('#').next().unwrap_or(&link);

        assert!(
            dir.join(target).exists(),
            "the ADR index links to `{link}`, which is not in docs/adr",
        );
        named.push(target.to_owned());
    }

    for path in records().values() {
        let name = path
            .file_name()
            .and_then(|it| it.to_str())
            .unwrap_or_default();

        assert!(
            named.iter().any(|it| it == name),
            "the record `{name}` is not named by the ADR index",
        );
    }
}

/// A record and the index say the same thing about its status, and a status that supersedes
/// another names a record that is there.
#[test]
fn the_status_of_a_record_is_the_one_the_index_gives_it() {
    let dir = project_root().join("docs/adr");
    let index = fs::read_to_string(dir.join("README.md")).expect("the ADR index is not readable");
    let known = [
        "accepted",
        "proposed",
        "rejected",
        "deprecated",
        "superseded",
    ];

    for (number, path) in records() {
        let name = path
            .file_name()
            .and_then(|it| it.to_str())
            .unwrap_or_default();

        // The status of the template is the placeholder a new record replaces, and the index
        // says so with a word of its own.
        if number == "0000" {
            assert_eq!(
                status_in_index(&index, &number).as_deref(),
                Some("template"),
                "the index does not say that `{name}` is the template",
            );
            continue;
        }

        let record = fs::read_to_string(&path).expect("a record is not readable");
        let said = status_of(&record)
            .unwrap_or_else(|| panic!("the record `{name}` has no `- Status:` line"));
        let listed = status_in_index(&index, &number)
            .unwrap_or_else(|| panic!("the index gives no status for `{name}`"));

        assert_eq!(
            word(&said),
            word(&listed),
            "`{name}` says `{said}`, and the index says `{listed}`",
        );
        assert!(
            known.contains(&word(&said).as_str()),
            "`{name}` is `{said}`, which is not one of the statuses: {known:?}",
        );

        for link in links_in(&said).into_iter().chain(links_in(&listed)) {
            assert!(
                dir.join(&link).exists(),
                "the status of `{name}` names `{link}`, which is not in docs/adr",
            );
        }
    }
}

/// No snapshot is left half-accepted: insta writes `*.snap.new` beside the snapshot it could
/// not decide about, and what is in the tree is what `just test-review` has seen.
#[test]
fn no_snapshot_is_left_half_accepted() {
    let root = project_root();
    let output = Command::new("git")
        .arg("-C")
        .arg(&root)
        .args([
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "--",
            "*.snap.new",
        ])
        .output()
        .expect("git is what finds a snapshot left behind");

    assert!(
        output.status.success(),
        "`git ls-files` failed: {}",
        String::from_utf8_lossy(&output.stderr),
    );

    let strays = String::from_utf8_lossy(&output.stdout);

    assert!(
        strays.trim().is_empty(),
        "a snapshot was left half-accepted:\n{strays}",
    );
}

/// The identifiers a glob of `statement` whose `*` stands at `star` imports through:
/// the path before the `::` of `...::*`, or the path the enclosing group was opened on
/// for a bare `*` in a `{...}`.
fn glob_path(statement: &str, star: usize) -> Option<String> {
    let before = statement[..star].trim_end();

    if let Some(path) = before.strip_suffix("::").map(str::trim_end) {
        return last_word(path);
    }

    if !before.ends_with('{') && !before.ends_with(',') {
        return None;
    }

    let mut depth = 0usize;
    let mut open = None;

    for (index, character) in statement[..star].char_indices().rev() {
        match character {
            '}' => depth += 1,
            '{' if depth == 0 => {
                open = Some(index);
                break;
            },
            '{' => depth -= 1,
            _ => (),
        }
    }

    let path = statement[..open?].trim_end();
    last_word(path.strip_suffix("::").map_or(path, str::trim_end))
}

/// The identifier a path-like text ends with.
fn last_word(text: &str) -> Option<String> {
    let word: String = text
        .chars()
        .rev()
        .take_while(|it| it.is_ascii_alphanumeric() || *it == '_')
        .collect();
    let word: String = word.chars().rev().collect();

    if word.is_empty() { None } else { Some(word) }
}

/// A module reaches into the one it is written in by naming what it reads: a glob over `super`
/// or `crate` hides the dependency, and clippy does not lint the test modules of a build.
/// Every source of the workspace --- the tests included --- says what it imports.
#[test]
fn no_source_globs_the_module_it_is_written_in() {
    let root = project_root();
    let mut files = Vec::new();

    for tree in ["crates", "xtask"] {
        rust_files(&root.join(tree), &mut files);
    }

    let mut found = Vec::new();

    for path in files {
        let source = fs::read_to_string(&path).expect("a source file is readable");

        for (line, statement) in use_statements(&source) {
            let ancestor = statement
                .match_indices('*')
                .filter_map(|(star, _)| glob_path(&statement, star))
                .any(|word| word == "super" || word == "crate");

            if ancestor {
                let name = path.strip_prefix(&root).unwrap_or(&path);
                found.push(format!("{}:{line}: {statement}", name.display()));
            }
        }
    }

    assert!(
        found.is_empty(),
        "a module imports a glob over the one it is written in:\n{}",
        found.join("\n"),
    );
}

/// The workspace members of the root `Cargo.toml`: every directory `crates/*` and `xtask/*`
/// that carries a `Cargo.toml`, by the path the map links it by --- `crates/mlkc-parser`.
fn workspace_members() -> Vec<String> {
    let root = project_root();
    let mut members = Vec::new();

    for tree in ["crates", "xtask"] {
        let dir = root.join(tree);
        let entries =
            fs::read_dir(&dir).unwrap_or_else(|_| panic!("{} is not readable", dir.display()));

        for entry in entries {
            let path = entry.expect("the tree is readable").path();

            if path.join("Cargo.toml").is_file() {
                let name = path
                    .file_name()
                    .and_then(|it| it.to_str())
                    .unwrap_or_default();

                members.push(format!("{tree}/{name}"));
            }
        }
    }

    members.sort();
    members
}

/// The passes `Pass::name` gives, in the order the match writes them.
///
/// The list is read from the source rather than linked against `mlkc-driver`, so that the
/// check is a text of the repository and the test of the driver remains the one that keeps
/// `Pass` and `Pass::ALL` in step.
fn driver_passes() -> Vec<String> {
    let path = project_root().join("crates/mlkc-driver/src/driver/stats.rs");
    let source = fs::read_to_string(&path).expect("the passes of the driver are readable");
    let mut passes = Vec::new();

    for line in source.lines() {
        let Some(rest) = line.trim().strip_prefix("Pass::") else {
            continue;
        };
        let Some((_, arm)) = rest.split_once("=>") else {
            continue;
        };
        let name = arm
            .trim()
            .strip_prefix('"')
            .and_then(|it| it.split('"').next());

        if let Some(name) = name {
            passes.push(name.to_owned());
        }
    }

    passes
}

/// The names of the pass table of the map, in the order it writes them.
fn map_passes(map: &str) -> Vec<String> {
    let mut passes = Vec::new();
    let mut in_table = false;

    for line in map.lines() {
        let line = line.trim_start();

        if !in_table {
            if line.starts_with("| Pass") {
                in_table = true;
            }

            continue;
        }

        if !line.starts_with('|') {
            break;
        }

        let name = line
            .trim_start_matches('|')
            .split('|')
            .next()
            .map(str::trim)
            .and_then(|it| it.strip_prefix('`'))
            .and_then(|it| it.strip_suffix('`'));

        if let Some(name) = name {
            passes.push(name.to_owned());
        }
    }

    passes
}

/// The map names every crate of the workspace, and every crate it names is one: a crate
/// added to `crates/` or `xtask/` without a row in a table of the map fails here, and so
/// does a row that outlives its crate.
#[test]
fn the_map_and_the_workspace_name_the_same_crates() {
    let root = project_root();
    let map = fs::read_to_string(root.join("docs/architecture/README.md"))
        .expect("the map of the architecture is not readable");
    let mut named = Vec::new();

    for link in links_in(&map) {
        if link.starts_with('#') || link.contains("://") {
            continue;
        }

        let target = link
            .split('#')
            .next()
            .unwrap_or(&link)
            .trim_end_matches('/');
        let Some(member) = target.strip_prefix("../../") else {
            continue;
        };

        if !(member.starts_with("crates/") || member.starts_with("xtask/")) {
            continue;
        }

        assert!(
            root.join(member).join("Cargo.toml").is_file(),
            "the map links `{target}`, which is not a crate of the workspace",
        );
        named.push(member.to_owned());
    }

    for member in workspace_members() {
        assert!(
            named.iter().any(|it| it == &member),
            "the map does not name `{member}`, a member of the workspace; add it to a crate table",
        );
    }
}

/// The map lists the passes the driver runs, in the order it runs them: a pass added to
/// `Pass` without a row here --- or a row that outlives its pass --- fails the test.
#[test]
fn the_map_lists_the_passes_the_driver_runs() {
    let map = fs::read_to_string(project_root().join("docs/architecture/README.md"))
        .expect("the map of the architecture is not readable");

    assert_eq!(
        map_passes(&map),
        driver_passes(),
        "the pass table of the map and `Pass` have drifted apart",
    );
}
