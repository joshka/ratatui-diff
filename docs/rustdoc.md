# Rustdoc Contracts

The crate root teaches the lifecycle: prepare a document, borrow a widget, retain viewport state,
and let the host own events and I/O. Modules explain ownership. Types document invariants,
construction, relationships, and lifecycle. Methods state inputs, effects, errors, and limitations.

Preserve non-obvious source/display mappings, cache invalidation rules, line-ending behavior,
comparison limits, and ordering constraints near their owning API or implementation. Establish
rationale from evidence; do not infer historical intent from current code alone.

Use runnable examples for construction, fallible input, state transitions, and integration. Group
related accessors rather than repeating trivial examples. Document enum variants and public fields.
Link canonical contracts without linking every repeated type mention. Keep README, crate Rustdoc,
and examples aligned. State the pre-1.0 compatibility policy explicitly.

Apply the prose review in [Documentation](documentation.md) to Rustdoc and comments too. Run
`just docs`, `just docs-rs`, doctests, and README synchronization. Private documentation builds
check internal links but do not establish that every explanation is accurate.
