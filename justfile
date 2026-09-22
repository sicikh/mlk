_default:
    @just --list -u

# The version of the tool that packs the compiler for the browser. It has to be the version
# of the `wasm-bindgen` crate that `mlkc-wasm` links against — the tool refuses a module
# that was compiled against another version of itself — so the number is read from the
# dependencies of that crate rather than written down a second time: bump the crate in
# `Cargo.toml`, run `just install-tools`, and the tool follows it.
wasm_bindgen := trim_start_matches(`cargo tree -p mlkc-wasm -i wasm-bindgen --depth 0 --format "{p}"`, "wasm-bindgen v")

# Install the tools needed to develop
install-tools:
    #!/usr/bin/env sh
    set -eu
    # `cargo binstall` is the one thing that cannot come from itself, so it is here first;
    # where it is already around — a CI image, a second run — it is not built again.
    command -v cargo-binstall >/dev/null || cargo install cargo-binstall
    cargo binstall cargo-insta
    cargo binstall wasm-bindgen-cli --version "={{ wasm_bindgen }}"
    rustup target add wasm32-unknown-unknown
    pnpm install

# Upgrades the tools needed to develop
upgrade-tools:
    cargo install cargo-binstall --force
    cargo binstall cargo-insta --force

# Format Rust and TOML files
format:
    cargo +nightly fmt --all --verbose
    pnpm format

# Run clippy on the whole codebase
lint:
    cargo clippy --workspace --all-features --all-targets -- --deny warnings

# Run tests of all crates
test:
    cargo test --no-fail-fast

# Run tests for the crate passed as argument e.g. just test-crate mlkc-cli
test-crate name:
    cargo test -p {{ name }} --no-fail-fast

# Run doc tests
test-doc:
    cargo test --doc

# Build the wasm package the editor loads from `packages/wasm`
build-wasm:
    pnpm --filter @mlk/wasm build

# Build the site the editor is served as, the wasm included
build-web:
    pnpm --filter @mlk/web build

# Run the editor on a dev server, which builds the wasm before it starts
dev-web:
    pnpm --filter @mlk/web dev

# Check the types of the editor sources
check-web:
    pnpm --filter @mlk/web check

# Check the editor in a browser: the wasm boundary, and what a person would see.
# Needs `obscura` on the path, and builds the site it drives.
check-browser:
    pnpm --filter @mlk/web check:browser

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
    just lint
    just test
    just test-doc
    just check-web
    just check-browser
    git diff --exit-code --quiet
