---
name: parser-development
description: "Use when implementing or changing MLK parser behavior: parse rules, the Present/Absent presence contract, error recovery, lists, the lexer, the token source, inline specs, and parser fixtures. Do not use merely for consuming an existing AST/CST."
---

# Parser development

The canonical guide is `crates/mlkc-parser-core/CONTRIBUTING.md`.
It was adapted from Biome's parser guide and still uses Biome names in places:
read `biome_parser` as `mlkc-parser-core`, a `biome_*_syntax` crate as `mlkc-syntax`,
and a language parser as `mlkc-parser`.

The map is `docs/architecture/README.md`; the grammar-to-lowering workflow is the
`syntax-and-sugar` skill. This skill is about the parser itself.

## The pieces

| Piece                         | Where                                                                                                                                                     |
| ----------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| The grammar (source of truth) | `xtask/codegen/mlk.ungram`; kinds in `xtask/codegen/src/mlk_kinds_src.rs`                                                                                 |
| Generated syntax              | `crates/mlkc-syntax/src/generated/`, `crates/mlkc-syntax-factory/src/generated/`                                                                          |
| The infrastructure            | `crates/mlkc-parser-core/src/` (`Parser`, `ParserContext`, `marker`, `event`, `parse_lists`, `parse_recovery`, `parsed_syntax`, `token_set`, `tree_sink`) |
| The parser of MLK             | `crates/mlkc-parser/src/parser.rs` (the `MlkParser` wrapper)                                                                                              |
| The parse rules               | `crates/mlkc-parser/src/syntax/{module,expr,ty,pat,attribute,auxiliary,parse_error}.rs`                                                                   |
| The lexer                     | `crates/mlkc-parser/src/lexer/` — `logos` patterns                                                                                                        |
| The token source              | `crates/mlkc-parser/src/token_source.rs`                                                                                                                  |
| Tests                         | `crates/mlkc-parser/tests/{spec_tests.rs,spec_test.rs,cache.rs,specs/}`                                                                                   |

## Grammar conventions

- One language: nodes carry no language prefix. Names come from the concept: `FunDecl`,
  `PathSegment`, `LambdaExpr`.
- A union is named by the concept; `AnyX` is used where the plain name is taken by a variant:
  `AnyParameter = Parameter | BogusParameter`.
- `Bogus*` nodes keep broken code in the tree. They are declared at the top of the grammar and
  are a variant of the union they belong to (`ModuleItem = … | BogusDecl`,
  `Expr = … | BogusExpr`, and the same for `Type`, `Pat`, and `AnyParameter`). Tokens the
  grammar does not expect where a bogus node belongs still land in it as `SyntaxElement`s.
- A list node ends with `List` (`ArgumentList`, `ModuleItemList`) and is mandatory: the node is
  created even when the list is empty, so the children of its parent keep their slots.
- Fields are labeled (`name: Name`, `lhs: Expr`) on nodes the code matches on.
- After a grammar change run `just gen-grammar`; it regenerates the Rust syntax crates and the
  editor's Lezer grammar. Never edit the generated files.

## The presence contract

A rule is named `parse_*` after the node it parses, takes `&mut MlkParser`, and returns
`ParsedSyntax`.
`crates/mlkc-parser/src/syntax/mod.rs` states the contract:

- `Present(marker)` — the rule parsed the node, consumed at least one token, and reported every
  mistake it ran into;
- `Absent` — the source does not hold the node here; the rule must have consumed nothing and
  reported nothing, because only the caller knows whether the absence is an error.

So test the first distinguishing token (`p.at(…)`, `p.nth_at(…)`) before `p.start()` or any
consuming call, and never advance and then return `Absent`.

The conveniences, as in the core guide:

- `p.eat(token)` for an optional token; `p.expect(token)` for a required one (it reports and
  inserts a missing marker);
- `parsed.ok()` for an optional node; `parsed.or_add_diagnostic(p, expected_…)` for a required
  one (missing marker plus a diagnostic);
- `parsed.precede(p)` / `parsed.precede_or_add_diagnostic(…)` when the parsed node needs a
  parent.

