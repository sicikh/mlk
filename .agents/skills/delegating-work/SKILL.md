---
name: delegating-work
description: "Use when a task in this repository is large enough to split across subagents: when to split, decomposition recipes with write scopes, the brief to give each subagent, and how to integrate and verify the pieces."
---

# Delegating work

One agent with full context is the right shape for most tasks.
Split only when the pieces are independent in both meaning and files: a split that makes two
agents edit the same file is not a split, it is a merge conflict.

## When to split

Split when at least two of these hold:

- the pieces touch disjoint crates or directories;
- each piece is at least a small task, not a one-file edit;
- the interface between the pieces is fixed before the work starts
  (a type, a grammar, a wasm API, a file format);
- the pieces can be verified separately.

Do not split "write the code" from "write its tests": the interface moves while the tests are
written, and one agent should own both.
Do not delegate a task you could finish in a few tool calls.

## Freeze the interface first

The common failure is two agents discovering different interfaces.
Before fanning out, land the interface in one change, or assign it to one agent and have the
others wait for it:

- a language feature: the grammar, the generated nodes, and the HIR types first;
- an editor feature: the wasm method names and their shapes first;
- a refactor: the new type and its module first.

## What each subagent gets

A self-contained brief with five parts:

1. **Goal** — one sentence, in terms of the repository, not of the editor.
2. **Write scope** — the exact paths it may write; everything else is read-only.
3. **Context** — the ADRs to read, the architecture map section, and the skills that apply
   (`pipeline-pass`, `syntax-and-sugar`, `snapshot-tests`, `backend-wasm`, `web-editor`).
4. **Commands** — how to verify its piece (`just test-crate X`, `just check-web`,
   `INSTA_UPDATE=always …`, and so on).
5. **Done** — what must be true: which tests pass, which snapshots changed and were reviewed,
   and what it must report.

A subagent does not see the orchestrator's conversation: put the context in the brief.

## Shared files have one writer

Assign exactly one writer for each of:
`Cargo.toml`, `Cargo.lock`, `justfile`, `AGENTS.md`, `docs/architecture/*`,
`docs/adr/README.md`, `.github/workflows/ci.yml`, `web/package.json`, `pnpm-lock.yaml`,
`xtask/codegen/mlk.ungram`, and every generated file
(`crates/mlkc-syntax/src/generated/*`, `crates/mlkc-syntax-factory/src/generated/*`,
`web/src/lib/grammar/mlk.ts`).
Two agents running `just gen-all` at once will clobber each other's generated files.
Snapshots belong to a suite: give one agent all of a suite, or expect conflicts.

## Common splits

| Task                                     | Pieces                                                                                                                                                                 |
| ---------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A language feature, end to end           | (1) grammar + generated nodes + parser + lowering, (2) type checking, (3) MIR + codegen, (4) editor highlighting + docs. Land (1) first; (2)–(4) depend on its shapes. |
| Two unrelated features                   | one agent per feature, per crate.                                                                                                                                      |
| A backend change with a front-end change | split only if the HIR types do not move.                                                                                                                               |
| Docs and ADR hygiene                     | one agent; the shared mapping files have one writer anyway.                                                                                                            |
| Test hardening of one area               | one agent per suite.                                                                                                                                                   |

## Integration

- Merge one piece at a time; run `just verify` after each merge.
- When the editor or the wasm boundary is in play, run `just verify-web` on the final tree.
- Resolve snapshots in the merge, not in the branches: a snapshot taken before the other piece
  landed is a guess.
- Report what ran and what could not run (`obscura` missing, for instance); never claim a check
  that did not run.
