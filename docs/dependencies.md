# Diff Dependency Decision

Choose Similar for comparison and Diffy for parsing. Keep both behind private adapters so callers
can supply structured documents and dependency changes do not dictate the public model.

Research on October 7, 2026 inspected published sources, registry owners, releases, commits, issues,
and consumer manifests. Similar is maintained by Armin Ronacher and used by Insta, Delta, and
Snapbox. Diffy was created by Brandon Williams and has substantial contributions from Weihang Lo.
Both have 2026 releases. Downloads indicate reach, including repeated CI downloads, not independent
users or guarantees of reliability. Maintenance remains concentrated.

Similar 3.2 supplies line and word comparison with Unicode support; enable std, text, inline, and
unicode. Diffy 0.5 supplies multi-file Git parsing; disable default features, color, and binary
decompression. The runtime graph adds unicode-segmentation and hashbrown rather than Git machinery.

Imara-diff is used by Helix, but its latest published 0.2 release is from June 2025, and a formatter
fix remains the subject of a release request. Gitoxide maintains a modified fork. Dissimilar is a
credible dependency-free text engine, but lacks the desired line/hunk surface. Unidiff is a viable
metadata parser. Older diff/difference APIs offer less functionality for this project.

Diffy has a pending CRLF extended-header correction. Normalize header delimiters only, preserving
payload line endings. Characterize parser gaps locally rather than assuming development-branch
behavior is available in a published release. Revisit the engine if measured pathological workloads,
maintenance delays, required semantics, or dependency growth outweigh this integration.

Sources: [Similar](https://github.com/mitsuhiko/similar),
[Insta](https://github.com/mitsuhiko/insta/blob/master/insta/Cargo.toml),
[Snapbox](https://github.com/assert-rs/snapbox/blob/main/crates/snapbox/Cargo.toml),
[Diffy](https://github.com/bmwill/diffy/blob/master/CHANGELOG.md),
[CRLF correction](https://github.com/bmwill/diffy/pull/89),
[imara release request](https://github.com/pascalkuthe/imara-diff/issues/48), and
[Gitoxide fork](https://github.com/gitoxidelabs/gitoxide/tree/main/gix-imara-diff).
