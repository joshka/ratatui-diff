# Split, wrapped search: Enter must reveal a match beside a longer wrapped partner line.
Set Width 1100
Set Height 330
Set Padding 22
Set FontSize 22
Set Theme '{"name":"Aardvark Ink","background":"#0f141f","foreground":"#b4bcca","cursor":"#b4bcca","black":"#222734","red":"#c26265","green":"#52aa60","yellow":"#ad9b49","blue":"#487fd4","magenta":"#af5bd1","cyan":"#269d9a","white":"#5a6377","brightBlack":"#3a4152","brightRed":"#e48383","brightGreen":"#75cf84","brightYellow":"#c7b461","brightBlue":"#76a8f2","brightMagenta":"#d58bf0","brightCyan":"#52c4c0","brightWhite":"#dfe5ee"}'
Set Shell "bash --noprofile --norc"
Set FontFamily "JetBrains Mono"
Set TypingSpeed 1ms
Hide
Type "\"$RATATUI_DIFF_VIEWER\" --fixture patch --aardvark-ink"
Enter
Wait+Screen /src\/client/
Type "s"
Wait+Screen /Split/
Type "w"
Wait+Screen /Split.{3}wrapped/
Type "/run("
Wait+Screen /Enter.to.find/
Enter
Wait+Screen /1\/1.{3}Split/
Show
Screenshot media/fixtures-patch-search.png
Hide
Type "q"
