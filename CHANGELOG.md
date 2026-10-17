# Changelog

## Unreleased

First release: a unified and split diff widget for Ratatui, with state retained between frames.

- **Inputs:** compare two strings (`DiffDocument::from_text`, `compare`), parse unified or Git
  patches including `git show`, `git log -p`, and `git format-patch` output (`parse`), or validate
  caller-built files and hunks (`new`). Renames, copies, binary and metadata-only changes, CRLF, and
  missing final newlines are preserved; hunk headings are kept.
- **Presentation:** unified or split view with similarity-aligned replacements, word highlights,
  Unicode-aware wrapping and clipping, line numbers, whitespace markers, tab stops, and four theme
  presets (`dark`, `light`, `monochrome`, `aardvark_ink`) with independently customizable styles.
- **Navigation:** line, page, hunk, and file movement, horizontal scrolling, source-position jumps,
  and viewport anchoring across resizes and mode changes.
- **Folding:** collapse whole files and fold retained unchanged context, including restoring saved
  file preferences before the first render.
- **Headers and totals:** opt-in sticky file headers and an insertion/deletion totals row.
- **Search:** case-sensitive literal search over available source, on either or both sides.
- **Selection and pointer input:** source-anchored selection that extracts exact source text, and
  hit-testing from terminal cells to source graphemes, gutters, fold summaries, and file headers.
- **Syntax colors (optional `syntax` feature):** bundled Syntect grammars and two-face themes,
  prepared outside rendering, with optional contrast adaptation for the diff theme.
