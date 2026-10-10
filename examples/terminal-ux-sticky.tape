# One scenario: scripts/sticky-headers.py derives baseline and paced captures from this tape.
# Candidate blocks contain only expectations or the pinned-header click, never scroll sequences.
Set Shell "bash --noprofile --norc"
Set Width 1340
Set Height 680
Set FontSize 24
Set FontFamily "JetBrains Mono"
Set LetterSpacing 0
Set Padding 20
Set Theme "Aardvark Ink"
Set TypingSpeed 0ms
Set CursorBlink false
Hide
Type "exec \"$RATATUI_DIFF_VIEWER\" --fixture multi-file --aardvark-ink --sticky-headers"
Enter
Wait+Screen@5s "src/routes.txt"
Caption "AFTER | Opt in with .sticky_file_headers(true)"
Show
AssertCells 2 3 '["▾"," ","s","r","c","/","r","o","u","t","e","s",".","t","x","t"]'
Screenshot media/terminal-ux/sticky/start.png
State media/terminal-ux/sticky/start.json
Sleep 250ms
Type "jjjjjjjj"
Sleep 250ms
AssertText "route 24: revised"
# Candidate begin
AssertCells 2 3 '["▾"," ","s","r","c","/","r","o","u","t","e","s",".","t","x","t"]'
AssertStyle 4 3 14 '{"fg":"#52c4c0","bg":"#0f141f","bold":true}'
AssertStyle 21 3 2 '{"fg":"#75cf84","bg":"#0f141f","bold":true}'
AssertStyle 24 3 2 '{"fg":"#e48383","bg":"#0f141f","bold":true}'
# Candidate end
# Baseline: AssertAbsent "src/routes.txt"
Caption "AFTER | Eight rows down: the path and +9 / -3 remain visible"
Screenshot media/terminal-ux/sticky/scrolled.png
State media/terminal-ux/sticky/scrolled.json
Sleep 250ms
Type "s"
Sleep 250ms
AssertText "Split"
# Candidate begin
AssertCells 2 3 '["▾"," ","s","r","c","/","r","o","u","t","e","s",".","t","x","t"]'
# Candidate end
# Baseline: AssertAbsent "src/routes.txt"
Caption "AFTER | Split view keeps one header above both source panes"
Screenshot media/terminal-ux/sticky/split.png
State media/terminal-ux/sticky/split.json
Sleep 250ms
Type "w"
Resize 50 16
Sleep 250ms
AssertText "Split"
# Candidate begin
AssertCells 2 3 '["▾"," ","s","r","c","/","r","o","u","t","e","s",".","t","x","t"]'
# Candidate end
# Baseline: AssertAbsent "src/routes.txt"
Caption "AFTER | Narrow wrapped view retains file identity"
Screenshot media/terminal-ux/sticky/narrow.png
State media/terminal-ux/sticky/narrow.json
Sleep 250ms
Resize 92 25
Type "sw"
Type "/setting 24"
Enter
AssertText "setting 24: revised"
Escape
Sleep 250ms
# Candidate begin
AssertCells 2 3 '["▾"," ","s","r","c","/","s","e","t","t","i","n","g","s",".","t","x","t"]'
# Candidate end
Caption "AFTER | Search crosses the file boundary: the pinned path changes"
Screenshot media/terminal-ux/sticky/boundary.png
State media/terminal-ux/sticky/boundary.json
Sleep 250ms
# Candidate begin
MouseMove 12 3
MouseDown Left
MouseUp Left
AssertCells 2 3 '["▸"," ","s","r","c","/","s","e","t","t","i","n","g","s",".","t","x","t"]'
# Candidate end
# Baseline: Enter
AssertAbsent "setting 24: revised"
Caption "AFTER | Click the pinned header to fold its file"
Screenshot media/terminal-ux/sticky/folded.png
State media/terminal-ux/sticky/folded.json
Sleep 250ms
Type "Zd"
Home
AssertText "3 files · +27 −9"
AssertAbsent "@@"
Caption "AFTER | Folding preserves counts; document totals stay above the header"
Screenshot media/terminal-ux/sticky/totals.png
State media/terminal-ux/sticky/totals.json
Sleep 250ms
Resize 50 7
AssertText "3 files · +27 −9"
AssertAbsent "src/routes.txt"
State media/terminal-ux/sticky/one-row.json
Resize 50 5
AssertAbsent "3 files"
State media/terminal-ux/sticky/zero-area.json
Resize 92 25
Type "Ad"
End
AssertText "check 47: ready"
# Candidate begin
AssertCells 2 3 '["▾"," ","d","o","c","s","/","c","h","e","c","k","l","i","s","t",".","t","x","t"]'
# Candidate end
Caption "AFTER | End still reaches the final supplied source line"
Screenshot media/terminal-ux/sticky/end.png
State media/terminal-ux/sticky/end.json
Sleep 250ms
Hide
Type "q"
