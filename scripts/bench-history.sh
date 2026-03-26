#!/usr/bin/env bash
# bench-history.sh — Run criterion benchmarks and archive results with git metadata.
#
# Usage: ./scripts/bench-history.sh [extra cargo bench args...]
#
# Outputs a timestamped JSON summary to benches/history/ and prints a table.

set -euo pipefail

HISTORY_DIR="benches/history"
mkdir -p "$HISTORY_DIR"

TIMESTAMP=$(date -u +%Y%m%dT%H%M%SZ)
COMMIT=$(git rev-parse --short HEAD 2>/dev/null || echo "uncommitted")
BRANCH=$(git branch --show-current 2>/dev/null || echo "unknown")
OUTFILE="$HISTORY_DIR/${TIMESTAMP}_${COMMIT}.txt"

echo "=== naad benchmark run ==="
echo "  timestamp: $TIMESTAMP"
echo "  commit:    $COMMIT"
echo "  branch:    $BRANCH"
echo ""

# Run benchmarks, capturing output
cargo bench "$@" 2>&1 | tee "$OUTFILE"

echo ""
echo "Results saved to $OUTFILE"

# Extract and display summary lines (criterion "time:" lines)
echo ""
echo "=== Summary ==="
grep -E '^\s*(time:|thrpt:)' "$OUTFILE" || echo "(no summary lines found — check full output)"
