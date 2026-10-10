"""Replay focused consumer tapes, retaining logs and Betamax failure artifacts."""

import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parent.parent
ARTIFACTS = ROOT / "media/terminal-ux"
TAPES = ("resize", "ux", "unicode", "multi-file", "folding")


def main():
    ARTIFACTS.mkdir(parents=True, exist_ok=True)
    betamax = os.environ.get("BETAMAX", "betamax")
    version = subprocess.check_output([betamax, "--version"], text=True).strip()
    expected = os.environ.get("BETAMAX_VERSION")
    if expected and version != f"betamax {expected}":
        raise SystemExit(f"Expected betamax {expected}; found {version}")
    revision = os.environ.get("BETAMAX_REVISION", "released tool or revision not supplied")
    (ARTIFACTS / "tool.txt").write_text(
        f"Executable: {betamax}\nVersion: {version}\nSource revision: {revision}\n"
    )
    env = os.environ.copy()
    env["BETAMAX_FAILURE_DIR"] = str(ARTIFACTS / "failures")
    print(f"Terminal UX: {betamax} ({version})", flush=True)

    failed = []
    for name in TAPES:
        tape = f"examples/terminal-ux-{name}.tape"
        command = [sys.executable, str(ROOT / "scripts/capture.py"), tape]
        with (ARTIFACTS / f"{name}.log").open("w") as log:
            result = subprocess.run(command, cwd=ROOT, env=env,
                                    stdout=log, stderr=subprocess.STDOUT)
        print(f"{name}: {'FAIL' if result.returncode else 'pass'}", flush=True)
        if result.returncode:
            failed.append(name)
    if failed:
        raise SystemExit(f"Failed tapes: {', '.join(failed)}. Inspect {ARTIFACTS}")
    subprocess.run([sys.executable, str(ROOT / "scripts/check-terminal-ux.py")],
                   cwd=ROOT, check=True)


if __name__ == "__main__":
    main()
