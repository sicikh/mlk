---
name: doc-comments
description: "Use when writing or editing Rust `//`, `///`, or `//!` comments in the MLK repository: crate docs, item docs, inline rationale, and the deletion test for what does not belong. Do not use for ADRs or the architecture docs, which follow their own process."
---

# Doc comments

Developer-facing comments and docs in this repository are read by contributors months or years
later, with none of the context you have now.
This skill defines who that reader is, what each kind of comment is for, and which patterns are
banned.

## The reader

Write for an MLK contributor who is competent in Rust but has **no access to your context**:
not this conversation, not the pull request, not the issue, not the diff.
They see only the repository at HEAD.

Two consequences follow directly:

1. **Never narrate change history.**
   "now", "previously", "no longer", "the new approach" are meaningless at HEAD, where only one
   approach exists.
   State how the code works, not how it came to be.
2. **Never address the reviewer.**
   A comment that argues your change is correct ("this properly handles X") belongs in the PR
   description.
   The comment must justify the code as it stands, permanently.

## Three kinds of documentation, three jobs

| Kind                        | Job         | Contains                                                                                                            |
| --------------------------- | ----------- | ------------------------------------------------------------------------------------------------------------------- |
| `//!` module and crate docs | Explanation | Why the module exists, the concepts and terms it defines, how the pieces relate, design rationale                   |
| `///` item docs             | Reference   | The contract: behavior, inputs and outputs, invariants, panics, errors. Neutral and factual                         |
| `//` inline comments        | Rationale   | Only what the code cannot say: constraints, workarounds, non-obvious coupling, why the obvious alternative is wrong |

Do not mix the jobs.
Implementation details do not belong in `///` docs — put them as `//` comments inside the body.
The contract does not belong scattered across inline comments — put it on the item.

## House conventions

- **`//!` docs open every crate**, saying what it is and where it sits in the pipeline.
  If you add a crate, write them first; `docs/architecture/README.md` says what belongs there.
  A crate whose guide is long can include it as its docs, as `mlkc-parser-core` does:
  `#![doc = include_str!("../CONTRIBUTING.md")]`.
- **Semantic linebreaks.** A doc comment breaks lines at sentence and clause boundaries, not at
  a column: one sentence or clause per line.
  The ADRs and the existing crate docs are written this way, and it keeps later diffs small.
- **Link ADRs by reference definition.** Put the definition at the bottom of the doc block, as
  the file around you does — `[ADR-0017]: ../../docs/adr/0017-resolved-types.md` — and write
  `[ADR-0017]` in the text.
  Some files use `[adr-0019]`; follow the file you are in.
- **Use the glossary vocabulary.** Driver, pass, input function, host, slot, unit, lowering,
  resolution — these words have fixed meanings in `docs/architecture/glossary.md`; use them as
  the glossary defines them.
- **Backticks for code**: identifiers, types, tokens, and paths.
  Link Rust items with intra-doc links where a reader benefits: ``[`Driver`]``,
  ``[`FileVersion`]``. Note that `mlkc-parser-core` denies broken intra-doc links.
- **A doc test is a promise.** Code blocks in `///` docs are compiled and run by
  `just test-doc` (and rustfmt formats them: `format_code_in_doc_comments`).
  Keep examples minimal and true, or mark them `ignore`/`text` when they are not Rust.
- **Vendored crates keep their origin's conventions** (`mlkc-rowan`, `mlkc-text-size`,
  `mlkc-text-edit`, `mlkc-string-case`, `mlkc-ungrammar`); do not re-style their comments.

## The deletion test

Before writing any comment, ask: **does this state something the reader cannot recover from the
code itself?**

- If names, types, or structure already carry it, do not write the comment.
  If the name fails to carry it, improve the name.
- Worth a comment: an invariant, a rationale, a coupling to code elsewhere, a workaround with a
  link, surprising behavior of a dependency, a term the module defines.
- When editing later, the same test applies in reverse: a comment that no longer passes it
  should be deleted, not left to rot.

## Behavior documentation

Write for a human reader, not as a translation of the implementation.

- Start with a plain-language description of what the function returns or accomplishes.
- One main idea per sentence; keep sentences short or medium.
- Avoid internal jargon; explain a necessary technical term in the same paragraph.
- Name the caveats that can surprise callers: fallback behavior, work limits, ambiguous
  results, ordering, conditions that return `None` or an error.
- Do not describe implementation details unless a caller needs them to understand behavior.

Add an example when the behavior depends on relationships the signature cannot show:
a path resolved through a re-export, an inferred type that depends on context, a value whose
meaning is not obvious from its type.
Introduce the example and state the expected result; keep snippets minimal.

Module docs describe a durable concept or design reason.
Do not list the functions of the file to summarize it — such lists go stale.
If there is no durable concept, use one line.

The best examples in this repository are the crate docs of `crates/mlkc-driver/src/lib.rs`
(the rules the driver follows, stated in the present tense, with no history) and
`crates/mlkc-vfs/src/lib.rs` (what the VFS owns, and why a version is immutable — a concept the
API alone would not show).

## Banned patterns

**Narrating the next line.** Delete these on sight:

```rust
// Increment the generation counter
generation += 1;
```

**Change-history narration.** Rewrite as present-tense rationale:

```rust
// BAD: We now intern types instead of cloning them.
// GOOD: Interning avoids cloning these types on every lookup.
```

**Reviewer-addressed justification.** Move the argument to the PR:

```rust
// BAD: This correctly fixes the trivia ordering from the bug report.
// GOOD: Trivia on the same line trails the token before it, so the line break decides which
//       token the rest leads.
```

**Restated rustdoc.** A `///` doc that rewords the item name says nothing:

```rust
// BAD:
/// Checks the body.
fn check_body(...)

// GOOD:
/// Checks one body against the signatures of its module, and reports what does not type-check.
fn check_body(...)
```

**Vague hedging.** "Some cases", "various reasons", "handles edge cases", "etc." — either name
them or drop the sentence.

**Ad-hoc section banners** (`// ----- helpers -----`, `// ==== TYPES ====`).
Organize with files, modules, and `impl` blocks instead.
The `// #region` markers appear only in the vendored `mlkc-rowan`; new MLK code does not use
them, and files are kept short enough not to need them.

## Editing existing code

- Preserve existing doc comments.
  If your change alters behavior, extend or correct the specific prose — never replace it with
  generic text.
  Deleting hard-won context is worse than leaving a comment slightly stale.
- Match the surrounding density.
  A heavily documented module deserves the same level on new items; do not blanket a sparse
  module with comments.
- Follow the file's own dash and footnote style rather than imposing your own.

## Self-check before finishing

After a task that touched comments, re-read **only the comments in your diff**, in isolation
from the code changes:

1. Does each one pass the deletion test?
2. Does any reference the conversation, the change itself, or the reviewer?
3. Would a reader without the diff understand each one?
4. Does every new crate or module have `//!` docs that say why it exists?

Fix or delete what fails.
Deletion is the default; a missing comment is cheaper than a misleading one.

## References

- [Diátaxis](https://diataxis.fr/) — the framework behind the explanation / reference /
  rationale split.
- `docs/architecture/glossary.md` — the vocabulary to write in.
- `docs/adr/0001-adr-process.md` — the ADR style (MADR, semantic linebreaks), which applies to
  `docs/adr/`, not to Rust comments.
