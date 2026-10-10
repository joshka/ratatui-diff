# Capabilities and deferred work

The library provides unified and split rendering through `&Diff` and a separate `DiffState`.
Applications choose inputs and presentation, retain state between frames, and own terminal events,
source retrieval, clipboard access, and repository workflows. These capabilities describe the
current checkout; a configured release workflow does not establish that this revision is published.

## Available capabilities

- Construct generated comparisons, parsed patches, or validated structured documents. Comparison
  retains three context lines by default; `DiffDocument::compare(old, new, usize::MAX)` retains all
  source for changed inputs. Identical inputs still produce no hunks.
- Choose unified/split mode, wrapping, gutters, whitespace, tabs, themes, and word emphasis. Bounded
  similarity anchors align replacement lines in source order, with positional fallback for
  dissimilar or oversized runs. See [split alignment](split-alignment.md).
- Search literal retained source, hit-test cells, select with source boundaries, and extract exact
  text. Search and extraction include folded retained text; omitted patch context cannot be
  reconstructed. Hosts implement input and clipboard integration. See
  [coordinates](interaction-coordinates.md) and [selection](selection.md).
- Fold files and retained context independently, restore file preferences before the first render,
  and reveal retained search/selection targets explicitly. Folding changes presentation rather than
  source. See [folding controls](folding.md).
- Opt into `Diff::show_stats(true)` to reserve one row for supplied text insertion/deletion totals,
  with separate binary and metadata-only counts. Totals survive folding and layout changes; they do
  not estimate binary sizes or missing source.
- Enable the non-default `syntax` feature for owned, document-bound syntax styles prepared outside
  rendering. Choose full snapshots for lexical continuity or retained input for best-effort hunks;
  choose original RGB colors or explicit contrast adaptation. See
  [syntax styling](syntax-styling.md) for limits, overlays, native dependencies, and bundled
  notices.

Cached painting visits visible rows, while preparation, initial layout, query updates, extraction,
and some rebuilds can process retained input. [Performance evidence](performance.md) separates these
costs and records measurement limits; it is not a latency or memory guarantee. Required
[terminal acceptance](testing.md#terminal-consumer-acceptance) covers real host interactions and
source preservation, including syntax. Hosted release prerequisites remain in
[releasing](releasing.md).

## Deferred capabilities

These capabilities have no release commitment. Select work from a concrete caller requirement and
record its evidence and compatibility impact. The mutable-widget prototype remains skipped: the
chosen contract is `StatefulWidget` with separate state.

| Capability                                           | Why deferred                                             | Revisit when                                                                       |
| ---------------------------------------------------- | -------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| Structural comparison and engine adapters            | Needs richer semantics than text patches                 | A consumer supplies a concrete engine integration                                  |
| Regex and advanced search                            | Adds query policy and dependency choices                 | A host needs more than case-sensitive literal source search                        |
| Retrieval of unavailable context                     | Requires source access and document replacement          | A host supplies a filesystem or repository adapter                                 |
| Moved-line detection                                 | Adds heuristics and expensive comparisons                | A review workflow needs correspondence across source order                         |
| Combined, three-way, conflict views and editing      | Changes the two-sided model                              | A merge-review workflow establishes requirements                                   |
| Non-UTF-8 sources                                    | Requires byte-oriented rendering contracts               | A consumer needs faithful arbitrary-byte display                                   |
| Streaming and million-line inputs                    | Changes ownership, indexes, and incremental state        | Large in-memory measurements establish a limit                                     |
| Standalone pager, Git/jj integration, external tools | Product and I/O scope belongs outside the widget         | A separate application or adapter is requested                                     |
| Documentation website                                | Needs curated examples, media provenance, and deployment | Implement the [small-site proposal](docs-site-design.md) in a separate review unit |
