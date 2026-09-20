#!/usr/bin/env bash
# Verifies one `bcr_check` deploy program.
#
# `$1` is the program rootpath, `$2` the expected module, `$3` the
# expected version. Runs with BCR_DRY_RUN=1 and asserts the would-submit
# PR names the module/version and submits nothing. Tagged `no-coverage`.
set -euo pipefail

prog="$1"
want_module="$2"
want_version="$3"

out="$(BCR_DRY_RUN=1 "${prog}")"
echo "${out}"

echo "${out}" | grep -q -F -e "module: ${want_module}" || { echo "bcr missing module ${want_module}" >&2; exit 1; }
echo "${out}" | grep -q -F -e "version: ${want_version}" || { echo "bcr missing version ${want_version}" >&2; exit 1; }
echo "${out}" | grep -q -F -e 'would submit, submitting nothing' || { echo "bcr missing dry-run gate" >&2; exit 1; }
echo "bcr OK: dry-run would-submit ${want_module}@${want_version}, submits nothing"
