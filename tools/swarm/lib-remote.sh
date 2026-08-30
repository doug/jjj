#!/usr/bin/env bash
#
# A cheap "has anything happened?" signal for the host-side supervisors.
#
# The sampler and the watchdog both did their expensive work on a fixed timer —
# a container run, or a clone plus a fetch plus a scorer — whether or not the
# fleet had moved since the last tick. That cost is paid hardest in exactly the
# situation it is least useful: a run that reached its ceiling at minute 35 and
# then sat still for the rest of the hour.
#
# Every effect the fleet can have on shared state arrives as a ref update in the
# bare remote: `main` for merged code, `jjj/*` for metadata, `review-s-*` for
# published diffs. So the hash of the ref table is a complete fingerprint of
# fleet activity, and computing it is one local git command with no clone, no
# container and no network.
#
# Unchanged fingerprint therefore *proves* the expensive work would produce the
# same answer, rather than merely suggesting it. That is what makes skipping it
# sound instead of a heuristic.

# Print a fingerprint of every ref in a bare repository, or the empty string if
# it cannot be read.
remote_fingerprint() {
    local remote="$1"
    [ -d "$remote" ] || return 0
    # Sorted, so ref enumeration order cannot make an idle fleet look busy.
    git --git-dir="$remote" for-each-ref --format='%(objectname) %(refname)' 2>/dev/null \
        | sort \
        | { shasum 2>/dev/null || sha1sum 2>/dev/null; } \
        | awk '{print $1}'
}