A rule never builds a diagnostic inline: every message comes from a builder in
`crates/mlkc-parser/src/syntax/parse_error.rs`, so the same mistake reads the same everywhere.
Lookahead is bounded (`p.nth_at`); do not backtrack where a presence test the caller already has
can decide.

## Error recovery

A recovery set holds the tokens at which the construct ends and the next one begins; see
`MODULE_ITEM_RECOVERY_SET`, `EXPR_RECOVERY_SET`, and `STEP_RECOVERY_SET` for worked sets.
Recovery wraps the unexpected tokens in a grammar-valid `Bogus*` node:

```rust
let recovery = ParseRecoveryTokenSet::new(BOGUS_EXPR, EXPR_RECOVERY_SET);

match recovery.recover(p) {
    Ok(bogus) => p.error(expected_expr(bogus.range(p))),
    // The parser is at the end of the file, already at a recovery point,
    // or speculatively parsing.
    Err(_) => …,
}
```

Rules:

- recovery must preserve the syntax that follows: add a fixture where malformed input is
  followed by a construct that must still parse;
- derive the recovery set from where the caller stands, never copy it from a similar rule;
- every loop must advance or exit — that is why list parsing goes through `ParseNodeList` /
  `ParseSeparatedList` rather than an open-coded `while`;
- a list's recovery stops before the enclosing terminator, and the list node is kept.

## The lexer and the token source

- Lexing is context-free: `logos` patterns generate the DFA.
  `LexerTrait::next_token` returns every token, trivia included.
  A lexical error is `ERROR_TOKEN`, one character wide, so the parser can recover near the
  mistake; a mistake is described more precisely than the parse rule that trips over it, and
  the merge in `Parser::finish` keeps the lexer's diagnostic.
- The token source hides trivia: `current` is always a non-trivia token.
  Trivia on the same line trails the token before it; from the line break on it leads the next
  token. No rule ever looks at trivia.
- Re-lexing with a different context goes through `BumpWithContext`; do not add a second lexer
  path outside it.
- Checkpoints (`LexerCheckpoint`, `TokenSourceCheckpoint`) are cheap; use them instead of
  re-parsing from the start.

## Testing

Two mechanisms, both must stay green:

1. **Inline specs** — a comment in the rule's source holding a module that must parse cleanly:

    ```rust
    // test mlk a_module_declares_the_path_it_is_of
    // module project::main-module
    //
    // fun main(): Int =
    //     1
    ```

    The directive is `// test mlk <name>`; the test `inline_specs` in
    `crates/mlkc-parser/tests/spec_tests.rs` collects every spec from the parser sources and
    checks that the parse is clean and that the tree holds the source text.
    There is nothing to register: writing the spec is running it.
    Put it next to the rule it covers.

2. **Fixtures** — `crates/mlkc-parser/tests/specs/{valid,invalid}`, one module per `.mlk` file,
   each with a snapshot and a line in `spec_tests.rs` (the suite fails on a fixture that has no
   line). `valid` must be diagnostics-free and hold no bogus or missing node; `invalid` records
   where the parser got to after a mistake.

- Accept a changed snapshot with `INSTA_UPDATE=always cargo test -p mlkc-parser`, review the
  diff, and run `just test-crate mlkc-parser`; the mechanics are in the `snapshot-tests` skill.
- `crates/mlkc-parser/tests/cache.rs` covers parsing through a `NodeCache`: the tree of a
  revision must be the tree of its text whatever the cache holds, and parts two revisions
  wrote the same way must be one allocation. Run it when touching the tree sink or the cache
  path.
- A parser bug fix needs the smallest fixture that failed before the change; a recovery change
  needs malformed input followed by valid syntax.

## Review checklist

- [ ] every `Absent` path consumes nothing and reports nothing;
- [ ] required tokens and nodes produce a diagnostic from `parse_error.rs`;
- [ ] recovery emits a `Bogus*` node the grammar permits at that position;
- [ ] every loop advances or exits, and list recovery stops at the enclosing boundary;
- [ ] the tree is lossless: it holds every byte of the source, malformed input included;
- [ ] valid and invalid paths both have a spec or a fixture;
- [ ] grammar changes include the regenerated files (`just check-generated`).
