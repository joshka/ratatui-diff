# Deferred Work

The rendering contract remains `StatefulWidget` with separate `DiffState`; the mutable-widget
prototype is skipped. Establish visual and interaction evidence for the existing layouts before
adding search, selection, or syntax coloring. See [architecture](architecture.md) for ownership and
[appearance captures](appearance-captures.md) for visual checks.

These capabilities have no release commitment. Record implementation evidence and compatibility
impact when selecting one; remove it from this list only when its documented contract is fulfilled.

| Capability                                           | Why deferred                                                   | Revisit when                                                       |
| ---------------------------------------------------- | -------------------------------------------------------------- | ------------------------------------------------------------------ |
| Syntax coloring                                      | Adds language data, dependencies, and style composition        | A consumer needs language-aware coloring and accepts the footprint |
| Structural comparison and engine adapters            | Needs richer semantics than text patches                       | A consumer supplies a concrete engine integration                  |
| Search and match navigation                          | Adds query state and match/source mapping                      | Navigation alone no longer serves review workflows                 |
| Selection, copying, mouse, hit-testing               | Needs stable source/display selection contracts                | A host requires interaction beyond viewport commands               |
| Context folding and expansion                        | Requires retained context and explicit missing-data boundaries | A host supplies full source or context retrieval                   |
| Moved-line detection and improved alignment          | Adds heuristics and potentially expensive comparisons          | Measured examples show source-order pairing is inadequate          |
| Combined, three-way, conflict views and editing      | Changes the two-sided data model                               | A concrete merge-review workflow establishes requirements          |
| Non-UTF-8 sources                                    | Requires byte-oriented rendering contracts                     | A consumer needs faithful arbitrary-byte display                   |
| Streaming and million-line inputs                    | Changes ownership, indexes, and incremental state              | Large in-memory benchmarks establish a limit                       |
| Standalone pager, Git/jj integration, external tools | Product/I/O scope belongs outside the widget                   | A separate application or adapter is requested                     |
