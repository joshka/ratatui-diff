# Deferred Work

The rendering contract remains `StatefulWidget` with separate `DiffState`; the mutable-widget
prototype is skipped. Establish visual and interaction evidence for the existing layouts before
adding further interaction features or syntax coloring. See [architecture](architecture.md) for
ownership and [appearance captures](appearance-captures.md) for visual checks.

Literal search, source hit-testing, selection, mouse dragging, and source-text extraction are
implemented. Clipboard access remains with the host. See the
[coordinate](interaction-coordinates.md) and [selection](selection.md) contracts.

Retained context can be folded within supplied hunks. Generated diffs opt into full retention with
`DiffDocument::compare(old, new, usize::MAX)`; the default comparison still retains three context
lines. Folding preserves searchable and extractable source. Missing patch text and context discarded
during comparison require a new document supplied by the host. Identical generated inputs continue
to have no hunks. See [architecture](architecture.md) for ownership and missing-data boundaries.

These capabilities have no release commitment. Record implementation evidence and compatibility
impact when selecting one; remove it from this list only when its documented contract is fulfilled.

| Capability                                           | Why deferred                                                   | Revisit when                                                       |
| ---------------------------------------------------- | -------------------------------------------------------------- | ------------------------------------------------------------------ |
| Syntax coloring                                      | Adds language data, dependencies, and style composition        | A consumer needs language-aware coloring and accepts the footprint |
| Structural comparison and engine adapters            | Needs richer semantics than text patches                       | A consumer supplies a concrete engine integration                  |
| Regex and advanced search                            | Adds query policy and dependency choices                       | A host needs more than case-sensitive literal source search        |
| Retrieval of unavailable context                     | Requires source access and document replacement                | A host supplies a concrete filesystem or repository adapter        |
| Moved-line detection and improved alignment          | Adds heuristics and potentially expensive comparisons          | Measured examples show source-order pairing is inadequate          |
| Combined, three-way, conflict views and editing      | Changes the two-sided data model                               | A concrete merge-review workflow establishes requirements          |
| Non-UTF-8 sources                                    | Requires byte-oriented rendering contracts                     | A consumer needs faithful arbitrary-byte display                   |
| Streaming and million-line inputs                    | Changes ownership, indexes, and incremental state              | Large in-memory benchmarks establish a limit                       |
| Standalone pager, Git/jj integration, external tools | Product/I/O scope belongs outside the widget                   | A separate application or adapter is requested                     |
