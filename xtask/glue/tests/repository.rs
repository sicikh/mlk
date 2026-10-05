//! What the repository itself has to hold: the index of the ADRs agrees with the records
//! beside it, and no snapshot was left half-accepted.
//!
//! These are checks rather than unit tests of a function: what they read is the tree as it
//! stands, so a record added to `docs/adr` without a line in the index, a status that drifts
//! from the record it belongs to, or a `*.snap.new` left behind fails `just test`.

use std::{collections::BTreeMap, fs, path::PathBuf, process::Command};

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
