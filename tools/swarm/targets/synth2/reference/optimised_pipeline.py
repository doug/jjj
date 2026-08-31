"""An aggressively optimised reference: all five levers pulled, single pass."""
from ops import tick

ALLOWED = frozenset(("a","A","bb","Bb","ccc","d","D","e","ff","Gg","hhh","i"))
STEPS = ((10,1),(20,2),(30,3),(40,4),(50,5),(60,6),(70,7),(80,8),(90,9),(100,10))

def render_report(summary):
    lines = []
    for k, v in sorted(summary.items()):
        tick("render.line")
        pad = " " * max(0, 24 - len(k))
        lines.append(f"{k}{pad}{v}")
        if len(lines) % 8 == 0:
            tick("render.rule"); lines.append("-" * 32)
    return "\n".join(lines)

def run(records):
    count = 0; total = 0; owners = set(); filtered = 0
    for r in records:
        tick("decode.parse", 3)
        _id, kind, size_s, owner = r.split("|")
        size = int(size_s)
        if kind in ALLOWED:
            count += 1
        weight = 1
        for threshold, w in STEPS:
            if size >= threshold:
                weight = w
        total += size * weight
        owners.add(owner)
        tick("filter.normalize", 2)
        if kind.strip().lower().startswith("b") and size > 50:
            filtered += 1
    summary = {"count": count, "sum": total, "groups": len(owners), "filtered": filtered}
    render_report(summary)
    return summary
