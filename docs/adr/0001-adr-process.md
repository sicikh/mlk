# Use ADR to record architecture decisions

- Status: accepted
- Date: 2026-09-20

## Context and Problem Statement

`mlkc` is a compiler and a language server.
It is a long-lived project
whose architecture is shaped
by a small number of decisions
with project-wide consequences:

- the syntax tree representation,
- the IR design,
- the module system and incrementality model,
- the compiler pipeline,
- the testing strategy,
- the diagnostics model,
- etc.

These decisions are expensive to reverse,
and their rationale is not visible in the code itself.
Without a record,
future contributors (including ourselves in six months) will not know why a design was chosen
and which alternatives were rejected.

## Decision Drivers

- Preserve the rationale and context of significant decisions.
- Keep the process cheap enough
  that writing a record never feels like bureaucracy.
- Keep records consistent and unambiguous:
  one format, one location, one style.
- Keep text diffs small and review-friendly.
- Align with the conventions of the projects
  this infrastructure is modeled after (rust-analyzer, biome).

## Considered Options

- **No ADRs** — record decisions only in code comments
  and ad-hoc design docs.
- **Nygard-style ADRs** — the classic `Status / Context / Decision / Consequences` template.
- **MADR** — Markdown Any Decision Records,
  a fixed template with decision drivers, considered options,
  and pros/cons.

## Decision Outcome

Chosen option: "MADR",
because it forces every record to list the considered alternatives and the reasons for rejecting them,
which is the part of a decision record that ages best.

The process is defined as follows.

### What gets recorded

An ADR is written for every architecture-significant decision:

- decisions that are hard or expensive to reverse
  (e.g. the syntax tree representation, the IR design, the compilation model);
- decisions with long-term consequences for performance, incrementality, or diagnostics;
- explicit non-decisions that would otherwise keep resurfacing
  (e.g. "we do not allow glob imports", "we do not build a query-based compiler").

An ADR is not written for:

- reversible, local implementation details;
- bug fixes and routine refactorings;
- style and formatting trivia already covered by formatter configuration.

When in doubt, write an ADR: an unnecessary record is cheap, a missing one is expensive.

### When to write

The ADR is written at the moment the decision is made:
as part of the change that introduces it, or before starting significant work that depends on it.
An ADR may also record a decision that has no immediate code impact (like this one).

### File format and location

- One decision per file in `docs/adr/`.
- File name: `NNNN-short-title.md`, where `NNNN` is a sequential number, zero-padded to four digits.
- Numbers are never reused and never renumbered.
- Records follow the MADR template from [0000-template.md].

### Lifecycle

- Every record starts as `proposed`.
- After review it becomes `accepted` or `rejected`.
- An accepted ADR is immutable: it is never edited in substance (typo fixes are fine).
- If a decision changes, a new ADR is written and the old one is marked `superseded by [ADR-NNNN](NNNN-title.md)`.
- A decision that is no longer relevant is marked `deprecated`.

### Style

- Records are written in English.
- Text uses semantic linebreaks: one sentence (or one clause) per line.
  This keeps diffs minimal when a single sentence is edited and makes review comments precise.
- Prefer linking over duplicating: link to related ADRs and to the code that implements the decision.

### Positive Consequences

- The rationale of every significant decision survives in one discoverable place.
- Onboarding becomes easier: new contributors read the decision log instead of doing archaeology.
- The "considered options" section prevents re-litigating settled decisions without new information.
- Semantic linebreaks keep the records cheap to edit and review.

### Negative Consequences

- Maintaining the log requires discipline; it is easy to skip writing a record in the rush of implementation.
- Records can become outdated.
  This is mitigated by treating accepted ADRs as immutable and superseding them instead of silently editing.

## Links

- MADR: <https://adr.github.io/madr/>
- Template: [0000-template.md]

[0000-template.md]: 0000-template.md
