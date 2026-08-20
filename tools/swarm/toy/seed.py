#!/usr/bin/env python3
"""Generate the toy workbench a swarm trial runs against.

The toy exists to force collisions, not to be interesting. Its shape is chosen
so that agents contend on all four surfaces at once:

  1. **A shared registry file.** Every operation must be registered in
     `opkit/registry.py`, so two agents finishing at the same time edit the same
     lines. This is the jj/git-level conflict.
  2. **A shared conformance table.** Cases live in one `tests/cases.py`, for the
     same reason.
  3. **Fewer problems than agents.** `--problems` below the agent count
     guarantees two agents pick the same top item, exercising the fact that
     `jjj next --claim` is advisory rather than a lock.
  4. **Mandatory cross-critique.** Agents critique each other's solutions and
     reply on the same critique, which produces divergent edits to one entity
     body — the jjj-level merge conflict.

Fitness is the number of conformance cases passing, which is a *count*. A
wall-clock score would be meaningless here: the swarm saturates the machine, so
it would be measuring its own contention rather than the code.

The operations are deliberately dull and unambiguous (reverse a string, roman
numerals, Levenshtein). Nobody should be debugging the domain; the domain is
scaffolding for the coordination we actually care about.
"""

import argparse
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

# Each op: (name, one-line spec, conformance cases as (input, expected)).
# Chosen so a competent agent implements one in a single pass — the difficulty
# of the trial must come from coordination, never from the puzzle.
OPS = [
    (
        "reverse",
        "Reverse a string.",
        [("abc", "cba"), ("", ""), ("racecar", "racecar"), ("a b", "b a")],
    ),
    (
        "rot13",
        "Apply ROT13 to ASCII letters; leave everything else untouched.",
        [("abc", "nop"), ("Hello, World!", "Uryyb, Jbeyq!"), ("", ""), ("123", "123")],
    ),
    (
        "roman",
        "Convert a positive integer (as a decimal string) to Roman numerals.",
        [("1", "I"), ("4", "IV"), ("1994", "MCMXCIV"), ("3999", "MMMCMXCIX")],
    ),
    (
        "unroman",
        "Convert a Roman numeral to its decimal value (as a string).",
        [("I", "1"), ("IV", "4"), ("MCMXCIV", "1994"), ("MMMCMXCIX", "3999")],
    ),
    (
        "levenshtein",
        "Edit distance between two strings separated by a single comma.",
        [("kitten,sitting", "3"), ("a,a", "0"), (",abc", "3"), ("flaw,lawn", "2")],
    ),
    (
        "wordwrap",
        "Wrap text to 20 columns on spaces; input is 'width|text'; join lines with \\n.",
        [
            ("5|aa bb cc", "aa bb\ncc"),
            ("10|hello", "hello"),
            ("3|a b c", "a b\nc"),
        ],
    ),
    (
        "base36",
        "Encode a non-negative decimal integer string as lowercase base-36.",
        [("0", "0"), ("35", "z"), ("36", "10"), ("1295", "zz")],
    ),
    (
        "runlength",
        "Run-length encode: 'aaab' -> 'a3b1'. Counts always written, even for 1.",
        [("aaab", "a3b1"), ("", ""), ("abc", "a1b1c1"), ("aa", "a2")],
    ),
]

REGISTRY_TEMPLATE = '''"""Operation registry.

Every operation must be registered here. This file is a **deliberate
contention point** for the swarm trial: agents finishing different operations at
the same time both edit it, which is exactly the conflict we want to observe.

Keep entries in alphabetical order so a conflict resolves predictably.
"""

from typing import Callable, Dict

# name -> callable(str) -> str
REGISTRY: Dict[str, Callable[[str], str]] = {}


def register(name: str, fn: Callable[[str], str]) -> None:
    """Register an operation under `name`."""
    REGISTRY[name] = fn


def apply(name: str, value: str) -> str:
    """Apply a registered operation, or raise KeyError with a useful message."""
    if name not in REGISTRY:
        raise KeyError(f"no operation named {name!r}; have {sorted(REGISTRY)}")
    return REGISTRY[name](value)


# --- registrations ---------------------------------------------------------
# Agents append their import + register() call below, alphabetically.
'''

OP_STUB = '''"""{name}: {spec}

NOT IMPLEMENTED. Implement `run` and register it in opkit/registry.py.
"""


def run(value: str) -> str:
    raise NotImplementedError("{name} is not implemented yet")
'''

SCORER = '''#!/usr/bin/env python3
"""Conformance runner — this is the fitness function.

Pure standard library on purpose. A swarm runs unattended for days; a scorer
that needs `pip install` is one more thing that can fail at 3am, and the score
is the one signal the whole trial depends on.

Prints "<passing> <total>" and always exits 0, so a caller can parse it even
when everything is broken. `--verbose` lists each failure for an agent to act on.
"""

import importlib
import sys
import traceback
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))


def main() -> None:
    verbose = "--verbose" in sys.argv or "-v" in sys.argv

    try:
        from tests.cases import CASES
    except Exception as exc:  # a broken shared file must score 0, not crash
        print("0 0")
        if verbose:
            print(f"cannot load tests/cases.py: {exc}", file=sys.stderr)
        return

    # Re-import rather than trusting a stale module: agents edit these files
    # underneath us constantly.
    try:
        registry = importlib.import_module("opkit.registry")
        importlib.reload(registry)
        table = registry.REGISTRY
    except Exception as exc:
        print(f"0 {len(CASES)}")
        if verbose:
            print(f"cannot load opkit/registry.py: {exc}", file=sys.stderr)
        return

    passed = 0
    for op, value, expected in CASES:
        try:
            if op not in table:
                raise KeyError(f"{op!r} is not registered in opkit/registry.py")
            got = table[op](value)
            if got != expected:
                raise AssertionError(f"{op}({value!r}) == {got!r}, expected {expected!r}")
            passed += 1
        except Exception as exc:
            if verbose:
                print(f"FAIL {op}({value!r}): {exc}", file=sys.stderr)
                if not isinstance(exc, (KeyError, AssertionError, NotImplementedError)):
                    traceback.print_exc()

    print(f"{passed} {len(CASES)}")


if __name__ == "__main__":
    main()
'''

