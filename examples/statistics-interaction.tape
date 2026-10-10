# Totals are optional and remain pinned while the document scrolls.
Output media/statistics-interaction-20261009-v2.gif
Set Shell "bash --noprofile --norc"
Set Width 1340
Set Height 680
Set FontSize 24
Set FontFamily "JetBrains Mono"
Set Padding 20
Set Theme "Aardvark Ink"
Set TypingSpeed 0ms
Set CursorBlink false
Hide
Type "exec \"$RATATUI_DIFF_VIEWER\" --fixture files --aardvark-ink"
Enter
Wait+Screen@5s "src/routes.txt"
Type "Z"
Wait+Screen@5s "scripts/check.sh"
Caption "Collapsed files: green additions and red deletions"
Show
Sleep 2s
Hide
Type "d"
Wait+Screen@5s "5 files · +9 −6"
Sleep 200ms
Show
Caption "d shows document totals, including binary and metadata files"
Screenshot media/statistics-overview-20261009-v2.png
Sleep 3s
Hide
Type "A"
Wait+Screen@5s "unchanged lines"
Sleep 200ms
Show
Caption "Totals stay visible when files expand"
Sleep 2s
Hide
PageDown
Wait+Screen@5s "src/settings.txt"
Sleep 200ms
Show
Caption "The summary stays pinned while source rows scroll"
Sleep 2s
Screenshot media/statistics-scrolled-20261009-v2.png
Hide
Type "d"
Sleep 200ms
Show
Caption "d hides totals and returns the row to the diff"
Sleep 2s
Hide
Type "q"
