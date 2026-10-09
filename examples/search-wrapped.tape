# Exercise clipping, wrapping, scrolling, mode changes, and theme reset at a narrow width.
# Pinned from the Ghostty Aardvark Ink palette; RGB diff colors are selected separately.
Set Width 730
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
Type "sw"
Wait+Screen /Split/
Sleep 100ms
Show
Screenshot media/search-host-baseline-wrapped-20261009-v3.png
Hide
Type "/status"
Enter
Wait+Screen /1\/3/
Sleep 100ms
Show
Screenshot media/search-host-after-wrapped-20261009-v3.png
Hide
Type "ff"
Wait+Screen /3\/3/
Sleep 100ms
Show
Screenshot media/search-host-next-wrapped-20261009-v3.png
Hide
Type "s"
Wait+Screen /3\/3.*Unified/
Sleep 150ms
Sleep 100ms
Show
Screenshot media/search-host-unified-wrapped-20261009-v3.png
Hide
Type "q"
