---
name: syntax-and-sugar
description: "Use when changing MLK's syntax, grammar, generated syntax nodes, or syntactic sugar: mlk.ungram and kinds, regeneration, the parser, lowering, and the editor's Lezer grammar (ADRs 0002, 0012, 0013, 0014)."
---

# Syntax and sugar

The surface of the language is defined in three places that must stay in step:
the generated syntax (from one ungrammar), the parser, and the meaning given by lowering.
The editor has its own grammar for highlighting.

## Files, in the order a change touches them

1. `xtask/codegen/mlk.ungram` — the grammar;
   `xtask/codegen/src/mlk_kinds_src.rs` — the tokens and node kinds.
   These are the sources of truth for the generated code.
2. `just gen-grammar` rewrites
   `crates/mlkc-syntax/src/generated/{kind,nodes,nodes_mut,macros}.rs` and
   `crates/mlkc-syntax-factory/src/generated/{syntax_factory,node_factory}.rs`.
   Never edit those by hand; `crates/mlkc-syntax/src/generated.rs` and
   `crates/mlkc-syntax-factory/src/generated.rs` are hand-written glue despite the name.
3. `crates/mlkc-parser/src/` — the parser (events, lexer, recovery).
   The infrastructure under `crates/mlkc-parser-core/` has its own `CONTRIBUTING.md`; read it
   before changing parsing rules, and load the `parser-development` skill for the working guide.
   Recovery uses bogus nodes, and the parser never fails
   (`docs/adr/0002-lossless-syntax-tree.md`).
4. `crates/mlkc-lower/src/` — the meaning of the tree: `decl.rs`, `item.rs`, `body.rs`, `path.rs`,
   `syntax.rs`, `source_map.rs`, `diagnostic.rs`.
   Lowering is the only stage that knows sugar (`docs/adr/0014-syntactic-sugar.md`), and it
   preserves the written form in the source map.
5. `web/src/lib/grammar/mlk.grammar` — the Lezer grammar for the editor's highlighting,
   generated into `mlk.ts` by `pnpm --filter @mlk/web build:grammar` (part of `just gen-grammar`).

## Rules of the surface

- `@`, `.`, and `::` are three different things, not one dot
  (`docs/adr/0012-split-the-dot-operator.md`).
- `x |> f(a, _)` is sugar for `let v = x in f(a, v)`
  (`docs/adr/0013-pipeline-operator.md`).
- The HIR gains no spelling variants: sugar stops at lowering
  (`docs/adr/0014-syntactic-sugar.md`).
- A new node needs a fixture that parses cleanly under `specs/valid` and, when it can be
  mistaken, one under `specs/invalid`.

## Workflow

1. Change the grammar and the kinds.
2. `just gen-grammar`; inspect the generated diff.
3. Teach the parser; add parser fixtures under `crates/mlkc-parser/tests/specs` and entries in
   `spec_tests.rs`.
4. Teach lowering; add fixtures under `crates/mlkc-lower/tests/specs/{valid,invalid}` and entries
   in `spec_tests.rs`. The suites fail on a fixture that has no test entry.
5. If the construct reaches later stages, follow it: type checking, MIR, and codegen each have
   their own snapshots.
6. Update the editor grammar when highlighting changes.
7. `just check-generated`, then `just verify`; commit the regenerated files.

Consumers of the AST are the generated wrappers in `mlkc-syntax` and the lowering.
Search for the generated node type name before assuming nothing else reads it.
