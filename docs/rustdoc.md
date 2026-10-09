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

## Usage Prose and Examples

Open with what callers can display, then teach the document, widget, and state lifecycle beside the
first rendering example. Keep defaults, input limits, navigation units, and replacement behavior
near the relevant task. Separate compatibility policy from performance advice.

Explain the supported rendering contract without debating unimplemented APIs. For example:

- Before: "Rendering through `Widget for &mut Diff` is a valid alternative, but is not currently
  implemented."
- After: "Build `Diff` in the draw closure and retain `DiffState` between frames."

Keep the tradeoff between rendering traits in the architecture guide. Include implementation detail
in usage prose when it changes a caller's decision: synchronous preparation and whole-document first
layout matter to event-loop responsiveness; the internal row representation usually does not.

Describe errors precisely. "Reports input offsets" leaves readers guessing whether offsets are a
successful result or error context. Say that parse errors carry byte offsets when available, and
leave the full error contract with the error type. Do not strengthen optional diagnostics into a
guarantee while shortening the sentence.

Examples should demonstrate a useful operation and compile with the documented imports. Hide routine
doctest result plumbing, retain meaningful assertions, and avoid comments that translate each call
into English. Explain sample-only setup briefly; link to an application example for terminal
initialization, restoration, and event handling.

Edit crate Rustdoc in `src/lib.rs`, regenerate README with `cargo rdme`, and run
`cargo rdme --check`. Preserve the README content outside the generated markers, including the
screenshot gallery and release asset URLs, unless that content is part of the requested edit. Review
both outputs: successful generation proves synchronization, not prose quality.
