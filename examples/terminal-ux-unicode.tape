# Adapted from Betamax PR173 at 9a7a34597ae08f428b8363269571325e87d0984f.
# Unicode selection must retain source newlines while display lines wrap.
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
Type "exec \"$RATATUI_DIFF_VIEWER\" --fixture unicode --aardvark-ink"
Enter
Wait+Screen@5s "café"
Resize 50 16
Type "w"
Type "v"
Home
End
Type "c"
Resize 250 25
State media/terminal-ux/unicode-title.json
Resize 50 16
Down
End
Type "c"
Down
End
Type "c"
Resize 250 25
State media/terminal-ux/unicode-selection.json
Resize 50 16
AssertText "Text preview:"
Caption "Unicode source selection across wrapped display lines"
Screenshot media/terminal-ux/unicode-selection.png
Type "q"
