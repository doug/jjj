#!/usr/bin/env bash
# journey-to-doc.sh — Strip test artifacts from journey specs for documentation
#
# Usage: ./tools/journey-to-doc.sh journeys/01-solo-quickstart.md

set -euo pipefail

if [[ $# -eq 0 ]]; then
    echo "Usage: $0 <journey-file.md>" >&2
    exit 1
fi

awk '
BEGIN { fm = 0; fm_count = 0; setup = 0; skip_covers = 0 }

/^---$/ {
    fm_count++
    if (fm_count == 1) { fm = 1; print; next }
    if (fm_count == 2) { fm = 0; skip_covers = 0; print; next }
    print; next
}

fm && /^replaces:/ { next }
fm && /^covers:/ { skip_covers = 1; next }
fm && skip_covers && /^  - / { next }
fm && skip_covers { skip_covers = 0 }

/^```jjj:setup/ { setup = 1; next }
/^```shell:setup/ { setup = 1; next }
setup && /^```$/ { setup = 0; next }
setup { next }

/^> / { next }
/^>! / { next }
/^>~ / { next }
/^>= / { next }

{ print }
' "$1"
