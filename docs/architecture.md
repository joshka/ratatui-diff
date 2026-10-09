# Architecture

The model owns validated files, hunks, numbered lines, line endings, and highlight byte ranges.
Documents are immutable after construction. Comparison and patch parsing adapt external libraries
into this model; consumers may provide structured input directly.

Comparison pairs deletion/insertion runs in source order and computes word ranges once. Automatic
ranges join changed words separated only by whitespace, leaving outer unchanged whitespace and
unchanged words outside the highlight. Explicit caller ranges remain authoritative. Runs over 256
lines, pairs over 8 KiB, or pairs over 2,048 whitespace-delimited words retain whole-line styling.
These bounds protect interactive preparation from fine-grained adversarial input. They do not bound
whole-document comparison time.

`&Diff` implements `StatefulWidget` with a separate `DiffState`. This is the chosen rendering
contract; a mutable-widget prototype is not planned. Hosts can recreate the widget with current
presentation options each frame while state retains navigation and reusable layout.

The widget owns presentation options; state owns layout and viewport caches. Immutable documents
carry identities so replacing a document invalidates caches even if its allocation address is
reused. Theme changes leave layout intact. Width, wrapping, mode, gutters, whitespace, and tab
settings invalidate layout. Height changes only clamp the viewport.

Logical rows index source content and aligned split runs. Width, gutter, and wrapping changes reuse
prepared glyphs and alignment; document, mode, whitespace, or tab changes rebuild them. Screen rows
refer to wrapped glyph ranges. Frame rendering visits only visible rows and uses display-column
lookup for horizontal clipping. Source indexes support navigation without a scan through the
document. Comparison never runs during rendering. First layout and resize may still process the
whole document.

A displayed line is a screen row, including headers and wrapped continuations. Source lines are
numbered independently. Mode/width changes preserve the nearest available source anchor; document
replacement resets the viewport. Missing patch context cannot be recovered by rendering.

Split content uses equal source widths and one separator cell. At even widget widths, the spare
rightmost cell is padding painted with the right row's style. It adds no source column, so wrapping
and horizontal clipping stay synchronized. Missing partners and exhausted wrapped segments remain
blank; missing partners use the context style and have no source identity.

Number gutters have one space before the change marker and no added space after it. The marker uses
the source row's style, independently of the quieter numbers and pane separator. Wrapped screen rows
retain a continuation flag: the first row shows numbers, later rows show a dim `↪`, and exhausted
split sides show neither numbers nor markers. Unified context uses one continuation cue in the new
number column. Source mapping continues to use the logical row's original numbers.

Synthetic cues recede toward their composed background: whitespace uses a quarter of the RGB channel
difference and continuation arrows use half of the number color's difference. Palette-owned colors
fall back to `DIM`, avoiding assumptions about terminal RGB values. Literal source dots and arrows
remain source text, and word emphasis retains its background and modifiers.

Hit-testing reuses prepared glyph byte ranges and the last painted viewport rectangle and offsets.
Navigation requires a redraw before cell lookup; hosts invalidate mapping when pending replacement
or resize events precede drawing. Source ranges identify line-local UTF-8 bytes, while gutters and
final-newline cues retain only line identities. See
[interaction coordinates](interaction-coordinates.md) for Unicode, padding, and missing-context
contracts.

Selection retains source boundaries in state, independently of viewport geometry. Rendering reuses
hit-testing glyph byte ranges and composes the selection overlay after word and whitespace styles.
Extraction uses numbered source lines and rejects omitted context. The
[selection contract](selection.md) defines ordering, source-line keyboard movement, line endings,
and host pointer/copy integration.

Literal search is updated explicitly through `DiffState::set_search`, outside rendering. Results
retain source line identities and UTF-8 byte ranges, with a sorted source index for visible glyph
lookup. Both-side search scans each supplied context line once and indexes both painted copies. It
searches available hunk text only; omitted context cannot be reconstructed. Query/side updates clear
the active occurrence; document replacement clears the query and results without scanning in a
frame. Navigation reveals the first intersecting grapheme through the existing source and screen
indexes. Mode/width changes retain results and reveal the selected occurrence in the new layout.
Search styles patch over line/word styles after synthetic cue dimming; defaults add underline or
reverse video without replacing source foregrounds or word backgrounds. Selection overlays search
styles; advanced search remains deferred.
