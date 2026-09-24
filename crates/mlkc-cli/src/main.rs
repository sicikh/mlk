//! The CLI host of the driver.
//!
//! A host owns what the driver must not: the file system, the process, the exit code.
//! This one reads the bytes of a file, pushes them into the [`Driver`], pulls the parse,
//! and prints what a person reads:
//! the typed view of the tree, the tree itself, and the diagnostics.

use std::{fs, path::PathBuf, process::ExitCode};

use anyhow::Context as _;
use bpaf::Bpaf;
use mlkc_diagnostics::Diagnostic;
use mlkc_driver::{Driver, Parse};
use mlkc_line_index::LineIndex;
use mlkc_span::TextRange;
use mlkc_vfs::{FileId, VfsPath};

#[derive(Debug, Clone, Bpaf)]
#[bpaf(options, version)]
/// Parses a module and prints the trees it parses into.
///
/// The exit code is the one of a check: a parse that reported errors exits non-zero.
struct Options {
    /// The path of the module to parse.
    #[bpaf(positional("FILE"))]
    file: PathBuf,
}

fn main() -> ExitCode {
    let options = options().run();

    match run(&options) {
        Ok(true) => ExitCode::FAILURE,
        Ok(false) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        },
    }
}

/// Parses the file, prints what came of it, and tells whether it reported errors.
fn run(options: &Options) -> anyhow::Result<bool> {
    let path = std::path::absolute(&options.file)
        .with_context(|| format!("failed to resolve {}", options.file.display()))?;

    // The host reads the bytes; whether they are text is the driver's business.
    let contents = fs::read(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let vfs_path = VfsPath::new_real_path(path.to_string_lossy().into_owned());

    let mut driver = Driver::new();
    driver.set_file_contents(vfs_path.clone(), Some(contents));

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

    Ok(print_diagnostics(&mut driver, file, &vfs_path))
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
