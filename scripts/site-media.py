"""Capture documentation-site images from checked-in tapes and stage them in site/public/assets.

The site never commits generated images: every figure is replayed from `examples/*.tape` against the
viewer built from the same revision, so screenshots match the code that the site documents. Sticky
header comparisons are reviewed evidence from an earlier revision pair and are copied from Git LFS.
"""
import argparse
import os
from pathlib import Path
import subprocess
import sys

root = Path(__file__).resolve().parent.parent
media = root / "media"
assets = root / "site" / "public" / "assets"

# Tapes beyond the appearance set and README demo, each writing named files into media/.
TAPES = [
    "minimal",
    "files-expanded",
    "files-collapsed",
    "statistics-after",
    "context-split-v2",
    "search-wide",
    "selection-wrapped",
    "hit-testing",
    "fixtures-unicode",
    # Requires the viewer built with `--features syntax`.
    "syntax-raw-split",
    "syntax-after-split",
    "syntax-after-unified",
]

# Site name -> source path relative to the repository root.
FIGURES = {
    "minimal-unified.png": "media/minimal-unified.png",
    "minimal-split.png": "media/minimal-split.png",
    "unified.png": "media/aardvark-unified.png",
    "split.png": "media/aardvark-split.png",
    "lines-only.png": "media/aardvark-lines-only.png",
    "no-numbers.png": "media/aardvark-no-numbers.png",
    "whitespace.png": "media/aardvark-whitespace.png",
    "mono.png": "media/aardvark-mono.png",
    "narrow-split.png": "media/aardvark-narrow-split.png",
    "narrow-wrapped.png": "media/aardvark-narrow-wrapped.png",
    "viewer.gif": "media/viewer.gif",
    "files-expanded.png": "media/files-expanded-v1.png",
    "files-collapsed.png": "media/files-collapsed-v1.png",
    "files-stats.png": "media/statistics-after-20261009.png",
    "context-folds.png": "media/context-split-v2.png",
    "search-before.png": "media/search-host-baseline-wide-20261009-v2.png",
    "search-matches.png": "media/search-host-after-wide-20261009-v2.png",
    "search-next.png": "media/search-host-next-wide-20261009-v2.png",
    "selection-wrapped.png": "media/selection-wrapped-after-01.png",
    "hit-testing.png": "media/hit-after-02.png",
    "unicode-unwrapped.png": "media/fixtures-unicode-split-before-wrap-compact.png",
    "unicode-wrapped.png": "media/fixtures-unicode-split-after-wrap-compact.png",
    "syntax-off.png": "media/syntax-raw-split-v1.png",
    "syntax-split.png": "media/syntax-after-split-v1.png",
    "syntax-unified.png": "media/syntax-after-unified-v1.png",
    "sticky-before.png": "docs/assets/sticky-headers/before-scrolled.png",
    "sticky-after.png": "docs/assets/sticky-headers/after-scrolled.png",
    "sticky-split.png": "docs/assets/sticky-headers/after-split.png",
    "sticky-comparison.gif": "docs/assets/sticky-headers/comparison.gif",
}

LFS_POINTER = b"version https://git-lfs.github.com/spec/v1"
SIGNATURES = {".png": b"\x89PNG\r\n\x1a\n", ".gif": b"GIF8"}


def run(*command: str, env: dict[str, str] | None = None) -> None:
    print("+", " ".join(command), flush=True)
    subprocess.run(command, cwd=root, env=env, check=True)


def capture() -> None:
    media.mkdir(exist_ok=True)
    run("cargo", "build", "--example", "viewer", "--example", "minimal", "--features", "syntax",
        "--locked")
    examples = root / "target" / "debug" / "examples"
    env = os.environ.copy()
    # Tapes start in the PTY shell's directory, so executables are passed by absolute path.
    env["RATATUI_DIFF_VIEWER"] = str(examples / "viewer")
    env["RATATUI_DIFF_MINIMAL"] = str(examples / "minimal")
    run(sys.executable, "scripts/capture-appearance.py", env=env)
    run(sys.executable, "scripts/capture.py", env=env)
    for tape in TAPES:
        run(sys.executable, "scripts/capture.py", f"examples/{tape}.tape", env=env)


def stage() -> None:
    assets.mkdir(parents=True, exist_ok=True)
    problems = []
    for name, source in FIGURES.items():
        path = root / source
        if not path.is_file():
            problems.append(f"missing {source}")
            continue
        data = path.read_bytes()
        if data.startswith(LFS_POINTER):
            # Smudge into the ignored output rather than hydrating the tracked file: jj would
            # otherwise snapshot the raw image in place of its pointer.
            data = subprocess.run(
                ["git", "lfs", "smudge", source], cwd=root, input=data, capture_output=True,
                check=True,
            ).stdout
        # Reject unfetched LFS objects and other non-image bytes before they reach the site.
        if not data.startswith(SIGNATURES[path.suffix]):
            problems.append(f"{source} is not a {path.suffix} image")
            continue
        (assets / name).write_bytes(data)
    if problems:
        raise SystemExit("\n".join(problems))
    print(f"staged {len(FIGURES)} figures in {assets.relative_to(root)}")


def check(directory: Path) -> None:
    """Verify every figure in a built site still carries image bytes."""
    problems = []
    for name in FIGURES:
        path = directory / "assets" / name
        if not path.is_file():
            problems.append(f"missing {path}")
        elif not path.read_bytes().startswith(SIGNATURES[path.suffix]):
            problems.append(f"{path} is not a {path.suffix} image")
    if problems:
        raise SystemExit("\n".join(problems))
    print(f"verified {len(FIGURES)} figures in {directory}")


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument(
    "--no-capture", action="store_true", help="Stage existing media/ files without replaying tapes."
)
parser.add_argument("--check", type=Path, metavar="DIR", help="Only verify figures in a built site.")
arguments = parser.parse_args()
if arguments.check:
    check(arguments.check)
else:
    if not arguments.no_capture:
        capture()
    stage()
