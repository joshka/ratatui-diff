"""Compile and run every Rust code block in the documentation site against this crate.

Blocks without their own `use ratatui_diff` continue the page's first importing block, matching how
the guides introduce a document once and then show further builder calls.
"""
from pathlib import Path
import os
import re
import subprocess

root = Path(__file__).resolve().parent.parent
docs = root / "site" / "src" / "content" / "docs"
crate = root / "target" / "site-snippets"

functions = []
for page in sorted(docs.rglob("*.md*")):
    first = None
    for block in re.findall(r"```rust\n(.*?)```", page.read_text(), re.S):
        if first is None and "use ratatui_diff" in block:
            first = block
        body = block if "use ratatui_diff" in block else f"{first or ''}\n{block}"
        functions.append((page.relative_to(docs), body))

(crate / "src").mkdir(parents=True, exist_ok=True)
(crate / "Cargo.toml").write_text(f"""[package]
name = "site-snippets"
version = "0.0.0"
edition = "2024"
publish = false

[workspace]

[dependencies]
ratatui-diff = {{ path = {str(root)!r}, features = ["syntax"] }}
ratatui-core = "0.1"
""")
source = ["#![allow(unused)]"]
calls = []
for index, (page, body) in enumerate(functions):
    source.append(
        f"// {page}\nfn snippet_{index}() -> Result<(), Box<dyn std::error::Error>> {{\n"
        f"{body}\nOk(())\n}}"
    )
    calls.append(f'    snippet_{index}().expect("{page}");')
source.append("fn main() {\n" + "\n".join(calls) + "\n}")
(crate / "src" / "main.rs").write_text("\n".join(source) + "\n")

subprocess.run(
    ["cargo", "run", "--quiet", "--manifest-path", str(crate / "Cargo.toml")],
    cwd=root,
    env={**os.environ, "CARGO_TARGET_DIR": str(root / "target")},
    check=True,
)
print(f"{len(functions)} site snippets compiled and ran")
