# Compatibility

Support current stable Rust and stable N−1. The declared minimum begins at 1.98. Update it when
needed in an explicit compatibility change, with compiler and downstream checks; do not silently
raise it in dependency maintenance. Default local compilation uses stable, without a pinned
`rust-toolchain.toml`.

This is a pre-1.0 library. Minor releases may change public APIs; patch releases preserve supported
behavior. Engine types remain private, while Ratatui rendering contracts are necessarily public.
Keep dependency requirements as wide as the implemented behavior honestly permits.

Input is UTF-8 and two-way unified/Git patch text. Binary payloads remain opaque. Escape sequences
are displayed visibly. Combined merges, unavailable context retrieval, streaming, and non-UTF-8
source rendering are deferred in the [roadmap](roadmap.md).
