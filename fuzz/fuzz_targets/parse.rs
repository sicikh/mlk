//! The parser is lossless on arbitrary text ([ADR-0002]).
//!
//! The property test of `mlkc-parser` asserts the same thing over the texts `quickcheck`
//! draws; this target asserts it over the ones a coverage-guided fuzzer finds, which reach
//! deeper into the lexer and the parser than a random draw does.
//!
//! [ADR-0002]: ../../docs/adr/0002-lossless-syntax-tree.md

#![no_main]

use libfuzzer_sys::fuzz_target;
use mlkc_parser::parse;
use mlkc_syntax::MlkLanguage;

fuzz_target!(|data: &[u8]| {
    let text = match std::str::from_utf8(data) {
        Ok(text) => text,
        // The source of a module is text; what is not text is not the parser's input.
        Err(_) => return,
    };

    let printed = parse(text).syntax::<MlkLanguage>().to_string();

    assert_eq!(
        printed, text,
        "the tree of a parse is not the text it was parsed from",
    );
});
