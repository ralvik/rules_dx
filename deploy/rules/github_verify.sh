#!/usr/bin/env bash
# Verifies one `github_release` deploy program (issue #182).
#
# `$1` is the program rootpath, `$2` the expected tag, `$3...` the
# expected asset basenames. Runs the program with `GH_RELEASE_DRY_RUN=1`
# (no network, no mutation) and asserts the reported tag, every asset,
# and the draft-only flags (`--draft --verify-tag`) appear in the
# would-run command. Tagged `no-coverage`: process-spawning tests stay
# out of the coverage denominator per the repo coverage preset.
set -euo pipefail

prog="$1"
want_tag="$2"
shift 2

out="$(GH_RELEASE_DRY_RUN=1 "${prog}")"
echo "${out}"

echo "${out}" | grep -q "tag: ${want_tag}$" || {
  echo "github tag line 'tag: ${want_tag}' not found" >&2
  exit 1
}

for base in "$@"; do
  echo "${out}" | grep -q "asset: ${base} " || {
    echo "github asset '${base}' not reported" >&2
    exit 1
  }
done

echo "${out}" | grep -q "gh release create ${want_tag} " || {
  echo "github would-run command missing 'gh release create ${want_tag}'" >&2
  exit 1
}

echo "${out}" | grep -q -- "--draft --verify-tag$" || {
  echo "github would-run command missing draft-only flags '--draft --verify-tag'" >&2
  exit 1
}

echo "github OK: draft ${want_tag} with $# asset(s), --draft --verify-tag"
