Output media/viewer.gif
Set Width 1440
Set Height 720
Set FontSize 22
Set Theme "Aardvark Ink"
Set Shell "bash --noprofile --norc"
Set FontFamily "JetBrains Mono"
Caption "Unified: additions and deletions together, with changed words emphasized"
Hide
Type "\"$RATATUI_DIFF_VIEWER\""
Enter
Wait+Screen /Unified/
Show
Screenshot media/unified.png
Sleep 5s
Hide
Type "s"
Wait+Screen /Split/
Caption "Split: old source on the left, new source on the right"
Show
Screenshot media/split.png
Sleep 5s
Hide
Type "t"
Wait+Screen /whitespace.on/
Caption "Whitespace: visible spaces in synchronized split panes"
Show
Screenshot media/whitespace.png
Sleep 5s
Hide
Type "q"
