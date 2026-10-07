# Agent Guidance

`ratatui-diff` is a reusable, backend-independent diff library. Host applications own terminal
events, filesystem access, repository operations, and review workflows.

Read [Rust conventions](docs/rust-conventions.md), [documentation](docs/documentation.md), and
[Rustdoc contracts](docs/rustdoc.md) when changing their respective surfaces. Use
[architecture](docs/architecture.md) for ownership, [testing](docs/testing.md) for evidence,
[compatibility](docs/compatibility.md) for dependency/MSRV changes, and
[releasing](docs/releasing.md) for publication. Deferred capabilities live in the
[roadmap](docs/roadmap.md).

Use jj, with a new described change for each independent review unit. Never create worktrees. Run
focused checks while iterating and `just check` before handoff. Fix warnings at their source. Use
nightly rustfmt and rumdl: prose wraps at 100 columns and tables remain aligned. Record reusable
maintainer feedback in its owning guide; keep this file as the map.
