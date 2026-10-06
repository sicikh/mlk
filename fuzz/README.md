# Fuzzing

The targets under `fuzz_targets/` run the parser and the lowering on arbitrary bytes:
`parse` asserts that the tree of a parse prints the text it was parsed from ([ADR-0002]),
and `lower` asserts that lowering what a parse built does not panic ([ADR-0009]).

    just fuzz parse 60     # 60 seconds; `lower` for the other target

The crate is a workspace of its own, because `cargo fuzz` builds it with sanitizer flags
only a nightly rustc accepts; `just fuzz` finds the nightly toolchain the dev shell provides.
The targets are reports, never gates ([ADR-0027]): CI runs them on a schedule, and
`just lint` compiles them but nothing in `just verify` runs them.

`fuzz/corpus/` is where a run keeps the inputs it generates, and it is not committed.
The inputs worth keeping are: `fuzz/seeds/`, which `just fuzz` copies into the corpus of a
target that has none yet, and the crash a report names, which is read from `fuzz/artifacts/`
and copied into the seeds so that the next run starts where the last one found the bug.

[ADR-0002]: ../docs/adr/0002-lossless-syntax-tree.md
[ADR-0009]: ../docs/adr/0009-pass-contract.md
[ADR-0027]: ../docs/adr/0027-scheduled-reports.md
