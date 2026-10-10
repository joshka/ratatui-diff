Output media/terminal-ux/syntax-interaction-v1.gif
# Pinned from the Ghostty Aardvark Ink palette; RGB diff colors are selected separately.
Set Width 1458
Set Height 594
Set Padding 22
Set FontSize 22
Set Theme '{"name":"Aardvark Ink","background":"#0f141f","foreground":"#b4bcca","cursor":"#b4bcca","black":"#222734","red":"#c26265","green":"#52aa60","yellow":"#ad9b49","blue":"#487fd4","magenta":"#af5bd1","cyan":"#269d9a","white":"#5a6377","brightBlack":"#3a4152","brightRed":"#e48383","brightGreen":"#75cf84","brightYellow":"#c7b461","brightBlue":"#76a8f2","brightMagenta":"#d58bf0","brightCyan":"#52c4c0","brightWhite":"#dfe5ee"}'
Set Shell "bash --noprofile --norc"
Set FontFamily "JetBrains Mono"
Set TypingSpeed 1ms
Hide
Type "exec \"$RATATUI_DIFF_VIEWER\" --fixture syntax --aardvark-ink --syntax Coldark-Dark --syntax-contrast"
Enter
Wait+Screen@5s "syntax on (h)"
AssertCells 9 7 '["f","n"]'
AssertStyle 9 7 2 '{"fg":"#e9ae7e","bg":"#0f141f"}'
Caption "Prepared syntax: keywords, strings, comments; diff backgrounds remain visible"
Show
Screenshot media/terminal-ux/syntax-on-v1.png
State media/terminal-ux/syntax-on-v1.json
Sleep 1800ms
Caption "h hides syntax without changing the document or its layout"
Type "h"
Wait+Screen@5s "syntax off (h)"
AssertCells 9 7 '["f","n"]'
AssertStyle 9 7 2 '{"fg":"#b4bcca","bg":"#0f141f"}'
Screenshot media/terminal-ux/syntax-off-v1.png
State media/terminal-ux/syntax-off-v1.json
Sleep 1800ms
Caption "h restores the prepared colors without tokenizing during rendering"
Type "h"
Wait+Screen@5s "syntax on (h)"
AssertCells 9 7 '["f","n"]'
AssertStyle 9 7 2 '{"fg":"#e9ae7e","bg":"#0f141f"}'
Sleep 1800ms
Hide
Type "q"
