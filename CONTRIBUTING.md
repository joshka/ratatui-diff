# Contributing

Install stable Rust, nightly rustfmt, just, rumdl, cargo-deny, zizmor, actionlint, and typos.
`cargo install cargo-rdme --locked` supplies the README synchronization check. The declared minimum
compiler is Rust 1.98; local default compilation uses stable.

Run `just --list` for commands. `just fmt` formats Rust and Markdown. `just test`, `just clippy`,
and `just docs` check behavior and API documentation. `just check` adds workflow and dependency
checks. `just msrv` tests the minimum compiler; `just package` verifies the published archive. Run
`just example` to explore the viewer, `just bench` to measure layout and rendering, and `just media`
to capture the deterministic example with Betamax.

Follow [Rust conventions](docs/rust-conventions.md), [documentation](docs/documentation.md),
[Rustdoc contracts](docs/rustdoc.md), and [testing](docs/testing.md). Keep independently reviewable
changes small. Explain behavior, validation, and remaining limitations without requiring readers to
know the conversation. See the [roadmap](docs/roadmap.md) before expanding scope.
