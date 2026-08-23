#!/usr/bin/env bash
# symbol-collision-check.sh — fail if dist/naad.cyr's top-level symbols collide
# with any dependency bundle or the pinned stdlib snapshot.
#
# WHY THIS EXISTS. Cyrius has ONE FLAT NAMESPACE once distlib bundles are
# concatenated by a consumer. Duplicate top-level `fn`s draw a warning; duplicate
# top-level `var`s draw NOTHING from cycc or cyrlint. naad shipped for three
# releases with `ERR_INVALID_FREQUENCY` defined as -1 here and -3 in goonj, so
# which value a consumer got depended on include order, and no tool would have
# said so. 2.1.3 renamed naad's six error constants to `NAAD_ERR_*` and drove
# the intersection to zero — this gate is what keeps it there.
#
# The audit that missed it was fn-scoped, so this one is deliberately NOT:
# it covers `fn`, `var`, `const` and `struct` alike.
#
# Usage:  ./scripts/symbol-collision-check.sh [extra-bundle.cyr ...]
set -euo pipefail

cd "$(dirname "$0")/.."

BUNDLE="dist/naad.cyr"
[ -f "$BUNDLE" ] || { echo "ERROR: $BUNDLE not found — run 'cyrius distlib' first." >&2; exit 1; }

PIN="$(grep '^cyrius = ' cyrius.cyml | head -1 | sed 's/cyrius = "\(.*\)"/\1/')"
STDLIB_DIR="${CYRIUS_HOME:-$HOME/.cyrius}/versions/${PIN}/lib"

# Top-level declarations only: anchored at column 0, so an indented local
# declaration inside a function body can never be mistaken for exported surface.
symbols() {
    grep -hoE '^(fn|var|const|struct)[[:space:]]+[A-Za-z_][A-Za-z0-9_]*' "$@" 2>/dev/null \
        | awk '{print $2}' | sort -u
}

NAAD_SYMS=$(mktemp); trap 'rm -f "$NAAD_SYMS" "$DEP_SYMS"' EXIT
DEP_SYMS=$(mktemp)
symbols "$BUNDLE" > "$NAAD_SYMS"
echo "naad bundle: $(wc -l < "$NAAD_SYMS") top-level symbols"

STATUS=0
check() {
    local label="$1"; shift
    local files=()
    for f in "$@"; do [ -e "$f" ] && files+=("$f"); done
    if [ ${#files[@]} -eq 0 ]; then
        echo "  skip  $label (not present)"
        return
    fi
    symbols "${files[@]}" > "$DEP_SYMS"
    local hits
    hits=$(comm -12 "$NAAD_SYMS" "$DEP_SYMS")
    if [ -n "$hits" ]; then
        echo "  FAIL  $label — $(printf '%s\n' "$hits" | wc -l) colliding symbol(s):"
        printf '          %s\n' $hits
        STATUS=1
    else
        echo "  ok    $label"
    fi
}

# Dependency bundles naad itself pulls in. A consumer concatenates all of these
# with naad, so any shared name is a live hazard, not a theoretical one.
check "vs hisab"  lib/hisab.cyr
check "vs goonj"  lib/goonj.cyr
check "vs sakshi" lib/sakshi.cyr

# The pinned stdlib snapshot — every module, not just the leaves naad declares.
# A consumer may pull in stdlib modules naad does not.
if [ -d "$STDLIB_DIR" ]; then
    check "vs stdlib $PIN" "$STDLIB_DIR"/*.cyr
else
    echo "  skip  vs stdlib $PIN (snapshot not installed at $STDLIB_DIR)"
fi

# Any bundle named on the command line — used to check a sibling library naad is
# co-linked with downstream (abaco was the 2.1.1 case).
for extra in "$@"; do check "vs $extra" "$extra"; done

# Self-consistency: the bundle must not define the same top-level name twice.
DUPES=$(grep -hoE '^(fn|var|const|struct)[[:space:]]+[A-Za-z_][A-Za-z0-9_]*' "$BUNDLE" \
        | awk '{print $2}' | sort | uniq -d)
if [ -n "$DUPES" ]; then
    echo "  FAIL  bundle defines a symbol more than once:"
    printf '          %s\n' $DUPES
    STATUS=1
else
    echo "  ok    bundle has no internal duplicates"
fi

if [ "$STATUS" -ne 0 ]; then
    echo ""
    echo "Symbol collision detected. In Cyrius's flat namespace a consumer that"
    echo "concatenates both bundles gets whichever definition comes last, and for"
    echo "top-level 'var's no diagnostic is emitted at all. Prefix naad's side"
    echo "(NAAD_* / naad_*), as 2.1.1 did for the dB helpers and 2.1.3 for ERR_*."
    exit 1
fi

echo "No collisions."
