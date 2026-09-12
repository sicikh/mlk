_default:
    @just --list -u

# Install the tools needed to develop
install-tools:
    cargo install cargo-binstall
    cargo binstall cargo-insta
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
    cargo test run --no-fail-fast

# Run tests for the crate passed as argument e.g. just test-crate mlkc-cli
test-crate name:
    cargo test run -p {{ name }} --no-fail-fast

# Run doc tests
test-doc:
    cargo test --doc

# Generates the code of the grammars
gen-grammar:
    cargo run -p xtask-codegen -- grammar

# Generate all files across crates and tools. You rarely want to use it locally.
gen-all:
    cargo run -p xtask-codegen -- all
    just format

# When you finished coding, run this command to run the same commands in the CI.
ready:
    git diff --exit-code --quiet
    just gen-all
    just documentation
    #just format # format is already run in `just gen-all`
    just lint
    just test
    just test-doc
    git diff --exit-code --quiet
