#!/usr/bin/env bash
#
# Keep a subscription credential usable by containers, without letting them
# touch it.
#
# On macOS the Claude Code CLI keeps its live OAuth credential in the **Keychain**
# and refreshes it there; `~/.claude/.credentials.json` is a stale artefact that
# is not what the CLI reads. A Linux container can reach neither, and it must not
# refresh the token itself — several containers racing to rotate one credential
# is a good way to break the host login.
#
# So exactly one process refreshes, and it runs here on the host:
#
#   1. read the live credential out of the Keychain
#   2. if it is close to expiring, trigger a refresh by making one trivial CLI
#      call — the CLI's own supported refresh path, not a hand-rolled OAuth
#      exchange against an endpoint that may change
#   3. export the current token to a file the containers mount **read-only**
#
# Each agent turn is a fresh `claude -p`, so it re-reads the file every turn and
# picks up rotations without restarting.
#
# Usage: token-refresher.sh <output-file> [--interval SECONDS] [--margin SECONDS]

set -uo pipefail

OUT="${1:?output credential file required}"
shift || true

INTERVAL=300      # re-export every 5 minutes
MARGIN=2700       # refresh when under 45 minutes of validity remain

while [ $# -gt 0 ]; do
    case "$1" in
        --interval) INTERVAL="$2"; shift 2 ;;
        --margin) MARGIN="$2"; shift 2 ;;
        *) echo "token-refresher: unknown option $1" >&2; exit 1 ;;
    esac
done

KEYCHAIN_SERVICE="Claude Code-credentials"

log() { printf '[%s] refresher: %s\n' "$(date +%H:%M:%S)" "$*"; }

# Read the credential blob that actually carries tokens.
#
# The CLI no longer keeps one entry under a fixed name. Alongside
# `Claude Code-credentials` there are per-account entries suffixed with a hash —
# `Claude Code-credentials-aea9b5d3` — and the bare entry can be left behind as a
# *shell*: valid JSON, right subscription, and `accessToken` an empty string.
#
# Exporting that shell is worse than exporting nothing. Every container starts
# with a credential file that looks well-formed and authenticates nothing, and
# the failure surfaces as six agents failing every turn rather than as a missing
# file. So choose by content — the entry holding a non-empty access token, most
# recently valid first — not by name.
read_keychain() {
    local svc blob best=""
    # Read line-wise: these service names contain spaces, so `for svc in $(...)`
    # splits "Claude Code-credentials-aea9b5d3" into two words and finds neither.
    while IFS= read -r svc; do
        [ -n "$svc" ] || continue
        blob="$(security find-generic-password -s "$svc" -w 2>/dev/null)" || continue
        [ -n "$blob" ] || continue
        if printf '%s' "$blob" | python3 -c '
import sys, json
try:
    d = json.load(sys.stdin)["claudeAiOauth"]
except Exception:
    sys.exit(1)
sys.exit(0 if (d.get("accessToken") or "").strip() else 1)
' 2>/dev/null; then
            best="$blob"
            break
        fi
    done <<EOF
$(keychain_services)
EOF
    [ -n "$best" ] || return 1
    printf '%s' "$best"
}

# Candidate service names, most specific first. The bare name goes last: when a
# suffixed entry exists it is the live one, and the bare entry is the leftover.
keychain_services() {
    security dump-keychain 2>/dev/null \
        | sed -n 's/.*"svce"<blob>="\(Claude Code-credentials[^"]*\)".*/\1/p' \
        | sort -u | grep -v '^Claude Code-credentials$'
    printf '%s\n' "$KEYCHAIN_SERVICE"
}

# Seconds of validity remaining; `unknown` when the credential does not carry an
# expiry; empty when the Keychain cannot be read at all.
#
# The three states are distinct and conflating two of them is dangerous. A live
# `claude` login can write `expiresAt: 0` — a sentinel meaning "not tracked",
# not a timestamp — and arithmetic on it says the token expired in 1970. This
# refresher would then declare the OAuth session dead, touch AUTH_DEAD, raise an
# escalation, and the watchdog would stop a perfectly healthy fleet. The
# credential in that state works: verified by calling the CLI with it.
remaining() {
    read_keychain | python3 -c '
import sys, json, time
raw = sys.stdin.read().strip()
if not raw:
    sys.exit(1)
try:
    exp = json.loads(raw)["claudeAiOauth"]["expiresAt"]
except Exception:
    sys.exit(1)
# 0 (or missing) is a sentinel, not a date. Never treat it as an expiry.
if not exp or exp <= 0:
    print("unknown")
else:
    print(int(exp / 1000 - time.time()))
' 2>/dev/null
}

if ! read_keychain >/dev/null; then
    log "FATAL: no Keychain entry under 'Claude Code-credentials*' holds an access token."
    log "       (an entry may exist but be empty — a shell left by an earlier install)"
    log "       Run \`claude\` on this host and log in, or set ANTHROPIC_API_KEY."
    exit 1
fi

log "started; exporting to $OUT every ${INTERVAL}s, refreshing under ${MARGIN}s validity"


