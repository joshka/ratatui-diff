"""Run the checked-in tape without depending on the PTY shell's home directory."""
import json
import os
from pathlib import Path
import subprocess
import sys

root = Path(__file__).resolve().parent.parent
metadata = json.loads(subprocess.check_output(
    ["cargo", "metadata", "--no-deps", "--format-version", "1"], cwd=root,
))
viewer = Path(metadata["target_directory"]) / "debug" / "examples" / "viewer"
if not viewer.is_file():
    raise SystemExit(f"Missing viewer: {viewer}. Run cargo build --example viewer --locked.")
env = os.environ.copy()
env["RATATUI_DIFF_VIEWER"] = str(viewer)
tape = sys.argv[1] if len(sys.argv) > 1 else "examples/viewer.tape"
betamax = env.get("BETAMAX", "betamax")
raise SystemExit(subprocess.run([betamax, "run", tape], cwd=root, env=env).returncode)
