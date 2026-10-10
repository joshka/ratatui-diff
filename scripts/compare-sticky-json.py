"""Compare focused public terminal behavior; this is not whole-screen visual approval."""
import argparse
import json
from pathlib import Path

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("baseline", type=Path)
parser.add_argument("candidate", type=Path)
args = parser.parse_args()


def read(root, name):
    return json.loads((root / f"{name}.json").read_text())


def row(state, index):
    result = []
    for span in state["viewport"][index]:
        text, style = (span, {}) if isinstance(span, str) else (span[0], state["styles"][span[1]])
        effective = state["default_style"] | style
        result.extend((char, effective) for char in text)
    return result


for name in ("start", "scrolled", "split", "narrow", "boundary", "folded", "totals", "end"):
    before, after = read(args.baseline, name), read(args.candidate, name)
    assert before["size"] == after["size"], name
    assert before["scrollback_rows"] == after["scrollback_rows"] == 0, name
    if name in ("scrolled", "split", "narrow"):
        assert "src/routes.txt" not in before["viewport_text"], name
        assert "▾ src/routes.txt · +9 −3" in after["viewport_text"].splitlines()[3], name
        # The pinned row consumes one bottom row, without replacing the first scrolled source.
        for y in range(3, before["size"][1] - 4):
            assert row(before, y) == row(after, y + 1), (name, y)
    if name == "boundary":
        assert "▾ src/settings.txt · +9 −3" in after["viewport_text"].splitlines()[3]
    if name == "folded":
        assert "setting 24: revised" not in after["viewport_text"]
    if name == "totals":
        assert "3 files · +27 −9" in before["viewport_text"]
        assert "3 files · +27 −9" in after["viewport_text"]
    if name == "end":
        assert "check 47: ready" in before["viewport_text"]
        assert "check 47: ready" in after["viewport_text"]
    print(f"PASS {name}: geometry, scrollback, and focused content/style contract")
