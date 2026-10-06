# Run coverage, mutation testing, and fuzzing as scheduled reports

- Status: accepted
- Date: 2026-10-06

## Context and Problem Statement

`just verify` is the gate every change passes:
it is fast enough to run on a laptop,
and it is deterministic,
because a red gate has to mean a bug in the change and nothing else.

Coverage, mutation testing, and fuzzing measure the suite rather than the change.
They are slow by orders of magnitude —
a coverage build instruments everything it compiles,
mutation testing runs the suite once per mutant,
and a fuzz target runs until it is told to stop —
and what they find is a weakness to read rather than a merge to block.

Where do they run, and what may they stop?

## Decision Drivers

- The gate stays fast and deterministic.
- The reports still run regularly, or they rot.
- A finding is recorded somewhere a person can read it.
- No account and no service are needed to read a report.

## Considered Options

- **No coverage, mutation testing, or fuzzing at all.**
- **In the gate**: thresholds and failing checks on every pull request.
- **Scheduled reports**, run by CI on a timer and by hand.
- **A hosted service** (Codecov and the like) with pull-request annotations.

## Decision Outcome

Chosen option: "Scheduled reports",
because the checks are worth having and not worth waiting for:
`just verify` stays the only gate,
and the reports run where slowness blocks nobody.

### What runs

- `just coverage` runs the suite under `cargo-llvm-cov`
  and writes HTML and an LCOV file under `target/coverage`;
  the vendored crates and the generated files are left out of the report.
- `just mutants [crate]` runs `cargo-mutants` on one crate,
  because one test run per mutant does not scale to the workspace.
- `just fuzz [target] [seconds]` runs a target of the `fuzz` workspace,
  which holds `parse` (losslessness, [ADR-0002](../adr/0002-lossless-syntax-tree.md))
  and `lower` (lowering does not panic, [ADR-0009](../adr/0009-pass-contract.md)).

### Where they run

- The `Reports` workflow of CI runs all three on a weekly schedule and by hand,
  and uploads each report as an artifact.
  It never runs on a pull request, and it is not a required check.
- `just lint` compiles the fuzz targets,
  so an API a target uses cannot drift away unnoticed between two reports.

### The fuzz workspace

`fuzz/` is a workspace of its own ([`fuzz/Cargo.toml`](../../fuzz/Cargo.toml)),
not a member of the root one:
`cargo fuzz` builds with sanitizer flags only a nightly rustc accepts,
and those flags would invalidate the build cache of every other command.
The dev shell pins the nightly toolchain and hands it to `just fuzz`
through `MLK_FUZZ_TOOLCHAIN`; without the shell, rustup's nightly is used.

The corpus a run builds is not committed.
The inputs worth keeping are:
the seeds under `fuzz/seeds`, which open a fresh corpus,
and the crash a report names, which is copied into the seeds
so that the next run starts where the last one found the bug.

### Positive Consequences

- The gate stays what it was: fast, deterministic, and about the change.
- The reports are read in the same place as everything else,
  with no service and no account.
- The fuzz targets compile in the gate, so they cannot rot silently.

### Negative Consequences

- A report runs once a week:
  a weakness introduced today is read on Monday.
- Nothing fails when the coverage or the mutation score drops;
  a person has to open the artifact and read it.
- The `fuzz` workspace carries its own lockfile and its own build cache.

## Links

- [ADR-0002](../adr/0002-lossless-syntax-tree.md): the losslessness the `parse` target asserts.
- [ADR-0006](../adr/0006-snapshot-testing.md): the suite the reports measure.
- [ADR-0009](../adr/0009-pass-contract.md): the totality the `lower` target asserts.
- [The map of the tests](../architecture/README.md).
