"""Replay the sticky-header scenario with immutable inputs and retained diagnostics.

The checked-in tape is directly runnable for CI. This small adapter changes only
baseline expectations, artifact paths, captions, and review pauses. It does not
maintain a second interaction sequence.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parent.parent


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--viewer", required=True, type=Path)
    parser.add_argument("--revision", required=True)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--baseline", action="store_true")
    parser.add_argument("--paced", action="store_true")
    parser.add_argument("--comparison-baseline", type=Path,
                        help="Prepend the baseline start/scroll/split replay to a paced comparison")
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    if (output / "manifest.json").exists():
        raise SystemExit("Use a fresh output directory to preserve earlier evidence")
    viewer = args.viewer.resolve()
    betamax = Path(shutil.which(os.environ.get("BETAMAX", "betamax"))).resolve()
    version = subprocess.check_output([str(betamax), "--version"], text=True).strip()
    if version != "betamax 0.1.22":
        raise SystemExit(f"Expected released betamax 0.1.22, found {version}")
    source = ROOT / "examples/terminal-ux-sticky.tape"
    lines = []
    candidate = False
    for line in source.read_text().splitlines():
        if line == "# Candidate begin":
            candidate = True
        elif line == "# Candidate end":
            candidate = False
        elif line.startswith("# Baseline: "):
            if args.baseline:
                lines.append(line.removeprefix("# Baseline: "))
        elif not (candidate and args.baseline):
            if args.baseline:
                line = line.replace(" --sticky-headers", "")
                if line.startswith("Caption "):
                    line = line.replace("AFTER |", "BEFORE |")
                    line = line.replace("Opt in with .sticky_file_headers(true)", "Headers scroll with source content")
                    line = line.replace("the path and +9 / -3 remain visible", "the path and file counts are lost")
                    line = line.replace("keeps one header above both source panes", "also loses the file header")
                    line = line.replace("retains file identity", "has no pinned file identity")
                    line = line.replace("the pinned path changes", "the file header remains off screen")
                    line = line.replace("Click the pinned header to fold its file", "Enter folds the current file")
            line = line.replace("media/terminal-ux/sticky/", str(output) + "/")
            if args.paced and line == "Sleep 250ms" and lines[-1].startswith("State "):
                line = "Sleep 2500ms"
            lines.append(line)
    if args.comparison_baseline:
        if not args.paced or args.baseline:
            parser.error("--comparison-baseline requires --paced and a candidate viewer")
        # Keep the comparison readable: the same first three checkpoints, before then after.
        # Full acceptance still covers every checkpoint in the canonical tape.
        first = [line for line in lines[:lines.index('Type "w"')]
                 if not line.startswith("Set ")]
        before_source = output.parent / (output.name + "-baseline")
        subprocess.run([
            os.sys.executable, __file__, "--viewer", str(args.comparison_baseline.resolve()),
            "--revision", "98707e6d", "--output", str(before_source), "--baseline",
        ], check=True)
        before = (before_source / "scenario.tape").read_text().splitlines()
        before = before[:before.index('Type "w"')]
        before = [line.replace("exec ", "").replace('\\"$RATATUI_DIFF_VIEWER\\"',
                  '\\"' + str(args.comparison_baseline.resolve()) + '\\"')
                  .replace("Sleep 250ms", "Sleep 2500ms" if index > 0 and before[index - 1].startswith("State ") else "Sleep 250ms")
                  .replace(str(before_source) + "/", str(output / "before") + "/")
                  for index, line in enumerate(before)]
        lines = before + ['Hide', 'Type "q"', 'Sleep 500ms'] + first + ['Hide', 'Type "q"']
    if args.paced:
        lines.insert(0, f"Output {output / 'replay.gif'}")
    tape = output / "scenario.tape"
    tape.write_text("\n".join(lines) + "\n")
    manifest = {
        "source_revision": args.revision, "viewer": str(viewer), "viewer_sha256": sha256(viewer),
        "betamax": str(betamax), "betamax_version": version, "betamax_sha256": sha256(betamax),
        "betamax_revision": "5650cb1ebcf7251b34ee9e9f70bfe7bb3f3242b2",
        "fixture": "multi-file", "fixture_sha256": sha256(ROOT / "examples/viewer_fixtures/mod.rs"),
        "canonical_tape_sha256": sha256(source), "generated_tape_sha256": sha256(tape),
        "initial_cells": [92, 25], "font": "JetBrains Mono", "font_size": 24,
        "theme": "Aardvark Ink", "baseline": args.baseline, "paced": args.paced,
        "comparison_baseline": str(args.comparison_baseline) if args.comparison_baseline else None,
        "comparison_baseline_sha256": sha256(args.comparison_baseline) if args.comparison_baseline else None,
    }
    (output / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    env = dict(os.environ, RATATUI_DIFF_VIEWER=str(viewer), BETAMAX_FAILURE_DIR=str(output / "failures"))
    with (output / "replay.log").open("w") as log:
        result = subprocess.run([str(betamax), "run", str(tape)], cwd=ROOT, env=env,
                                stdout=log, stderr=subprocess.STDOUT)
    (output / "checksums.txt").write_text("".join(
        f"{sha256(path)}  {path.relative_to(output)}\n"
        for path in sorted(output.rglob("*")) if path.is_file() and path.name != "checksums.txt"
    ))
    print(f"{'PASS' if result.returncode == 0 else 'FAIL'}: {output}")
    raise SystemExit(result.returncode)


if __name__ == "__main__":
    main()
