#!/usr/bin/env bash
# bench-history.sh — run naad's Cyrius benchmark suites and append the results
# to a CSV history with the measurement regime recorded alongside each row.
#
# Usage:
#   ./scripts/bench-history.sh                          # both suites -> bench-history.csv
#   ./scripts/bench-history.sh results.csv              # custom output
#   ./scripts/bench-history.sh "" tests/hotpath.bcyr    # one suite
#
# Replaces the pre-port script of the same name, which ran `cargo bench` against
# criterion and wrote to benches/history/. Neither exists: the port dropped
# criterion, there is no Cargo.toml, and there is no benches/ directory — so the
# old script could not run from either end and naad has NO archived bench rows.
set -euo pipefail

cd "$(dirname "$0")/.."

HISTORY_FILE="${1:-bench-history.csv}"
SUITES="${2:-tests/hotpath.bcyr tests/naad.bcyr}"
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
COMMIT=$(git rev-parse --short HEAD 2>/dev/null || echo "uncommitted")
BRANCH=$(git branch --show-current 2>/dev/null || echo "unknown")

# Every `cyrius` call re-resolves deps and races on cyrius.lock (concurrent runs
# corrupt it), so the bench run is serialized behind the same lock the rest of
# the project's tooling uses. Override with NAAD_BUILD_LOCK.
LOCK="${NAAD_BUILD_LOCK:-${TMPDIR:-/tmp}/naad-build.lock}"

# Schema. `stat` names WHICH statistic estimate_ns holds; `regime` names what the
# INSTRUMENT was doing when it was taken. They are independent, and the second is
# the one that silently invalidates cross-run comparison.
#
# cyrius 6.5.19 taught lib/bench.cyr to calibrate one clock read on the host and
# subtract it from every sample, and taught bench_run to size its own batches
# instead of wrapping a clock pair around every iteration. Both rewrite the
# number without touching the code being measured. naad crossed that boundary at
# 2.1.2 (pin 6.3.19 -> 6.5.35), and the effect is not subtle: naad's four
# hot-path rows sat on a ~1.34 us timer floor that was 95-98% of each reported
# figure, which is why the 2.0.0 CHANGELOG records four operations differing by
# 4.78x as a single "~1.4 us/sample".
#
# regime is DERIVED, not declared — read from whether this run's harness printed
# its own measured floor, i.e. the instrument reporting on itself — so it cannot
# go stale the way a hand-maintained constant does. floor_ns records what the
# clock cost on THIS host, THIS boot: upstream measured a 230x spread across the
# four hosts its gate runs on (~15 ns macOS arm64 to ~3,550 ns aarch64 Linux),
# and lib/bench.cyr records this very machine producing both ~400 ns and
# ~1,700 ns across reboots. It is not a property of the code, so it must travel
# with the row.
#
# What this CANNOT do, said plainly: it separates "floor subtracted" from "floor
# not subtracted", because that is what the output makes visible. A future
# instrument change that kept printing the floor line would not flip it. This
# narrows the hole; it does not close it.
CSV_HEADER="timestamp,commit,branch,suite,benchmark,estimate_ns,stat,avg_ns,min_ns,max_ns,iters,regime,floor_ns"
if [ ! -f "$HISTORY_FILE" ]; then
    echo "$CSV_HEADER" > "$HISTORY_FILE"
elif ! head -1 "$HISTORY_FILE" | grep -q ',regime,'; then
    sed -i "1s/.*/${CSV_HEADER}/" "$HISTORY_FILE"
    echo "note: bench-history header widened with regime/floor_ns; earlier rows read back as regime=''"
fi

echo "=== naad benchmark run ==="
echo "  commit:    $COMMIT"
echo "  branch:    $BRANCH"
echo "  timestamp: $TIMESTAMP"
echo "  suites:    $SUITES"
echo ""

normalize_to_ns() {
    awk -v v="$1" -v u="$2" 'BEGIN{
        if (u == "ps")                 printf "%.4f", v / 1000;
        else if (u == "ns")            printf "%s",   v;
        else if (u == "us" || u == "µs") printf "%.4f", v * 1000;
        else if (u == "ms")            printf "%.4f", v * 1000000;
        else if (u == "s")             printf "%.4f", v * 1000000000;
        else                           printf "%s",   v;
    }'
}

TOTAL=0
SKIPPED=0

