#!/usr/bin/env bash
# Verifies the human-run release driver dry-run gate (live
# successor to closed for the human-run path).
#
# `$1` is the release.sh rootpath. Asserts the default dry run prints
# the seven-step plan, names the never-creates-tags ceiling, keeps the
# signing-first order (signing_demo step before the github_draft step),
# and publishes nothing. Tagged `no-coverage`.
set -euo pipefail

prog="$1"
out="$("${prog}" v0.0.0-dryrun)"
echo "${out}"

echo "${out}" | grep -q -F -e 'dry run (RELEASE_DRY_RUN=1)' || { echo "release missing dry-run gate" >&2; exit 1; }
echo "${out}" | grep -q -F -e 'never creates or pushes tags' || { echo "release missing tag ceiling" >&2; exit 1; }
echo "${out}" | grep -q -F -e 'publishing nothing' || { echo "release missing nothing-publishes record" >&2; exit 1; }
echo "${out}" | grep -q -F -e 'dx_verify' || { echo "release missing verify step" >&2; exit 1; }
sign_line="$(echo "${out}" | grep -n -F -e 'signing_demo' | head -n 1 | cut -d: -f1)"
draft_line="$(echo "${out}" | grep -n -F -e 'github_draft' | head -n 1 | cut -d: -f1)"
[[ -n "${sign_line}" && -n "${draft_line}" ]] || { echo "release missing signing-first steps" >&2; exit 1; }
[[ "${sign_line}" -lt "${draft_line}" ]] || { echo "release violates signing-first order" >&2; exit 1; }
echo "release OK: dry-run prints the owner-gated human-run plan, signing-first order held, publishes nothing"
