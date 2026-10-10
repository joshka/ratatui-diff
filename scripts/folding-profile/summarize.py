"""Count outer native symbols from samply's saved presymbolication sidecars.

Inline frames in a symbol-table entry describe one representative address, not every
known address. Do not treat them as accurate per-sample inline call stacks.
"""
import argparse
from collections import Counter
import gzip
import json
from pathlib import Path

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("profile", type=Path)
parser.add_argument("--start-ms", type=float, default=0)
parser.add_argument("--end-ms", type=float, default=float("inf"))
args = parser.parse_args()
with gzip.open(args.profile) as file:
    profile = json.load(file)
sidecar = args.profile.with_suffix(".syms.json")
symbols = json.loads(sidecar.read_text())
lookup = {}
for library in symbols["data"]:
    lookup[library["debug_name"]] = {
        address: symbols["string_table"][library["symbol_table"][index]["symbol"]]
        for address, index in library["known_addresses"]
    }
inclusive = Counter()
leaf = Counter()
total = 0
for thread in profile["threads"]:
    for time, stack in zip(thread["samples"]["time"], thread["samples"]["stack"]):
        if not args.start_ms <= time <= args.end_ms:
            continue
        if stack is None:
            continue
        total += 1
        names = []
        while stack is not None:
            frame = thread["stackTable"]["frame"][stack]
            function = thread["frameTable"]["func"][frame]
            resource = thread["funcTable"]["resource"][function]
            if resource >= 0:
                library = profile["libs"][thread["resourceTable"]["lib"][resource]]["debugName"]
                address = thread["frameTable"]["address"][frame]
                name = lookup.get(library, {}).get(address)
                if name:
                    names.append(name)
            stack = thread["stackTable"]["prefix"][stack]
        inclusive.update(set(names))
        if names:
            leaf[names[0]] += 1
print(f"Samples: {total}; range {args.start_ms}–{args.end_ms} ms; outer native symbols only")
for title, counts in [("Inclusive", inclusive), ("Leaf", leaf)]:
    print(title)
    for name, count in counts.most_common(25):
        print(f"{count:6} {100 * count / total:5.1f}% {name}")
