# Changelog

## Unreleased

## [0.1.0](https://github.com/joshka/ratatui-diff/compare/v0.0.0...v0.1.0) - 2026-10-30

### Other

- Keep commit text out of Git patch sections
- Fix sticky fold reveal and speed up emphasis
- Blend translucent syntax theme colors
- Bring the docs up to date for 0.1.0
- Fix reveal, scroll, and anchor bugs in the widget
- Settle the public API before 0.1.0
- Fix per-file patch metadata, paths, and offsets
- Skip commit headers and reject context diffs
- Use ratatui::run in the crate example
- Bind the widget before rendering in examples
- Use Event::as_key_press_event in example hosts
- Add an Astro/Starlight docs site for GitHub Pages
- Keep file headers visible while scrolling
- Measure optional syntax costs
- Profile folding and resize costs
- Add optional syntax highlighting
- Integrate efficient glyph and selection storage
- Reduce prepared glyph memory
- Validate selections without copying text
- Integrate related split line alignment
- Improve pairing in split diff views
- Measure diff preparation and rendering costs
- Show colored diff statistics and totals
- Verify file folding terminal interactions
- Preserve the viewport during file expansion
- Integrate file folding contracts and UX
- Add file folding presentation and controls
- Design retained context folding contracts
- Design context folding presentation
- Expand whitespace indentation fixture
- Verify combined diff interactions
- Clarify diff interaction feedback
- Expand interactive diff fixtures
- Add diff search and match navigation
- Add source selection and text extraction
- Map display cells to source positions
- Polish diff gutters and wrapped continuations
- Add Aardvark Ink theme and visual guide
- Clarify usage documentation and API contracts ([#2](https://github.com/joshka/ratatui-diff/pull/2))
- Show captioned diff views in README
- Improve diff demo presentation
- Set up project guidance and automation
- Add navigable Ratatui diff widget
- Initialize Rust library

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