CASES_HEADER = '''"""Conformance cases.

Second deliberate contention point: every operation's cases live in this one
list, so agents adding cases collide here too.

Each entry is (operation, input, expected_output).
"""

CASES = [
'''


def sh(args, cwd, env=None, check=True):
    return subprocess.run(
        args, cwd=str(cwd), env=env, check=check, capture_output=True, text=True
    )


def write(path: Path, content: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content)


def build_workbench(root: Path, ops) -> None:
    """Write the Python package, tests, and the contention points."""
    write(root / "opkit" / "__init__.py", "")
    write(root / "opkit" / "registry.py", REGISTRY_TEMPLATE)
    write(root / "opkit" / "ops" / "__init__.py", "")

    for name, spec, _ in ops:
        write(root / "opkit" / "ops" / f"{name}.py", OP_STUB.format(name=name, spec=spec))

    write(root / "tests" / "__init__.py", "")
    write(root / "score.py", SCORER)
    os.chmod(root / "score.py", 0o755)

    lines = [CASES_HEADER]
    for name, _, cases in ops:
        lines.append(f"    # --- {name} ---\n")
        for value, expected in cases:
            lines.append(f"    ({name!r}, {value!r}, {expected!r}),\n")
    lines.append("]\n")
    write(root / "tests" / "cases.py", "".join(lines))


    write(
        root / "README.md",
        """# opkit — swarm trial workbench

A registry of small pure string operations. The domain is deliberately dull;
this exists to make agents collide.

- **Implement** an operation in `opkit/ops/<name>.py` as `run(value: str) -> str`
- **Register** it in `opkit/registry.py` (shared — expect conflicts)
- **Score** with `./score.py`, which prints `<passing> <total>`
  (`./score.py -v` lists each failure)

The score is a count of passing conformance cases. Do not optimise for speed;
nothing here is timed.
""",
    )


def init_repo(root: Path, env, jjj: str) -> None:
    sh(["git", "init", "-q", "."], root, env)
    sh(["git", "config", "user.name", "swarm-seed"], root, env)
    sh(["git", "config", "user.email", "swarm-seed@example.invalid"], root, env)
    sh(["git", "add", "-A"], root, env)
    sh(["git", "commit", "-q", "-m", "seed: opkit workbench"], root, env)
    sh(["jj", "git", "init", "--colocate"], root, env, check=False)
    sh(["jj", "config", "set", "--repo", "user.name", "swarm-seed"], root, env, check=False)
    sh(
        ["jj", "config", "set", "--repo", "user.email", "swarm-seed@example.invalid"],
        root,
        env,
        check=False,
    )
    sh([jjj, "init"], root, env)


def seed_problems(root: Path, ops, env, jjj: str) -> list:
    """Create one jjj problem per operation, as the steering user would."""
    created = []
    for name, spec, cases in ops:
        out = sh(
            [
                jjj, "problem", "new",
                f"Implement the {name} operation",
                "--priority", "high",
                "--tags", f"op,{name}",
                "--force",
                "--json",
            ],
            root,
            env,
        )
        problem = json.loads(out.stdout)
        created.append({"op": name, "id": problem["id"], "spec": spec, "cases": len(cases)})
    return created


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("root", type=Path, help="directory to create the workbench in")
    ap.add_argument(
        "--problems",
        type=int,
        default=len(OPS),
        help="how many operations to seed; set BELOW the agent count to force "
        "claim contention (default: all)",
    )
    ap.add_argument(
        "--jjj",
        default=os.environ.get(
            "JJJ_BIN", str(Path(__file__).resolve().parents[3] / "target" / "release" / "jjj")
        ),
    )
    ap.add_argument("--force", action="store_true", help="delete an existing workbench")
    args = ap.parse_args()

    root: Path = args.root.resolve()
    if root.exists():
        if not args.force:
            sys.exit(f"{root} exists; pass --force to replace it")
        shutil.rmtree(root)
    root.mkdir(parents=True)

    if not Path(args.jjj).exists():
        sys.exit(f"jjj binary not found at {args.jjj} (build with `cargo build --release`)")

    ops = OPS[: args.problems]
    env = dict(os.environ)

    build_workbench(root, ops)
    init_repo(root, env, args.jjj)
    problems = seed_problems(root, ops, env, args.jjj)

    manifest = {
        "root": str(root),
        "ops": [p["op"] for p in problems],
        "problems": problems,
        "total_cases": sum(p["cases"] for p in problems),
    }
    (root / "swarm-manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")

    print(f"workbench:  {root}")
    print(f"operations: {len(problems)}  ({', '.join(p['op'] for p in problems)})")
    print(f"cases:      {manifest['total_cases']} (this is the fitness ceiling)")
    score = subprocess.run(["./score.py"], cwd=str(root), capture_output=True, text=True)
    print(f"score now:  {score.stdout.strip()} (expected 0 of {manifest['total_cases']})")


if __name__ == "__main__":
    main()
