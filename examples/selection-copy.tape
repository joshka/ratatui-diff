Set Width 1440
Set Height 720
Set FontSize 22
Set Theme "Aardvark Ink"
Set Shell "bash --noprofile --norc"
Set FontFamily "JetBrains Mono"
Hide
Type "\"$RATATUI_DIFF_VIEWER\" --aardvark-ink"
Enter
Wait+Screen /Unified/
Show
Screenshot media/selection-copy-baseline-01.png
Hide
Type "p"
Wait+Screen /Pointer/
Type "v"
Right 10
Type "c"
Wait+Screen /Copy.preview/
Show
Screenshot media/selection-copy-after-01.png
Hide
Type "q"
