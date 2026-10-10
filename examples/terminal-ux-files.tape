# File controls complement retained-context controls; all coordinates are terminal cells.
Output media/terminal-ux/file-folding-interaction-20261009.gif
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
Type "exec \"$RATATUI_DIFF_VIEWER\" --fixture files --aardvark-ink"
Enter
Wait+Screen@5s "src/routes.txt"
Caption "Before: expanded file with retained context around both edits"
Show
Screenshot media/terminal-ux/file-folding-expanded-20261009.png
State media/terminal-ux/file-folding-expanded-20261009.json
Sleep 1800ms
Caption "Click the file header to collapse its body"
MouseMove 12 3
MouseDown Left
MouseUp Left
AssertText "src/routes.txt"
AssertText "src/settings.txt"
AssertCells 2 3 '["▸"," ","s","r","c","/","r","o","u","t","e","s",".","t","x","t"]'
Screenshot media/terminal-ux/file-folding-collapsed-20261009.png
State media/terminal-ux/file-folding-collapsed-20261009.json
Sleep 1800ms
Caption "Z folds all files, including binary and mode-only changes"
Type "Z"
AssertText "src/routes.txt"
AssertText "src/settings.txt"
AssertText "docs/review-guide.txt"
AssertText "assets/banner.png"
AssertText "scripts/check.sh"
AssertText "+3 −2"
AssertText "binary"
AssertText "metadata"
AssertCells 2 4 '["▸"," ","s","r","c","/","s","e","t","t","i","n","g","s",".","t","x","t"]'
AssertAbsent "@@"
AssertAbsent "unchanged lines"
Screenshot media/terminal-ux/file-folding-all-20261009.png
State media/terminal-ux/file-folding-all-20261009.json
Sleep 2200ms
Caption "A expands all files while retaining the current overview anchor"
Type "A"
AssertText "src/routes.txt"
AssertCells 2 3 '["▾"," ","s","r","c","/","r","o","u","t","e","s",".","t","x","t"]'
State media/terminal-ux/file-folding-all-open-anchor-20261009.json
Sleep 1500ms
Type "Z"
Home
Caption "Tab focuses the next file; Enter opens it without changing context folds"
Tab
AssertText "File focus · src/routes.txt"
Sleep 800ms
Tab
AssertText "File focus · src/settings.txt"
Sleep 800ms
Shift+Tab
AssertText "File focus · src/routes.txt"
Sleep 800ms
Tab
AssertText "File focus · src/settings.txt"
Enter
AssertText "unchanged lines"
AssertText "File focus · src/settings.txt"
AssertCells 2 3 '["▸"," ","s","r","c","/","r","o","u","t","e","s",".","t","x","t"]'
AssertCells 2 4 '["▾"," ","s","r","c","/","s","e","t","t","i","n","g","s",".","t","x","t"]'
Sleep 1800ms
Caption "Click a lower file header to close and reopen it in place"
MouseMove 12 4
MouseDown Left
MouseUp Left
AssertCells 2 4 '["▸"]'
Sleep 900ms
MouseDown Left
MouseUp Left
AssertCells 2 4 '["▾"]'
AssertCells 2 3 '["▸"]'
AssertText "unchanged lines"
Sleep 1800ms
Caption "A opens every file; scrolling and Enter fold the current file"
Type "A"
PageDown
AssertAbsent "File focus"
Enter
State media/terminal-ux/file-folding-scrolled-20261009.json
Sleep 1800ms
Caption "Folded file headers survive split, wrap, monochrome, and live resize"
Type "Zswm"
Resize 50 16
AssertText "Monochrome"
AssertText "assets/banner.png"
AssertAbsent "@@"
Screenshot media/terminal-ux/file-folding-resized-20261009.png
State media/terminal-ux/file-folding-resized-20261009.json
Sleep 2200ms
Resize 92 25
Caption "A restores the expanded file bodies"
Type "A"
Home
AssertText "unchanged lines"
Sleep 1800ms
Hide
Type "q"
