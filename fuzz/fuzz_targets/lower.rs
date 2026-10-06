//! Lowering arbitrary text does not panic ([ADR-0009]).
//!
//! The target mirrors what the spec harness of `mlkc-lower` does with a fixture: parse the
//! text, lower the module with the standard prelude, and lower every body of it. Unlike a
//! fixture, the text is whatever the fuzzer wrote, so the lowering is under test on the
//! trees a parse of a mistake builds as much as on the ones it accepts.
//!
//! [ADR-0009]: ../../docs/adr/0009-pass-contract.md

#![no_main]

use libfuzzer_sys::fuzz_target;
use mlkc_hir_def::{ModuleId, Prelude, ProjectId};
use mlkc_lower::{lower_body, lower_module};
use mlkc_parser::parse;
use mlkc_syntax::ModuleRoot;
use mlkc_vfs::{FileId, RelPathBuf};

fuzz_target!(|data: &[u8]| {
    let text = match std::str::from_utf8(data) {
        Ok(text) => text,
        // The source of a module is text; what is not text is not the parser's input.
        Err(_) => return,
    };

    let parsed = parse(text);
    let root = parsed.tree::<ModuleRoot>();
    let relative = RelPathBuf::try_from("main.mlk").expect("a file name is a relative path");

    let lowered = lower_module(
        ModuleId(FileId::from_raw(0)),
        &root,
        Prelude::standard(),
        &[ProjectId::new("std")],
        relative.as_path(),
    );

    for decl in &lowered.bodies {
        let _ = lower_body(&lowered.item_tree, &decl.decl);
    }
});
