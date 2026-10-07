# Architecture

The model owns validated files, hunks, numbered lines, line endings, and highlight byte ranges.
Documents are immutable after construction. Comparison and patch parsing adapt external libraries
into this model; consumers may provide structured input directly.

Comparison pairs deletion/insertion runs in source order and computes word ranges once. Runs over
256 lines, pairs over 8 KiB, or pairs over 2,048 whitespace-delimited words retain whole-line
styling. These bounds protect interactive preparation from fine-grained adversarial input. They do
not bound whole-document comparison time.

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
