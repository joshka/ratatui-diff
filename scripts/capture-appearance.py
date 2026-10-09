"""Capture each widget at its measured row count, with no application chrome."""
import json
import os
from pathlib import Path
import re
import subprocess

root = Path(__file__).resolve().parent.parent
metadata = json.loads(subprocess.check_output(
    ["cargo", "metadata", "--no-deps", "--format-version", "1"], cwd=root,
))
viewer = Path(metadata["target_directory"]) / "debug" / "examples" / "viewer"
env = os.environ.copy()
env["RATATUI_DIFF_VIEWER"] = str(viewer)

# The pinned 22px JetBrains Mono capture uses 14px columns and 22px rows.
# Odd column counts fit two equal split panes and their one-column separator.
# Widths contain an exact number of cells plus the symmetric 22px image padding.
for template, columns, prefix in [
    ("aardvark-ink.tape", 101, ""),
    ("aardvark-ink-narrow.tape", 49, "narrow-"),
]:
    source = (root / "examples" / template).read_text()
    for variant in ["unified", "split", "lines-only", "no-numbers", "whitespace", "wrapped", "mono"]:
        rows = int(subprocess.check_output(
            [str(viewer), "--measure", variant, str(columns)], cwd=root, text=True,
        ))
        tape = re.sub(r"Set Height \d+", f"Set Height {rows * 22 + 44}", source)
        tape = tape.replace("--capture unified", f"--capture {variant}")
        tape = tape.replace(f"aardvark-{prefix}unified.png", f"aardvark-{prefix}{variant}.png")
        # All variants display the file header before their first source row.
        tape = tape.replace("/timeout_ms:/", "/src\\/client.rs/")
        path = root / "media" / f"capture-{prefix}{variant}.tape"
        path.write_text(tape)
        subprocess.run(["betamax", "run", str(path)], cwd=root, env=env, check=True)