for SUITE in $SUITES; do
    [ -f "$SUITE" ] || { echo "ERROR: no such suite: $SUITE" >&2; exit 1; }
    OUTPUT=$(flock "$LOCK" cyrius bench "$SUITE" 2>&1 | sed 's/\x1b\[[0-9;]*m//g')
    echo "$OUTPUT"
    echo ""

    # Regime, read off the harness's own output rather than declared here. Since
    # 6.5.19 bench_report prints, once per process:
    #   [timer floor 1.349us per clock read, measured; subtracted from every sample]
    # Presence of that line IS the marker. Absent -> the pre-6.5.19 instrument,
    # whose numbers include the clock and whose bench_run paid a clock pair per
    # iteration. Those two populations must never be averaged together.
    REGIME=""
    FLOOR_NS=""
    if [[ "$OUTPUT" =~ \[timer\ floor\ ([0-9]+(\.[0-9]+)?)(ps|ns|µs|us|ms|s)\ per\ clock\ read ]]; then
        REGIME="net"
        FLOOR_NS=$(normalize_to_ns "${BASH_REMATCH[1]}" "${BASH_REMATCH[3]}")
        echo "  regime: net (timer floor ${FLOOR_NS} ns, measured on this host/boot, subtracted per sample)"
    else
        REGIME="raw"
        echo "  regime: raw (no measured floor reported — pre-6.5.19 instrument)"
    fi
    echo ""

    # Anchored on the harness's own field names, NOT by scavenging the last
    # number in the line — that trick records the max= field. naad's benchmark
    # names contain spaces and parens ("osc_next_sample (sine)", "noise pink
    # (Voss-McCartney)"), so the name group is permissive and the line is
    # disambiguated by the literal " avg (min=" that follows it. A line that
    # looks like a result but does not parse is COUNTED and fails the run: a
    # format change upstream must be loud, not quietly halve the history.
    while IFS= read -r line; do
        if [[ "$line" =~ ^[[:space:]]*([A-Za-z_][^:]*):[[:space:]]+([0-9]+(\.[0-9]+)?)(ps|ns|µs|us|ms|s)[[:space:]]+avg[[:space:]]+\(min=([0-9]+(\.[0-9]+)?)(ps|ns|µs|us|ms|s)[[:space:]]+max=([0-9]+(\.[0-9]+)?)(ps|ns|µs|us|ms|s)\)([[:space:]]+\[([0-9]+)[[:space:]]+iters\])? ]]; then
            NAME="${BASH_REMATCH[1]}"
            AVG_NS=$(normalize_to_ns "${BASH_REMATCH[2]}"  "${BASH_REMATCH[4]}")
            MIN_NS=$(normalize_to_ns "${BASH_REMATCH[5]}"  "${BASH_REMATCH[7]}")
            MAX_NS=$(normalize_to_ns "${BASH_REMATCH[8]}"  "${BASH_REMATCH[10]}")
            ITERS="${BASH_REMATCH[12]:-}"
            printf '%s,%s,%s,%s,"%s",%s,avg,%s,%s,%s,%s,%s,%s\n' \
                "$TIMESTAMP" "$COMMIT" "$BRANCH" "$SUITE" "$NAME" \
                "$AVG_NS" "$AVG_NS" "$MIN_NS" "$MAX_NS" "$ITERS" \
                "$REGIME" "$FLOOR_NS" >> "$HISTORY_FILE"
            TOTAL=$((TOTAL + 1))
        elif echo "$line" | grep -qE '^[[:space:]]+[A-Za-z_].*:[[:space:]]+[0-9.]+(ps|ns|us|ms|s)' \
             && ! echo "$line" | grep -q 'timer floor'; then
            SKIPPED=$((SKIPPED + 1))
            echo "::warning:: unparsed benchmark line (format drift?): $line" >&2
        fi
    done <<< "$OUTPUT"
done

if [ "$SKIPPED" -gt 0 ]; then
    echo "ERROR: $SKIPPED benchmark line(s) did not match the expected format;" >&2
    echo "       lib/bench.cyr's output shape has changed — fix the parser." >&2
    exit 1
fi
if [ "$TOTAL" -eq 0 ]; then
    echo "ERROR: no benchmarks parsed — refusing to write an empty history." >&2
    exit 1
fi

echo "=== $TOTAL row(s) appended to $HISTORY_FILE ==="
echo ""
echo "Trend comparison MUST filter on regime: rows either side of the 6.5.19"
echo "instrument boundary are all stat=avg, so comparing them on stat alone"
echo "reports pure artefact as improvement. Never compare a 'raw' row to a"
echo "'net' one, and treat floor_ns as part of a row's identity — the floor"
echo "moves between reboots on a single host."
