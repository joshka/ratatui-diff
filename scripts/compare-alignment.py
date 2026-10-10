"""Small deterministic pairing experiment; production behavior is covered by Rust tests.

Compare anchor choices, excluding similarity cost/timing. Run from any directory with Python 3.
Indexes in output are one-based. Gap completion is source order in both candidate approaches.
"""
from collections import Counter


def similarity(old, new):
    def grams(text):
        text = text.strip()
        return Counter(zip("\0" + text, text))

    left, right = grams(old), grams(new)
    total = sum(left.values()) + sum(right.values())
    return 2000 * sum((left & right).values()) // total if total else 1000


def fill_gaps(old, new, anchors):
    pairs = []
    a = b = 0
    for x, y in anchors + [(len(old), len(new))]:
        for offset in range(max(x - a, y - b)):
            pairs.append((a + offset + 1 if a + offset < x else None,
                          b + offset + 1 if b + offset < y else None))
        if x < len(old):
            pairs.append((x + 1, y + 1))
        a, b = x + 1, y + 1
    return pairs


def greedy(old, new):
    if not bounded(old, new):
        return fill_gaps(old, new, [])
    anchors = []
    start = 0
    for a, line in enumerate(old):
        candidates = [(similarity(line, new[b]), -b) for b in range(start, len(new))]
        if candidates:
            score, negative_b = max(candidates)
            if score > 500:
                b = -negative_b
                anchors.append((a, b))
                start = b + 1
    return fill_gaps(old, new, anchors)


def bounded(old, new):
    run_bytes = sum(len(line.encode()) for line in old + new)
    return len(old) + len(new) <= 256 and run_bytes <= 32768 and len(old) * len(new) * run_bytes <= 1048576


def monotonic(old, new):
    if not bounded(old, new):
        return fill_gaps(old, new, [])
    scores = [[0] * (len(new) + 1) for _ in range(len(old) + 1)]
    choices = {}
    for a in reversed(range(len(old))):
        for b in reversed(range(len(new))):
            reward = max(0, similarity(old[a], new[b]) - 500)
            down, right = scores[a + 1][b], scores[a][b + 1]
            paired = scores[a + 1][b + 1] + reward
            if reward and paired >= max(down, right):
                scores[a][b], choices[a, b] = paired, "pair"
            elif down >= right:
                scores[a][b], choices[a, b] = down, "old"
            else:
                scores[a][b], choices[a, b] = right, "new"
    a = b = 0
    anchors = []
    while a < len(old) and b < len(new):
        choice = choices[a, b]
        if choice == "pair":
            anchors.append((a, b))
            a, b = a + 1, b + 1
        elif choice == "old":
            a += 1
        else:
            b += 1
    return fill_gaps(old, new, anchors)


cases = [
    ("inserted comment", ["timeout_ms: 1000", "retries: 2", "endpoint: /api/v1", "label: café 界"],
     ["# Audit requests before sending", "timeout_ms: 2500", "retries: 4", "endpoint: /api/v2", "label: café 世界"],
     [(1, 2), (2, 3), (3, 4), (4, 5)]),
    ("uneven removal", ["header: old", "removed extra", "tail: old"], ["header: new", "tail: new"],
     [(1, 1), (3, 2)]),
    ("repeated lines", ["repeat old", "repeat old"], ["unrelated", "repeat new", "repeat new"],
     [(1, 2), (2, 3)]),
    ("Unicode", ["label: café 界 👩‍💻 old"], ["unrelated", "label: café 世界 👩‍💻 new"], [(1, 2)]),
    ("greedy trap", ["rule: retain source context for every request alpha", "rule: retain source context for every request beta"],
     ["rule: retain source context for every request changed alpha", "rule: retain source context for every request alpha"],
     [(1, 1), (2, 2)]),
    ("dissimilar", ["aaa", "bbb"], ["xxx", "yyy", "zzz"], [(1, 1), (2, 2)]),
    ("long fallback", ["界" * 6000, "setting: old"], ["unrelated", "界" * 6000, "setting: new"], [(1, 1), (2, 2)]),
]

print("case | source order | greedy | monotonic")
for name, old, new, expected in cases:
    results = [fill_gaps(old, new, []), greedy(old, new), monotonic(old, new)]
    counts = [f"{sum(pair in result for pair in expected)}/{len(expected)}" for result in results]
    print(f"{name} | {' | '.join(counts)}")
