"""Replay focused consumer tapes, retaining logs and Betamax failure artifacts."""

import argparse
import hashlib
import os
from pathlib import Path
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parent.parent
ARTIFACTS = ROOT / "media/terminal-ux"
TAPES = ("resize", "ux", "unicode", "multi-file", "folding", "files", "statistics", "alignment")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--case", action="append", choices=TAPES, dest="cases",
                        help="Replay only this case; repeat to select multiple cases.")
    parser.add_argument("--viewer", type=Path, help="Use an explicit immutable viewer executable.")
    parser.add_argument("--viewer-revision", help="Record the explicit viewer's source revision.")
    arguments = parser.parse_args()
    cases = arguments.cases or TAPES
    viewer = arguments.viewer.resolve() if arguments.viewer else None
    ARTIFACTS.mkdir(parents=True, exist_ok=True)
    betamax = os.environ.get("BETAMAX", "betamax")
    version = subprocess.check_output([betamax, "--version"], text=True).strip()
    revision = os.environ.get("BETAMAX_REVISION", "released tool or revision not supplied")
    executable = Path(shutil.which(betamax) or betamax).resolve()
    digest = hashlib.sha256(executable.read_bytes()).hexdigest()
    (ARTIFACTS / "tool.txt").write_text(
        f"Executable: {executable}\nVersion: {version}\nSource revision: {revision}\n"
        f"Executable SHA-256: {digest}\n"
    )
    expected = os.environ.get("BETAMAX_VERSION")
    if expected and version != f"betamax {expected}":
        raise SystemExit(f"Expected betamax {expected}; found {version}")
    env = os.environ.copy()
    env["BETAMAX_FAILURE_DIR"] = str(ARTIFACTS / "failures")
    print(f"Terminal UX: {betamax} ({version})", flush=True)

    if viewer:
        viewer_digest = hashlib.sha256(viewer.read_bytes()).hexdigest()
        (ARTIFACTS / "viewer.txt").write_text(
            f"Executable: {viewer}\nSource revision: {arguments.viewer_revision or 'not supplied'}\n"
            f"Executable SHA-256: {viewer_digest}\n"
        )

    failed = []
    for name in cases:
        tape = f"examples/terminal-ux-{name}.tape"
        command = [sys.executable, str(ROOT / "scripts/capture.py"), tape]
        if viewer:
            command.extend(["--viewer", str(viewer)])
        with (ARTIFACTS / f"{name}.log").open("w") as log:
            result = subprocess.run(command, cwd=ROOT, env=env,
                                    stdout=log, stderr=subprocess.STDOUT)
        print(f"{name}: {'FAIL' if result.returncode else 'pass'}", flush=True)
        if result.returncode:
            failed.append(name)
    source_cases = {"resize", "unicode", "multi-file"}
    if source_cases.issubset(cases) and source_cases.isdisjoint(failed):
        with (ARTIFACTS / "exact-source.log").open("w") as log:
            result = subprocess.run(
                [sys.executable, str(ROOT / "scripts/check-terminal-ux.py")],
                cwd=ROOT, stdout=log, stderr=subprocess.STDOUT,
            )
        print((ARTIFACTS / "exact-source.log").read_text(), end="", flush=True)
        if result.returncode:
            failed.append("exact-source")
    if failed:
        raise SystemExit(f"Failed checks: {', '.join(failed)}. Inspect {ARTIFACTS}")


if __name__ == "__main__":
    main()
