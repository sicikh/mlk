_default:
    @just --list -u

# The version of the tool that packs the compiler for the browser. It has to be the version
# of the `wasm-bindgen` crate that `mlkc-wasm` links against — the tool refuses a module
# that was compiled against another version of itself — so the number is read from the
# dependencies of that crate rather than written down a second time: bump the crate in
# `Cargo.toml`, run `just install-tools`, and the tool follows it.
wasm_bindgen := trim_start_matches(`cargo tree -p mlkc-wasm -i wasm-bindgen --depth 0 --format "{p}"`, "wasm-bindgen v")

# Install the tools needed to develop.
# Inside the Nix dev shell the tools come from the shell, and this installs only the pnpm
# dependencies; see `flake.nix`.
install-tools:
    #!/usr/bin/env sh
    set -eu
    if [ -n "${IN_NIX_SHELL:-}" ]; then
        echo "the Nix dev shell provides the Rust tools; installing the pnpm dependencies"
        pnpm install
        exit 0
    fi
    # `cargo binstall` is the one thing that cannot come from itself, so it is here first;
    # where it is already around — a CI image, a second run — it is not built again.
    command -v cargo-binstall >/dev/null || cargo install cargo-binstall
    cargo binstall cargo-insta
    cargo binstall cargo-deny
    cargo binstall cargo-nextest
    cargo binstall wasm-bindgen-cli --version "={{ wasm_bindgen }}"
    cargo binstall wasm-tools
    cargo binstall wasmtime-cli
    rustup target add wasm32-unknown-unknown
    pnpm install

# Upgrades the tools needed to develop
upgrade-tools:
    cargo install cargo-binstall --force
    cargo binstall cargo-insta --force
    cargo binstall cargo-deny --force
    cargo binstall cargo-nextest --force
    cargo binstall wasm-bindgen-cli --force
    cargo binstall wasm-tools --force
    cargo binstall wasmtime-cli --force

# Check that the tools the repository uses are on the path; run it before blaming the build
doctor:
    #!/usr/bin/env sh
    set -u
    missing=""
    for tool in cargo rustc just node pnpm cargo-nextest; do
        if command -v "$tool" >/dev/null 2>&1; then
            printf 'ok       %s\n' "$tool"
        else
            printf 'MISSING  %s\n' "$tool"
            missing="$missing $tool"
        fi
    done
    for tool in cargo-insta wasm-bindgen wasm-tools wasmtime obscura cargo-deny; do
        if command -v "$tool" >/dev/null 2>&1; then
            printf 'ok       %s\n' "$tool"
        else
            printf 'optional %s is missing; the checks that use it cannot run\n' "$tool"
        fi
    done
    # The search and browsing tools of the dev shell; a setup without Nix may not have them.
    for tool in rg fd jq tokei nixfmt nil; do
        if command -v "$tool" >/dev/null 2>&1; then
            printf 'ok       %s\n' "$tool"
        else
            printf 'optional %s is missing; the Nix dev shell provides it\n' "$tool"
        fi
    done
    if [ -n "$missing" ]; then
        printf '\nthe required tools above are missing: install the prerequisites of README.md, then run `just install-tools`\n' >&2
        exit 1
    fi

# Format the Rust, TOML, and Nix files.
# The Rust formatting needs the nightly rustfmt: `RUSTUP_TOOLCHAIN` selects it where rustup is
# installed, and `RUSTFMT` does in the Nix dev shell.
format:
    RUSTUP_TOOLCHAIN=nightly cargo fmt --all --verbose
    pnpm format
    nixfmt flake.nix

# Run clippy on the whole codebase
lint:
    cargo clippy --workspace --all-features --all-targets -- --deny warnings

# Build the documentation of the workspace, refusing every warning of rustdoc
doc:
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps

# Check every dependency of the lockfile: its license, its advisories, and the bans the workspace keeps
# The advisories are read from the RustSec database, which is fetched on the first run.
deny:
    cargo deny check

# Run tests of all crates
test:
    cargo nextest run

# Run tests for the crate passed as argument e.g. just test-crate mlkc-cli
test-crate name:
    cargo nextest run -p {{ name }}

# Run doc tests
test-doc:
    cargo test --doc

# Review the pending snapshot changes interactively, and accept the ones you mean
test-review:
    cargo insta test --review

# Build the wasm package the editor loads from `packages/wasm`
build-wasm:
    pnpm --filter @mlk/wasm build

# Build the site the editor is served as, the wasm included
build-web:
    pnpm --filter @mlk/web build

# Run the editor on a dev server, which builds the wasm before it starts
dev-web:
    pnpm --filter @mlk/web dev

# Lint the editor sources and check their types
check-web:
    pnpm --filter @mlk/web check

# Run the tests of the pure editor modules: values in and values out, with no browser
test-web:
    pnpm --filter @mlk/web test

# Check the editor in a browser: the wasm boundary, and what a person would see.
# Needs `obscura` on the path, and builds the site it drives.
check-browser:
    pnpm --filter @mlk/web check:browser

# Say whether generated files are up to date, without writing them; works on a dirty tree
check-generated:
    #!/usr/bin/env sh
    set -eu
    cargo run -p xtask-codegen -- check
    # lezer-generator writes the output as `<path>.ts`, so the temporary name ends in `.ts` too.
    tmp="$(mktemp).ts"
    trap 'rm -f "$tmp"' EXIT
    pnpm --filter @mlk/web exec lezer-generator --noTerms --typeScript src/lib/grammar/mlk.grammar -o "$tmp"
    if ! cmp -s web/src/lib/grammar/mlk.ts "$tmp"; then
        echo 'web/src/lib/grammar/mlk.ts is not what lezer-generator makes of src/lib/grammar/mlk.grammar' >&2
        echo 'run `just gen-grammar` and commit the result' >&2
        exit 1
    fi

# Check the tree the way CI does, without a browser; unlike `just ready`, works on a dirty tree
verify: check-generated lint doc test test-doc test-web check-web

# `just verify` plus the editor in a real browser; needs `obscura`, and builds the site
verify-web: verify check-browser

# Generates the code of the grammars
gen-grammar:
    cargo run -p xtask-codegen -- grammar
    pnpm --filter @mlk/web build:grammar

# Generate all files across crates and tools, the parser of the editor included.
# That parser is the one file here that `lezer-generator` writes, not the xtask.
# You rarely want to use it locally.
gen-all:
    cargo run -p xtask-codegen -- all
    pnpm --filter @mlk/web build:grammar
    just format

# When you finished coding, run this command to run the same commands in the CI.
ready:
    git diff --exit-code --quiet
    just gen-all
    #just format # format is already run in `just gen-all`
    just verify
    just check-browser
    git diff --exit-code --quiet
