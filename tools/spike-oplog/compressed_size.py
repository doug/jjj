"""Compressed size of a synthesised pack set, per blob.

The spike reports raw JSON bytes. git stores each pack as its own zlib blob, so
raw bytes overstate what a clone fetches. Measured separately because the Rust
side would need a compression dependency to do it and this spike is throwaway.

    python3 -I tools/spike-oplog/compressed_size.py tools/spike-oplog/target/spike-out

Caveat on the numbers this produces: the synthetic bodies are drawn from a
78-word vocabulary, so they compress somewhat better than real prose.
"""

import glob
import gzip
import os
import sys

root = sys.argv[1] if len(sys.argv) > 1 else "target/spike-out"
dirs = sorted(
    glob.glob(os.path.join(root, "*-nested")) + glob.glob(os.path.join(root, "*-flat"))
)
for d in dirs:
    files = sorted(glob.glob(os.path.join(d, "*.json")))
    if not files:
        continue
    raw = 0
    comp = 0
    for f in files:
        b = open(f, "rb").read()
        raw += len(b)
        comp += len(gzip.compress(b, 9))
    print(
        f"{os.path.basename(d):26s} packs={len(files):5d} raw={raw:10d} "
        f"zlib-per-blob={comp:10d} ratio={raw / comp:.2f}"
    )
