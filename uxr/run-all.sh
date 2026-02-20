#!/usr/bin/env bash
# Run all UXR scenarios and capture output.
#
# Usage:
#   ./uxr/run-all.sh              # Run all scenarios
#   ./uxr/run-all.sh 01           # Run specific scenario by number
#   ./uxr/run-all.sh --save       # Run all and save output to uxr/output/
#
# Environment variables:
#   JJJ_BIN=/path/to/jjj          # Override jjj binary location
#   UXR_KEEP_TMPDIR=1             # Don't clean up temp dirs (for debugging)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SCENARIOS_DIR="$SCRIPT_DIR/scenarios"
OUTPUT_DIR="$SCRIPT_DIR/output"

SAVE=0
FILTER=""
for arg in "$@"; do
    case "$arg" in
        --save) SAVE=1 ;;
        *) FILTER="$arg" ;;
    esac
done

# Build jjj first if needed
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
if [[ ! -x "$REPO_ROOT/target/debug/jjj" ]] && [[ -z "${JJJ_BIN:-}" ]]; then
    echo "Building jjj..."
    (cd "$REPO_ROOT" && cargo build 2>&1 | tail -1)
fi

TOTAL_PASS=0
TOTAL_FAIL=0
TOTAL_SKIP=0
RESULTS=()

mkdir -p "$OUTPUT_DIR"

for scenario in "$SCENARIOS_DIR"/*.sh; do
    name=$(basename "$scenario" .sh)

    # Apply filter if specified
    if [[ -n "$FILTER" ]] && [[ "$name" != *"$FILTER"* ]]; then
        continue
    fi

    echo ""
    echo "================================================================="
    echo "Running: $name"
    echo "================================================================="

    output_file="$OUTPUT_DIR/$name.log"

    set +e
    if [[ $SAVE -eq 1 ]]; then
        bash "$scenario" 2>&1 | tee "$output_file"
        exit_code=${PIPESTATUS[0]}
    else
        bash "$scenario" 2>&1
        exit_code=$?
    fi
    set -e

    if [[ $exit_code -eq 0 ]]; then
        RESULTS+=("  PASS  $name")
    else
        RESULTS+=("  FAIL  $name")
    fi
done

echo ""
echo "================================================================="
echo "UXR Test Summary"
echo "================================================================="
for r in "${RESULTS[@]}"; do
    echo "$r"
done
echo ""

# Save summary
if [[ $SAVE -eq 1 ]]; then
    TIMESTAMP=$(date +%Y%m%d-%H%M%S)
    SUMMARY="$OUTPUT_DIR/summary-$TIMESTAMP.txt"
    {
        echo "UXR Run: $TIMESTAMP"
        echo "Binary: ${JJJ_BIN:-$REPO_ROOT/target/debug/jjj}"
        echo ""
        for r in "${RESULTS[@]}"; do
            echo "$r"
        done
    } > "$SUMMARY"
    echo "Output saved to: $OUTPUT_DIR/"
    echo "Summary: $SUMMARY"
fi
