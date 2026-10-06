//! The CLI host of the driver.
//!
//! A host owns what the driver must not: the file system, the process, the exit code. This one
//! reads the bytes of a file and pushes them into the [`Driver`], and prints what a person
//! reads: the trees of a parse and its diagnostics ([`parse`]), or what a build printed when
//! it ran ([`run`]).
//!
//! [`Driver`]: mlkc_driver::Driver

// The CLI is a host: printing what a person asked for is what it is for.
#![expect(
    clippy::print_stdout,
    reason = "the CLI host prints the trees, the diagnostics, and what a build wrote"
)]

pub mod parse;
pub mod run;

mod archive;
