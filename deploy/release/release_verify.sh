#!/usr/bin/env bash
# Verifies the human-run release driver dry-run gate (issue #311).
#
# `$1` is the release.sh rootpath. Asserts the default dry run prints
# the seven-step plan, names the never-creates-tags ceiling, and
# publishes nothing. Tagged `no-coverage`.
set -euo pipefail

prog="$1"
out="$("${prog}" v0.0.0-dryrun)"
echo "${out}"

echo "${out}" | grep -q -F -e 'dry run (RELEASE_DRY_RUN=1)' || { echo "release missing dry-run gate" >&2; exit 1; }
echo "${out}" | grep -q -F -e 'never creates or pushes tags' || { echo "release missing tag ceiling" >&2; exit 1; }
echo "${out}" | grep -q -F -e 'publishing nothing' || { echo "release missing nothing-publishes record" >&2; exit 1; }
echo "${out}" | grep -q -F -e 'dx_verify' || { echo "release missing verify step" >&2; exit 1; }
echo "release OK: dry-run prints the owner-gated human-run plan, publishes nothing"
