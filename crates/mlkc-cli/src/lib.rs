//! The CLI host of the driver.
//!
//! A host owns what the driver must not: the file system, the process, the exit code. This one
//! reads the bytes of a file and pushes them into the [`Driver`], and prints what a person
//! reads: the trees of a parse and its diagnostics ([`parse`]), or what a build printed when
//! it ran ([`run`]).
//!
//! [`Driver`]: mlkc_driver::Driver

pub mod parse;
pub mod run;

mod archive;
