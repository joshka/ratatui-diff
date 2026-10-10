# Supplied text counts remain independent of folding and geometry.
Set Shell "bash --noprofile --norc"
Set Width 1340
Set Height 680
Set FontFamily "JetBrains Mono"
Set FontSize 24
Set LetterSpacing 0
Set Padding 20
Set Theme "Aardvark Ink"
Set TypingSpeed 0ms
Set CursorBlink false
Hide
Type "exec \"$RATATUI_DIFF_VIEWER\" --fixture files --aardvark-ink"
Enter
Wait+Screen@5s "src/routes.txt"
Type "dZ"
AssertText "5 files · +9 −6 · 1 binary · 1 metadata"
AssertText "assets/banner.png"
AssertAbsent "@@"
Caption "Statistics count supplied text across collapsed binary and metadata files"
Screenshot media/terminal-ux/statistics-collapsed.png
State media/terminal-ux/statistics-collapsed.json
Type "A"
AssertText "5 files · +9 −6 · 1 binary · 1 metadata"
AssertText "unchanged lines"
Type "Zsw"
Resize 50 16
AssertText "5 files · +9 −6 · 1 binary · 1 metadata"
AssertText "Split"
State media/terminal-ux/statistics-narrow.json
# Seven terminal rows leave exactly one widget row after the host title/footer/margins.
Resize 50 7
AssertText "5 files · +9 −6 · 1 binary · 1 metadata"
State media/terminal-ux/statistics-one-row.json
# At five rows the host assigns no widget area; restoring it must retain the totals.
Resize 50 5
AssertText "Split"
AssertAbsent "5 files"
State media/terminal-ux/statistics-zero-area.json
Resize 92 25
AssertText "5 files · +9 −6 · 1 binary · 1 metadata"
Type "d"
AssertAbsent "5 files"
Type "q"
