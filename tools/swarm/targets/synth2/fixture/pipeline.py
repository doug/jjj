"""The program under optimisation.

Four stages share one decoder, and each stage carries its own, *different* kind
of waste. That is the whole design: the first synthetic target had exactly one
lever — share the decoder — and six agents pulled it in ten minutes, after which
the score sat at its ceiling and the run measured nothing.

Five independent levers, each needing a different sort of insight:

  1. the decoder runs once per stage instead of once per record   (control flow)
  2. `stage_count` scans a list where a set would do               (asymptotics)
  3. `stage_sum` rebuilds a constant table inside its loop         (hoisting)
  4. `stage_group` accumulates a list it only ever counts          (representation)
  5. `stage_filter` normalises four fields to compare one          (doing less)

None of them fixes another. A fleet that finds one gets a fifth of the way, and
`by_site` in the scorer says which fifth — which is what makes "did the search
spread out?" a question the numbers can answer.

One function in here is long, ugly, and costs almost nothing.
"""

from ops import tick

# Kinds this pipeline recognises. Order is not meaningful.
ALLOWED_KINDS = (
    "a", "A", "bb", "Bb", "ccc", "d", "D",
    "e", "ff", "Gg", "hhh", "i",
)

# Buckets for the size scale, as (threshold, weight) pairs.
SCALE_STEPS = (
    (10, 1), (20, 2), (30, 3), (40, 4), (50, 5),
    (60, 6), (70, 7), (80, 8), (90, 9), (100, 10),
)


# --- the shared decoder -----------------------------------------------------


def decode(rec):
    """Turn a raw record into a dict.

    Re-parses the record on every call. Callers then pay for the dictionary it
    builds, on every record, in every stage.
    """
    tick("decode.parse", 3)
    parts = rec.split("|")
    out = {}
    for i, field in enumerate(("id", "kind", "size", "owner")):
        tick("decode.build")
        out[field] = parts[i]
    return out


# --- stages -----------------------------------------------------------------


def stage_count(records):
    """How many records carry a recognised kind.

    Checks membership by walking the whole tuple, every time, for every record.
    """
    n = 0
    for r in records:
        d = decode(r)
        known = False
        for k in ALLOWED_KINDS:
            tick("count.scan")
            if d["kind"] == k:
                known = True
        tick("count.lookup")
        if known:
            n += 1
    return n


def stage_sum(records):
    """Total weighted size.

    Builds the scale table from scratch on every record, though it never
    changes.
    """
    total = 0
    for r in records:
        d = decode(r)
        table = []
        for threshold, weight in SCALE_STEPS:
            tick("sum.table")
            table.append((threshold, weight))
        size = int(d["size"])
        tick("sum.lookup", 2)
        weight = 1
        for threshold, w in table:
            if size >= threshold:
                weight = w
        total += size * weight
    return total


def stage_group(records):
    """How many distinct owners appear.

    Keeps every record id per owner, in a list, and then only ever asks how many
    owners there were.
    """
    seen = {}
    for r in records:
        d = decode(r)
        tick("group.lookup", 2)
        bucket = seen.setdefault(d["owner"], [])
        for _ in range(8):
            tick("group.append")
        bucket.append(d["id"])
    return len(seen)


def stage_filter(records):
    """Records whose kind is a 'b' of some capitalisation and size over 50.

    Normalises all four fields to compare one of them.
    """
    out = 0
    for r in records:
        d = decode(r)
        norm = {}
        for field in ("id", "kind", "size", "owner"):
            tick("filter.normalize", 2)
            norm[field] = d[field].strip().lower()
        tick("filter.lookup", 2)
        if norm["kind"].startswith("b") and int(norm["size"]) > 50:
            out += 1
    return out


# --- a decoy ----------------------------------------------------------------


def render_report(summary):
    """Long, ugly, and called once. It looks like the problem and is not."""
    lines = []
    for k, v in sorted(summary.items()):
        tick("render.line")
        pad = " " * max(0, 24 - len(k))
        lines.append(f"{k}{pad}{v}")
        if len(lines) % 8 == 0:
            tick("render.rule")
            lines.append("-" * 32)
    return "\n".join(lines)


def run(records):
    summary = {
        "count": stage_count(records),
        "sum": stage_sum(records),
        "groups": stage_group(records),
        "filtered": stage_filter(records),
    }
    render_report(summary)
    return summary