# Raise or clear the auth escalation through jjj, in the seed clone.
#
# Best-effort throughout: a refresher that dies because the seed is mid-fetch
# would take the credential supply down with it, and a missing escalation is a
# worse outcome only than no credentials at all.
escalate_auth() {
    local root="$1" action="$2"
    local seed="$root/seed"
    local jjj="${JJJ_BIN:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)/target/release/jjj}"
    [ -d "$seed/.jj" ] && [ -x "$jjj" ] || return 0

    case "$action" in
        raise)
            ( cd "$seed" \
              && "$jjj" escalate "The host OAuth session has expired. Run \`claude\` on the host and log in; the fleet cannot make any progress until then." \
                 >/dev/null 2>&1 \
              && "$jjj" push >/dev/null 2>&1 ) || log "warning: could not publish the auth escalation"
            ;;
        clear)
            # Clear every open escalation whose reason names the session, not
            # just the one we raised: a restarted refresher does not remember
            # which id it used, and a stale auth escalation would stop the fleet
            # after the credential came back.
            ( cd "$seed" \
              && "$jjj" escalate --json 2>/dev/null \
                 | python3 -c "
import json, sys
try:
    for r in json.load(sys.stdin):
        if 'OAuth session' in r.get('reason', ''):
            print(r['id'])
except Exception:
    pass" \
                 | while read -r id; do "$jjj" escalate --clear "$id" >/dev/null 2>&1; done \
              && "$jjj" push >/dev/null 2>&1 ) || true
            ;;
    esac
}

while true; do
    left="$(remaining || true)"

    if [ -z "$left" ]; then
        log "WARNING: cannot read the Keychain credential; retrying"
    elif [ "$left" = "unknown" ]; then
        # No expiry to act on. Export the credential as usual and say nothing
        # further: the CLI refreshes its own token when it makes a call, and the
        # honest signal for "is auth working" is whether agent turns succeed —
        # which the health check already reads. Guessing here is what would kill
        # a healthy run.
        if [ ! -f "$(dirname "$OUT")/.expiry_unknown_logged" ]; then
            log "credential carries no expiry (expiresAt=0); not tracking validity"
            touch "$(dirname "$OUT")/.expiry_unknown_logged"
        fi
        rm -f "$(dirname "$OUT")/AUTH_DEAD"
    else
        if [ "$left" -lt "$MARGIN" ]; then
            log "token has ${left}s left; triggering a refresh via the CLI"
            # A trivial call makes the CLI refresh and write back to the Keychain.
            # Cheap, and it uses the supported path rather than a hand-rolled
            # token exchange.
            (cd /tmp && claude -p "ok" --model haiku </dev/null >/dev/null 2>&1) || \
                log "WARNING: refresh call failed; token may expire"
            left="$(remaining || echo 0)"
            log "after refresh: ${left}s remaining"

            # A refresh that leaves the token already expired means the OAuth
            # *session* is gone, not just the access token, and no amount of
            # retrying fixes that — only an interactive `claude /login` does.
            #
            # This has to be loud. A fleet once spent 6.8 hours of a 24-hour run
            # failing 400 turns in a second each while this logged a warning
            # nobody was watching; the score sat frozen and every container
            # looked healthy from the outside. The marker file lets the agents
            # stop asking, and lets a person see the cause without reading logs.
            #
            # The marker was the first fix and is a patch for one instance of a
            # general gap. `jjj escalate` is the general form: it travels with
            # the metadata, so every agent and every clone sees the same signal,
            # and the watchdog stops the fleet rather than spending the rest of
            # the deadline on turns that cannot succeed.
            root="$(dirname "$OUT")"
            if [ "${left:-0}" -le 0 ]; then
                log "FATAL: the OAuth session has expired and cannot be renewed"
                log "       run \`claude\` on this host and log in; the fleet will resume"
                if [ ! -f "$root/AUTH_DEAD" ]; then
                    # Only on the transition into the dead state: re-raising every
                    # poll would bury the real one under identical copies.
                    touch "$root/AUTH_DEAD"
                    escalate_auth "$root" raise
                fi
            else
                if [ -f "$root/AUTH_DEAD" ]; then
                    rm -f "$root/AUTH_DEAD"
                    escalate_auth "$root" clear
                fi
            fi
        fi
    fi

    # Export on every pass, whatever the expiry state.
    #
    # This sat inside the branch that handles a credential *with* a known
    # expiry, so adding the `unknown` case silently skipped the refresher's
    # actual job and every agent started with no credential at all. Exporting is
    # unconditional: the expiry only decides whether to trigger a refresh first.
    #
    # Write via a temp file and rename, so a container never reads a
    # half-written credential.
    tmp="$OUT.tmp.$$"
    if read_keychain > "$tmp" && [ -s "$tmp" ]; then
        chmod 600 "$tmp"
        mv -f "$tmp" "$OUT"
    else
        rm -f "$tmp"
        log "WARNING: export produced nothing; keeping the previous file"
    fi

    sleep "$INTERVAL"
done
