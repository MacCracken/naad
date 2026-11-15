#!/usr/bin/env bash
# fetch-sibling-bundles.sh — download the newest tagged bundle of every sibling
# library naad shares a compilation unit with, for symbol-collision-check.sh.
#
# WHY THIS EXISTS. naad's own gate checked only hisab, goonj, sakshi and the
# stdlib. abaco 2.8.0 added `power_spectrum`, which naad also defined, and
# dhvani, which links both, stopped building. Nothing in naad's CI could have
# seen it, because abaco is not naad's dependency. The siblings below are
# dhvani's `[deps.*]` (read 2026-11-15): the widest unit naad is linked into.
# naad, hisab, goonj and sakshi are left out because the gate covers them.
#
# "Newest" is the highest x.y.z tag. abaco also carries a calendar tag
# (2026.3.18) from before its semver line, so tags with a major past 999 are
# skipped. A repo or tag that cannot be fetched fails the script: a gate that
# silently checks fewer bundles than it says is how the old one went wrong.
#
# Usage:  ./scripts/fetch-sibling-bundles.sh OUT_DIR
# Prints the path of each bundle it wrote, one per line.
set -euo pipefail

OUT="${1:?usage: fetch-sibling-bundles.sh OUT_DIR}"
mkdir -p "$OUT"

SIBLINGS=(abaco sankoch shravan svara nidhi ghurni garjan prani varna
          shabdakosh shabda patra yukti vani dhvani)

# Each failure names the repo and the step. Under `set -euo pipefail` a pipeline
# whose grep matches nothing fails the assignment it sits in, so the tag filter
# tolerates no match and the empty result is reported below instead.
for repo in "${SIBLINGS[@]}"; do
    url="https://github.com/MacCracken/${repo}.git"
    if ! refs=$(git ls-remote --tags --refs "$url"); then
        echo "ERROR: cannot list the tags of $repo ($url)" >&2
        exit 1
    fi
    tag=$(printf '%s\n' "$refs" \
          | awk '{sub("refs/tags/", "", $2); print $2}' \
          | { grep -E '^[0-9]{1,3}\.[0-9]+\.[0-9]+$' || true; } \
          | sort -V | tail -1)
    if [ -z "$tag" ]; then
        echo "ERROR: no x.y.z tag found for $repo" >&2
        exit 1
    fi
    dest="$OUT/${repo}-${tag}.cyr"
    if ! curl -sSfL "https://raw.githubusercontent.com/MacCracken/${repo}/${tag}/dist/${repo}.cyr" -o "$dest"; then
        echo "ERROR: cannot fetch dist/${repo}.cyr at ${repo} ${tag}" >&2
        exit 1
    fi
    echo "$dest"
done
