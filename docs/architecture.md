# Architecture

The model owns validated files, hunks, numbered lines, line endings, and highlight byte ranges.
Documents are immutable after construction. Comparison and patch parsing adapt external libraries
into this model; consumers may provide structured input directly.

`from_text` retains three context lines around edits. `compare(old, new, usize::MAX)` retains all
source lines for changed inputs, using the existing hunk and line model; identical inputs still
produce no hunks. A context line stores its text once with both original source numbers. Smaller
comparison context counts discard omitted text, and parsed patches supply only their advertised hunk
text. Neither input form promises retrieval of missing source.

Comparison and split layout share bounded, monotonic similarity anchors for deletion/insertion runs.
Unanchored gaps and oversized runs pair in source order; see [split alignment](split-alignment.md).
Comparison computes word ranges once. Automatic ranges join changed words separated only by
whitespace, leaving outer unchanged whitespace and unchanged words outside the highlight. Explicit
caller ranges remain authoritative. Runs over 256 lines, pairs over 8 KiB, or pairs over 2,048
whitespace-delimited words retain whole-line styling. These bounds protect interactive preparation
from fine-grained adversarial input. They do not bound whole-document comparison time.

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

File folding retains the file header and hides its metadata, hunks, and source. Files default to
expanded. Document-issued `FileFold` handles carry immutable file indexes and document identities. A
fresh state can queue preferences before drawing by binding to the first handle's document; the
matching first render preserves those preferences. Rendering another document clears them and binds
to the replacement. Clones preserve identity. Width and mode changes preserve both file and context
expansion; context-radius changes preserve file expansion. Collapsed file headers remain control
hits with no source coordinates. See [folding controls](folding.md) for saved-state and bulk host
flows.

Context folding changes presentation of retained context runs within a hunk. It never crosses an
unavailable gap or changes source text, line numbers, or hunk ranges. The presentation option
selects how many context lines remain beside changes: `Diff::context_lines(None)` disables folding
by default, while `Some(3)` keeps the nearest three lines at leading and trailing changes and three
at each end of an internal run. Only the remaining interior becomes a fold. The widget option
changes visible context; the `DiffDocument::compare` context argument determines which source is
retained.

State owns expansion independently for each fold. File/hunk indexes and original old/new line ranges
identify folds across width and mode changes. Fold handles also retain document identity and context
radius; changing either clears expansion and rejects stale handles. `context_folds` includes
expanded candidates so hosts can offer collapse controls. A collapsed fold is a synthetic
`HitTest::Fold` with no source byte range or selection boundary. Known leading and inter-hunk gaps
use an unavailable context cue with an ordinary header hit; no trailing gap is inferred from an
unknown source extent. Context candidates and their expansion preferences remain available while
their containing file is collapsed. File navigation visits its header without expanding it; hunk
navigation opens the containing file and visits the original hunk header.

Search indexes and selection extraction include folded retained text. `scroll_to_source` accepts a
retained line inside a collapsed fold, then expands and scrolls to it on the next render. Requested
search or selection reveal also opens the containing file and expands the context fold before
resolving its source position in the new layout. Setting a query or selection alone leaves folds
collapsed. Manual collapse cancels pending automatic reveal. Extraction across unavailable source
gaps continues to fail rather than inserting display summaries or guessed text. Hosts own filesystem
and repository access if they need to construct a replacement document with more source.

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
