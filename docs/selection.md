# Source selection

Keep selection in `DiffState`. Hosts choose keyboard bindings, interpret pointer events, and copy
returned text through their own clipboard service. `SourceBoundary` uses the same file, side, and
one-based line identity as `SourcePosition`; its byte is a caret boundary within retained source.
`SourceSelection` retains anchor and focus; ordering is by source line and byte, end-exclusive.

Selections stay on one side of one file. Endpoints must lie on grapheme boundaries. A terminated
line accepts `text.len() + 1` to include its LF; `text.len()` excludes LF. CR in CRLF remains source
text. Empty carets return an empty string. Extraction rejects missing patch context instead of
inventing text. Diff gutters, wrap arrows, split padding, control notation, tab expansion, dotted
circle bases, and final-newline cues never enter copied text.

Keyboard movement advances through whole graphemes, optional LF, and adjacent numbered source lines.
It cannot cross a missing line. Vertical movement preserves the byte offset, clamped backward to a
grapheme boundary in the adjacent source line. Line start/end refer to source text, independently of
wrapping. Anchor remains fixed when focus moves backward. Resize, scrolling, theme, and view-mode
changes keep the selection; rendering a replacement document clears it. A cloned document retains
identity. Keyboard movement changes focus only; hosts choose when to scroll it into view.

Selection styling applies after the row style, word highlights, and synthetic whitespace treatment.
Only glyphs whose original byte ranges intersect selection receive the overlay. Tabs highlight all
expanded cells; wide graphemes and control notation highlight as a unit. Headers, number gutters,
blank partners, exhausted wrapped segments, and padding remain outside selection. A selected LF has
no source glyph, so it has no painted cell. A future host-supplied search overlay should compose
before selection, with selection taking precedence for attributes it sets.

## Host example

```rust
use ratatui_diff::{DiffDocument, DiffState, SelectionMotion, Side, SourceBoundary,
    SourcePosition, SourceSelection};

let document = DiffDocument::from_text("old\n", "new\n");
let caret = SourceBoundary {
    position: SourcePosition { file: 0, side: Side::New, line: 1 },
    byte: 0,
};
let mut state = DiffState::new();
assert!(state.set_selection(&document, SourceSelection { anchor: caret, focus: caret }));
assert!(state.extend_selection(&document, SelectionMotion::Next));
assert_eq!(state.selected_text(&document).as_deref(), Some("n"));
// Pass the returned String to the host's clipboard integration.
```

For pointer selection, use the rendered `SourceRange` from hit-testing. Use `start_boundary()` or
`end_boundary()` as a caret boundary based on the host's drag policy. Unified context offers both
sides: choose a side at the start of the gesture and retain it. Reject other-side or other-file hits
during that gesture. A host may turn a gutter's line identity into a whole-line selection, but
padding has no source. Invalidate stale hit-testing before processing events following a resize or
option replacement.
