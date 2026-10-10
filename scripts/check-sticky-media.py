"""Verify curated documentation assets are hydrated and match their recorded digests."""
import hashlib
import json
from pathlib import Path

root = Path(__file__).resolve().parent.parent / "docs/assets/sticky-headers"
manifest = json.loads((root / "media.json").read_text())
for name, expected in manifest.items():
    data = (root / name).read_bytes()
    magic = b"\x89PNG\r\n\x1a\n" if name.endswith(".png") else b"GIF89a"
    if not data.startswith(magic):
        raise SystemExit(f"{name}: expected hydrated media, found pointer or invalid bytes")
    if hashlib.sha256(data).hexdigest() != expected:
        raise SystemExit(f"{name}: media checksum mismatch")
print(f"Verified {len(manifest)} hydrated documentation images")
