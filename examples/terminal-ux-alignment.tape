# Matched lines, wide-cell continuations, and original source mapping after split alignment.
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
Type "exec \"$RATATUI_DIFF_VIEWER\" --fixture alignment --aardvark-ink"
Enter
Wait+Screen@5s "timeout_ms:"
Type "s"
AssertText "Split"
# The inserted annotation has a blank old counterpart; subsequent changed lines pair by content.
AssertCells 2 5 '[" "," "," "," "," "]'
AssertCells 49 5 '["#"," ","A","u","d","i","t"]'
AssertCells 5 6 '["t","i","m","e","o","u","t","_","m","s",":"," ","1","0","0","0"]'
AssertCells 49 6 '["t","i","m","e","o","u","t","_","m","s",":"," ","2","5","0","0"]'
AssertCells 17 9 '["界"," "]'
AssertCells 61 9 '["世"," ","界"," "]'
Caption "Split alignment leaves the annotation unpaired and matches the changed source lines"
Screenshot media/terminal-ux/alignment-paired.png
State media/terminal-ux/alignment-paired.json
# Source previews must follow original side/line mappings after alignment.
MouseMove 61 6
MouseDown Left
MouseMove 64 6
MouseUp Left
Type "c"
AssertText 'Text preview: "2500"'
Escape
MouseMove 17 6
MouseDown Left
MouseMove 20 6
MouseUp Left
Type "c"
AssertText 'Text preview: "1000"'
Resize 50 16
Type "w"
AssertText 'Text preview: "1000"'
State media/terminal-ux/alignment-wrapped.json
Type "q"
