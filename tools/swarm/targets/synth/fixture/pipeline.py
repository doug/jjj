"""The program under optimisation.

Four stages share one decoder. The decoder's own line in a profile is modest;
what it costs is paid by its callers, once per record, every time. Making a
stage faster on its own is worth a little. Changing what the decoder hands back
is worth a great deal — and cannot be done without changing all four stages,
which is why it is a judgement call rather than a measurement.
"""

from ops import tick

# --- the shared decoder -----------------------------------------------------


def decode(rec):
    """Turn a raw record into a dict.

    Re-parses the key on every call. Callers then pay dictionary lookups for
    every field they touch, on every record, in every stage.
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
    n = 0
    for r in records:
        d = decode(r)
        tick("stage_count.lookup")
        if d["kind"] == "a":
            n += 1
    return n


def stage_sum(records):
    total = 0
    for r in records:
        d = decode(r)
        tick("stage_sum.lookup", 2)
        total += int(d["size"])
    return total


def stage_group(records):
    seen = {}
    for r in records:
        d = decode(r)
        tick("stage_group.lookup", 2)
        seen.setdefault(d["owner"], 0)
        seen[d["owner"]] += 1
    return len(seen)


def stage_filter(records):
    out = []
    for r in records:
        d = decode(r)
        tick("stage_filter.lookup", 2)
        if d["kind"] == "b" and int(d["size"]) > 50:
            out.append(d["id"])
    return len(out)


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
