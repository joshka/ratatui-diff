default:
    @just --list
check: fmt-check test clippy docs ci-check zizmor deny
fmt: fmt-rust fmt-md
fmt-check: fmt-rust-check fmt-md-check
fmt-rust:
    cargo +nightly fmt --all
fmt-rust-check:
    cargo +nightly fmt --all -- --check
fmt-md:
    rumdl fmt .
fmt-md-check:
    rumdl check .
test:
    cargo test --all-targets --locked
    cargo test --doc --locked
clippy:
    cargo clippy --all-targets --all-features --locked -- -D warnings
docs:
    RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features --locked
    RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --document-private-items --locked
docs-rs:
    RUSTDOCFLAGS="-D warnings --cfg docsrs" cargo +nightly doc --no-deps --all-features --locked
msrv:
    cargo +1.98.0 test --all-targets --locked
package:
    cargo package --locked
bench:
    cargo bench --bench viewer --locked
example:
    cargo run --example viewer --locked
ci-check:
    actionlint
zizmor:
    zizmor --no-progress --strict-collection --persona pedantic .github
deny:
    cargo deny --config .config/deny.toml --all-features --locked check
readme-check:
    cargo rdme --check
media:
    mkdir -p media
    cargo build --example viewer --locked
    python3 scripts/capture.py
# Generate the local visual guide without publishing media.
appearance:
    mkdir -p media
    cargo build --example viewer --locked
    python3 scripts/capture-appearance.py
    pandoc docs/appearance.md --standalone --embed-resources --resource-path=docs:. --css=scripts/appearance.css --lua-filter=scripts/color-swatches.lua --metadata pagetitle="Choosing a diff presentation" --output=media/appearance.html
