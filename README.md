# MLK

MLK is a small statically typed language in the ML language family.
Its compiler is named as `mlkc` and is under active development.

The goal is to create a language with ergonomic typeclasses and higher-kinded types,
possibly with effects, and a _blazingly fast™_ compiler with great support for incremental compilation and IDE features.

Much of the work was inspired by Alexey _matklad_ Kladov's posts
on incremental compilation and compiler architecture[^against-queries].
You may find many interesting ideas and literature in the [ADR](docs/adr/README.md) documents,
which explain the design decisions behind the compiler.

[^against-queries]: Alexey Kladov, _Against Query Based Compilers,_ <https://matklad.github.io/2026/02/25/against-query-based-compilers.html>

The compiler compiles ahead of time to WebAssembly (support for Cranelift backend is planned).
It is built as a pipeline of pure passes —
lexing and parsing, lowering, resolution, type checking, MIR, the WASM LIR, codegen, linking —
driven by a memoizing driver that recompiles per module and per body.

Try it in the browser at [MLK playground](https://sicikh.github.io/mlk/).

## Repository layout

| Path                 | What is there                                                                                |
| -------------------- | -------------------------------------------------------------------------------------------- |
| `crates/`            | The compiler workspace: one crate per pipeline stage, plus the infrastructure and the hosts. |
| `xtask/`             | Code generation: the syntax tree and AST are generated from `xtask/codegen/mlk.ungram`.      |
| `library/std/`       | The standard library, written in MLK.                                                        |
| `web/`               | The editor: SvelteKit, CodeMirror 6, and the inspector panels.                               |
| `packages/wasm/`     | The `wasm-bindgen` package the editor loads, built from `mlkc-wasm`.                         |
| `docs/architecture/` | The map of the compiler: pipeline, crates, where to change what.                             |
| `docs/adr/`          | The architecture decision records, and their index.                                          |
| `AGENTS.md`          | How to work in this repository when you are a coding agent.                                  |

## Getting started

With [Nix](https://nixos.org/) and flakes enabled, the toolchain comes from `flake.nix`:

```sh
nix develop   # rust 1.99.0, the nightly rustfmt, node, pnpm, wasm-bindgen-cli, obscura
pnpm install  # once; pnpm comes from corepack
just verify
```

With `direnv`, `.envrc` enters the shell on `cd`.
`nix build .#mlkc` builds the compiler as a package.

Without Nix, install the prerequisites and the tools by hand:

- Rust (`rustup`; the version is pinned in `rust-toolchain.toml`), edition 2024.
- Node.js, the version in `.nvmrc`; pnpm through Corepack (`corepack enable`).
- [`just`](https://github.com/casey/just).

Then:

```sh
just install-tools   # cargo-insta, wasm-bindgen-cli, wasm-tools, wasmtime, pnpm dependencies
just test            # the workspace test suite
just lint            # clippy with warnings denied
just dev-web         # the editor on a dev server, with the wasm compiler built first
```

To poke at the compiler from the command line:

```sh
cargo run -p mlkc-cli -- parse library/std/core.mlk   # print the trees of one file
cargo run -p mlkc-cli -- run path/to/a/build          # run a build the editor produced
```

## Checking a change

| Command           | What it checks                                                                           |
| ----------------- | ---------------------------------------------------------------------------------------- |
| `just verify`     | Generated files, lint, tests, doc tests, and the editor's types. Works on a dirty tree.  |
| `just verify-web` | `verify` plus the editor in a real browser (needs `obscura` on the path).                |
| `just ready`      | The human's final gate: requires a clean tree and a browser, and regenerates everything. |

`just --list` shows every recipe.

## Documentation

- `docs/architecture/README.md` — the pipeline, the crates, and where to change what.
- `docs/architecture/glossary.md` — the vocabulary of the compiler.
- `docs/adr/README.md` — the decision index; the ADRs say why the code has its shape.
- `AGENTS.md` — the entry point for coding agents: commands, invariants, delegation.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT), at your option.
