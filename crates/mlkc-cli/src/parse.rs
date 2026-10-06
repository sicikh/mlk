//! Parsing a module: the trees it parses into, and what the pipeline reported.

use std::{fs, path::Path, time::Duration};

use anyhow::Context as _;
use mlkc_diagnostics::Diagnostic;
use mlkc_driver::{Driver, Parse, system_clock};
use mlkc_line_index::LineIndex;
use mlkc_span::TextRange;
use mlkc_vfs::{FileId, VfsPath};

/// Parses the file, prints what came of it, and tells whether it reported errors.
///
/// # Errors
///
/// Returns an error if the path cannot be made absolute or its bytes cannot be read,
/// if the file has no id in the driver once it was pushed,
/// or if the bytes of the file are not text.
pub fn parse(file: &Path) -> anyhow::Result<bool> {
    let path = std::path::absolute(file)
        .with_context(|| format!("failed to resolve {}", file.display()))?;

    // The host reads the bytes; whether they are text is the driver's business.
    let contents = fs::read(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let vfs_path = VfsPath::new_real_path(path.to_string_lossy().into_owned());

    let mut driver = Driver::new();

    // A pass is measured by the clock of the host, and this host has one: what a module costs
    // to read is read off it ([`Driver::set_clock`]).
    driver.set_clock(system_clock());
    driver.set_file_contents(vfs_path.clone(), Some(contents));

    // The standard library is part of the compiler, and it goes in beside the file a person
    // asked about: what a module of any project is read with is the library the compiler holds,
    // not whatever a directory of the file system happens to be.
    driver.use_std();

    let file = driver
        .file_id(&vfs_path)
        .context("the pushed file to have an id")?;

    let Some(parse) = driver.parse(file) else {
        anyhow::bail!(
            "{} is not text: {:?}",
            path.display(),
            driver.file_state(file)
        );
    };

    print_ast(&parse);
    print_cst(&parse);

    let failed = print_diagnostics(&mut driver, file, &vfs_path);

    print_metrics(&mut driver);

    Ok(failed)
}

/// Prints what the compiler did, and what each pass of it cost.
///
/// The driver counts its own work ([`Driver::take_stats`]): which passes ran, for which unit,
/// and --- since this host gave it a clock --- how long each of them took.
fn print_metrics(driver: &mut Driver) {
    let taken = driver.take_stats();

    if taken.is_empty() {
        return;
    }

    println!();
    println!("## Metrics");
    println!();

    let mut total = Duration::ZERO;

    for (pass, unit, tally) in taken.iter() {
        total += tally.took;

        println!(
            "{:<22} {:>7.2} ms  {:<32} {} hit(s), {} read again ({} kept), {} dropped",
            pass.name(),
            tally.took.as_secs_f64() * 1000.0,
            driver.unit_name(unit),
            tally.hits,
            tally.misses + tally.stales,
            tally.kept,
            tally.dropped,
        );
    }

    println!("{:<22} {:>7.2} ms", "total", total.as_secs_f64() * 1000.0);
}

/// Prints the typed view over the tree, which is the AST.
fn print_ast(parse: &Parse) {
    println!("## AST");
    println!();

    match parse.module_root() {
        Some(root) => println!("{root:#?}"),
        None => println!("The root of the tree is not a module."),
    }
}

/// Prints the concrete syntax tree, tokens and trivia included.
fn print_cst(parse: &Parse) {
    println!();
    println!("## CST");
    println!();
    println!("{:#?}", parse.syntax());
}

/// Prints the diagnostics, and tells whether one of them is an error.
fn print_diagnostics(driver: &mut Driver, file: FileId, path: &VfsPath) -> bool {
    let diagnostics = driver
        .diagnostics(file)
        .expect("the file to have been parsed");

    println!();
    println!("## Diagnostics");
    println!();

    if diagnostics.is_empty() {
        println!("No diagnostics.");
        return false;
    }

    let index = driver
        .line_index(file)
        .expect("the file to have been parsed");
    let text = driver
        .file_text(file)
        .expect("the file to have been parsed");

    for diagnostic in diagnostics.iter() {
        render(diagnostic, path, &text, &index);
    }

    diagnostics
        .iter()
        .any(|diagnostic| diagnostic.level.is_error())
}

/// Renders one diagnostic the way a person reads one:
/// where it is, the line it points at, and a caret under the place it means.
fn render(diagnostic: &Diagnostic, path: &VfsPath, text: &str, index: &LineIndex) {
    println!(
        "{}[{}{}]: {}",
        diagnostic.level.as_str(),
        diagnostic.category.as_code(),
        diagnostic.code,
        diagnostic.message
    );

    for label in &diagnostic.labels {
        let marker = if label.primary { '^' } else { '-' };
        let at = index.line_col(label.span.range.start());
        let number = at.line + 1;
        let column = at.col + 1;
        let gutter = number.to_string().len() + 1;

        if label.message.is_empty() {
            println!("{:gutter$}--> {path}:{number}:{column}", "");
        } else {
            println!(
                "{:gutter$}--> {path}:{number}:{column} {}",
                "", label.message
            );
        }

        // A label points at a byte; a caret is padded by what a reader sees before it,
        // which is why the piece of the line before the label is counted in characters.
        let Some(range) = index.line_range(at.line) else {
            continue;
        };
        let before = TextRange::new(range.start(), label.span.range.start().min(range.end()));
        let pad = " ".repeat(text[before].chars().count());

        println!("{:gutter$} |", "");
        println!("{number:>gutter$} | {}", &text[range]);
        println!("{:gutter$} | {pad}{marker}", "");
    }

    for note in &diagnostic.notes {
        println!("  = note: {note}");
    }
}
