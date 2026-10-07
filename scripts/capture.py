"""Run the checked-in tape without depending on the PTY shell's home directory."""
import json
import os
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parent.parent
metadata = json.loads(subprocess.check_output(
    ["cargo", "metadata", "--no-deps", "--format-version", "1"], cwd=root,
))
viewer = Path(metadata["target_directory"]) / "debug" / "examples" / "viewer"
env = os.environ.copy()
env["RATATUI_DIFF_VIEWER"] = str(viewer)
subprocess.run(["betamax", "run", "examples/viewer.tape"], cwd=root, env=env, check=True)
