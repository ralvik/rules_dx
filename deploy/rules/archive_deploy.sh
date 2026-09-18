#!/usr/bin/env bash
# Deploy program for `archive_release` (issue #181).
#
# Invoked via `bazel run :<name>` or `dx deploy :<name>`. The generated
# launcher resolves the staged app, tarball, and checksum from its
# runfiles forest and execs this script with their paths as `$1..$3`;
# any further args are user args after `--`, whose first entry optionally
# selects the output directory. Verifies the checksum, copies both
# artifacts out, and reports the resolved profile from `$DX_PROFILE`
# (forwarded by `dx deploy`; bare `bazel run` leaves it unset).
set -euo pipefail

# Portable realpath (issue #299): GNU `realpath` is absent on macOS;
# `readlink -f` covers some platforms, python3 covers the rest.
portable_realpath() {
  if command -v realpath >/dev/null 2>&1; then
    realpath "$1"
  elif command -v readlink >/dev/null 2>&1 && readlink -f "$1" >/dev/null 2>&1; then
    readlink -f "$1"
  else
    python3 -c 'import os,sys; print(os.path.realpath(sys.argv[1]))' "$1"
  fi
}

app_rel="$1"
tarball_rel="$2"
checksum_rel="$3"
shift 3

# Display the staged executable name (`hello`, `deploy_program`): do not
# `realpath` the app — it is a symlink chain (`stage -> exe ->
# upstream`) and resolving would report the final target
# (`hello_upstream`, `deploy_program.sh`) instead of the release member.
app_name="$(basename "${app_rel}")"
tarball="$(portable_realpath "${tarball_rel}")"
checksum="$(portable_realpath "${checksum_rel}")"

outdir=""
if [ "$#" -ge 1 ]; then
  outdir="$1"
elif [ -n "${BUILD_WORKSPACE_DIRECTORY:-}" ]; then
  outdir="${BUILD_WORKSPACE_DIRECTORY}"
else
  outdir="${PWD}"
fi
mkdir -p "${outdir}"

if command -v sha256sum >/dev/null 2>&1; then
  expected="$(cut -d' ' -f1 "${checksum}")"
  actual="$(sha256sum "${tarball}" | cut -d' ' -f1)"
else
  expected="$(cut -d' ' -f1 "${checksum}")"
  actual="$(shasum -a 256 "${tarball}" | cut -d' ' -f1)"
fi
if [ "${expected}" != "${actual}" ]; then
  echo "archive_deploy: checksum mismatch for ${tarball}" >&2
  echo "  expected: ${expected}" >&2
  echo "  actual:   ${actual}" >&2
  exit 1
fi

base_tarball="$(basename "${tarball}")"
base_checksum="$(basename "${checksum}")"
cp "${tarball}" "${outdir}/${base_tarball}"
cp "${checksum}" "${outdir}/${base_checksum}"

echo "archive_deploy: released ${app_name} (profile: ${DX_PROFILE:-<unset>})"
echo "  tarball:  ${outdir}/${base_tarball}"
echo "  checksum: ${outdir}/${base_checksum}"
