# Adapted from Betamax PR172 at 2688c76e9f1b1d930c9be056f91d63258ceb6c79.
# Set RATATUI_DIFF_VIEWER to a built viewer from merged ratatui-diff main.
Set Shell "bash"
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
Type "exec \"$RATATUI_DIFF_VIEWER\" --aardvark-ink"
Enter
Wait+Screen@5s "timeout_ms: 1500"
Caption "Before: source diff, no selection"
Screenshot media/terminal-ux/diff-before.png
State media/terminal-ux/diff-before.json
# Cell coordinates are pinned to this fixture and geometry.
MouseMove 29 10
MouseDown Left
# Keep the drag active while the terminal grid changes.
Resize 50 16
AssertCells 29 8 '["1","5","0","0"]'
# The compact viewport omits two metadata rows; use the new source-cell position.
MouseMove 32 8
MouseUp Left
Type "c"
AssertText 'Text preview: "1500"'
Resize 92 25
AssertText 'Text preview: "1500"'
Caption "After: mouse drag selects new 1500; c previews exact source"
Screenshot media/terminal-ux/diff-after.png
State media/terminal-ux/diff-after.json
Resize 50 16
Wait+Screen@5s "Arrows select · c preview · Esc clear"
Caption "Resized: 50x16 grid retains the selected source and preview"
Screenshot media/terminal-ux/diff-resize-50x16.png
State media/terminal-ux/diff-resize-50x16.json
Resize 92 25
Wait+Screen@5s "Home/End"
Caption "Restored: 92x25 grid retains exact 1500 source selection"
Screenshot media/terminal-ux/diff-resize-restored.png
State media/terminal-ux/diff-resize-restored.json
Type "q"
