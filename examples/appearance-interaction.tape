# Exercise clipping, wrapping, scrolling, mode changes, and theme reset at a narrow width.
# Pinned from the Ghostty Aardvark Ink palette; RGB diff colors are selected separately.
Set Width 772
Set Height 352
Set Padding 22
Set FontSize 22
Set Theme '{"name":"Aardvark Ink","background":"#0f141f","foreground":"#b4bcca","cursor":"#b4bcca","black":"#222734","red":"#c26265","green":"#52aa60","yellow":"#ad9b49","blue":"#487fd4","magenta":"#af5bd1","cyan":"#269d9a","white":"#5a6377","brightBlack":"#3a4152","brightRed":"#e48383","brightGreen":"#75cf84","brightYellow":"#c7b461","brightBlue":"#76a8f2","brightMagenta":"#d58bf0","brightCyan":"#52c4c0","brightWhite":"#dfe5ee"}'
Set Shell "bash --noprofile --norc"
Set FontFamily "JetBrains Mono"
Set TypingSpeed 1ms
Hide
Type "\"$RATATUI_DIFF_VIEWER\" --aardvark-ink"
Enter
Wait+Screen /Unified/
Show
Screenshot media/interaction-unified.png
Hide
Type "s"
Wait+Screen /Split/
Show
Screenshot media/interaction-split.png
Hide
Right
Sleep 300ms
Show
Screenshot media/interaction-horizontal.png
Hide
Left
Type "wt"
Wait+Screen /·/
Show
Screenshot media/interaction-wrapped.png
Hide
End
Wait+Screen /should_retry/
Show
Screenshot media/interaction-end.png
Hide
Type "s"
Wait+Screen /Unified/
Show
Screenshot media/interaction-mode.png
Hide
Type "m"
Wait+Screen /Monochrome/
Show
Screenshot media/interaction-mono.png
Hide
Home
Type "sm"
Wait+Screen /Split.*Aardvark.Ink/
Show
Screenshot media/interaction-reset.png
Hide
Type "q"
