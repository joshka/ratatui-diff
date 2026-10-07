# Rust Conventions

Optimize for reader locality, cohesive domain types, explicit side effects, and small meaningful
functions. Extract a concept when it reduces the facts readers must hold at once, not merely to move
code into another file. Keep algorithms visible in reading order.

Prefer clear ownership and borrowing. Keep comparison-engine types private. Document public
contracts at their owning API. Do not suppress warnings broadly. Measure hot paths before choosing
an abstraction for performance. Keep maintenance-only dependency updates separate from parsing,
MSRV, trait, and other consumer-visible changes.

Match `rustfmt.toml`, including nightly import grouping and comment formatting. Assign a multiline
receiver to a local variable before chaining methods. Use imperative jj summaries of at most 50
characters, with optional bodies wrapped at 72 columns.
