"""Run the checked-in tape without depending on the PTY shell's home directory."""
import argparse
import json
import os
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parent.parent
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("tape", nargs="?", default="examples/viewer.tape")
parser.add_argument("--viewer", type=Path, help="Use an explicit immutable viewer executable.")
arguments = parser.parse_args()
if arguments.viewer:
    viewer = arguments.viewer.resolve()
else:
    metadata = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"], cwd=root,
    ))
    viewer = Path(metadata["target_directory"]) / "debug" / "examples" / "viewer"
if not viewer.is_file():
    raise SystemExit(f"Missing viewer: {viewer}. Run cargo build --example viewer --locked.")
env = os.environ.copy()
env["RATATUI_DIFF_VIEWER"] = str(viewer)
tape = arguments.tape
betamax = env.get("BETAMAX", "betamax")
raise SystemExit(subprocess.run([betamax, "run", tape], cwd=root, env=env).returncode)
