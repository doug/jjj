"""The zlib inflate cost the Rust spike omits.

The spike reads plain JSON files. If packs are stored as git objects, every read
inflates them first, and that cost is not in any of the spike's three layers.
This measures inflate alone over the same pack sets so the omission is a number
rather than a hand-wave. Python's zlib is the same C library the Rust stack
binds, and per-call interpreter overhead (~1-2 us) is the error bar.

    python3 -I tools/spike-oplog/inflate_cost.py tools/spike-oplog/target/spike-out
"""

import glob
import os
import sys
import time
import zlib

root = sys.argv[1] if len(sys.argv) > 1 else "target/spike-out"
for d in sorted(glob.glob(os.path.join(root, "*-nested"))):
    files = sorted(glob.glob(os.path.join(d, "*.json")))
    if not files:
        continue
    # Level 6: git's default.
    blobs = [zlib.compress(open(f, "rb").read(), 6) for f in files]
    times = []
    for _ in range(60):
        t = time.perf_counter_ns()
        for b in blobs:
            zlib.decompress(b)
        times.append(time.perf_counter_ns() - t)
    times.sort()
    p50 = times[len(times) // 2] / 1e6
    p99 = times[min(len(times) - 1, int(len(times) * 0.99))] / 1e6
    print(
        f"{os.path.basename(d):26s} packs={len(files):5d} "
        f"inflate_p50={p50:8.3f}ms inflate_p99={p99:8.3f}ms"
    )
