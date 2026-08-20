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

read_keychain() {
    security find-generic-password -s "$KEYCHAIN_SERVICE" -w 2>/dev/null
}

# Seconds of validity remaining, or empty if unreadable.
remaining() {
    read_keychain | python3 -c '
import sys, json, time
raw = sys.stdin.read().strip()
if not raw:
    sys.exit(1)
try:
    exp = json.loads(raw)["claudeAiOauth"]["expiresAt"] / 1000
except Exception:
    sys.exit(1)
print(int(exp - time.time()))
' 2>/dev/null
}

if ! read_keychain >/dev/null; then
    log "FATAL: no Keychain entry '$KEYCHAIN_SERVICE'. Log in with \`claude\` first."
    exit 1
fi

log "started; exporting to $OUT every ${INTERVAL}s, refreshing under ${MARGIN}s validity"

while true; do
    left="$(remaining || true)"

    if [ -z "$left" ]; then
        log "WARNING: cannot read the Keychain credential; retrying"
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
        fi

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
    fi

    sleep "$INTERVAL"
done
