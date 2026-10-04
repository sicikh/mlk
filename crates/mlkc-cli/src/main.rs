//! The CLI of the compiler: what a person runs.

use std::{path::PathBuf, process::ExitCode};

use bpaf::Bpaf;

#[derive(Debug, Clone, Bpaf)]
#[bpaf(options, version)]
/// The host of the compiler: a parse a person reads, or a build a person runs.
enum Command {
    /// Parses a module and prints the trees it parses into.
    ///
    /// The exit code is the one of a check: a parse that reported errors exits non-zero.
    #[bpaf(command)]
    Parse {
        /// The path of the module to parse.
        #[bpaf(positional("FILE"))]
        file: PathBuf,
    },

    /// Runs a build: the modules and the manifest of a project.
    #[bpaf(command)]
    Run {
        /// Debug the run in a native debugger: the engine keeps DWARF, and does not optimize.
        #[bpaf(short('g'), long("debug"))]
        debug: bool,

        /// The build: the directory it is written in, or the archive it came in.
        #[bpaf(positional("BUILD"))]
        build: PathBuf,
    },
}

fn main() -> ExitCode {
    match command().run() {
        Command::Parse { file } => {
            match mlkc_cli::parse::parse(&file) {
                Ok(true) => ExitCode::FAILURE,
                Ok(false) => ExitCode::SUCCESS,
                Err(error) => fail(error),
            }
        },
        Command::Run { build, debug } => {
            match mlkc_cli::run::run(&build, debug) {
                Ok(printed) => {
                    for line in printed {
                        println!("{line}");
                    }

                    ExitCode::SUCCESS
                },
                Err(error) => fail(error),
            }
        },
    }
}

/// Says what went wrong, and hands back the exit code of a failure.
fn fail(error: anyhow::Error) -> ExitCode {
    eprintln!("error: {error:#}");

    ExitCode::FAILURE
}
