# Integration scenario: requires the viewer's context fixture and e/z folding controls.
Output media/terminal-ux/folding-interaction-20261009.gif
Set Shell "bash --noprofile --norc"
Set Width 1100
Set Height 660
Set FontSize 22
Set FontFamily "JetBrains Mono"
Set Theme "Aardvark Ink"
Set TypingSpeed 0ms
Set CursorBlink false
Hide
Type "exec \"$RATATUI_DIFF_VIEWER\" --fixture context --aardvark-ink"
Enter
Wait+Screen@5s "22 unchanged lines"
AssertCells 2 5 '["…"," ","2"," ","u","n","c","h","a","n","g","e","d"]'
AssertStyle 2 5 13 '{"fg":"#6f7a8f","bg":"#0f141f"}'
AssertAbsent "rule 20:"
Caption "Collapsed: retained context around edits; 22 middle lines hidden"
Show
Screenshot media/terminal-ux/folding-collapsed-20261009.png
State media/terminal-ux/folding-collapsed-20261009.json
Sleep 1800ms
Caption "e expands the first fold, then the middle fold"
Type "e"
AssertText "rule 01:"
Sleep 1000ms
Type "e"
AssertText "rule 10:"
Screenshot media/terminal-ux/folding-expanded-20261009.png
State media/terminal-ux/folding-expanded-20261009.json
Sleep 1800ms
Caption "z collapses retained context again"
Type "z"
Home
AssertText "22 unchanged lines"
Sleep 1500ms
Caption "A mouse click opens the middle fold"
AssertCells 2 14 '["…"," ","2","2"," ","u","n","c","h","a","n","g","e","d"]'
MouseMove 10 14
MouseDown Left
MouseUp Left
AssertText "rule 10:"
Sleep 1800ms
Type "z"
Home
AssertText "22 unchanged lines"
Caption "Search for rule 20; f reveals its retained source line"
Type "/rule 20"
Enter
Type "f"
AssertText "rule 20: retain source context"
Screenshot media/terminal-ux/folding-search-20261009.png
State media/terminal-ux/folding-search-20261009.json
Sleep 2200ms
Caption "Split, wrapped, monochrome; revealed source survives live resize"
Type "swm"
AssertText "Split"
AssertText "↪"
AssertText "Monochrome"
Resize 40 16
AssertText "/rule 20"
# Resize retains the source viewport anchor; navigate to reveal the match in the smaller view.
Type "f"
AssertText "rule 20:"
Screenshot media/terminal-ux/folding-narrow-20261009.png
State media/terminal-ux/folding-narrow-20261009.json
Sleep 2200ms
Resize 70 23
AssertText "rule 20:"
Sleep 1000ms
Hide
# Known patch gaps report unavailable context and cannot create a source selection.
Escape
Type "swm"
Resize 70 24
End
AssertCells 2 11 '["…"," ","c","o","n","t","e","x","t"," ","u","n","a","v","a","i","l","a","b","l","e"]'
MouseMove 10 11
MouseDown Left
MouseUp Left
Type "c"
AssertAbsent "Text preview:"
AssertText "context unavailable"
State media/terminal-ux/folding-unavailable-20261009.json
Type "q"
