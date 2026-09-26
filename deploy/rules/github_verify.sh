#!/usr/bin/env bash
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
