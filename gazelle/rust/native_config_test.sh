#!/usr/bin/env bash
# M10 WP3: native-config generation end to end.
#
# Covers the temporal and failure paths golden files cannot: fresh
# generation with hint binding, rerun idempotency, config-file removal
# (target stubs out, hint clears), unknown-tool failure before BUILD
# emission, and ambiguous same-tool configs failing closed.
set -euo pipefail

gazelle="$(realpath "$1")"
root="${TEST_TMPDIR}/workspace"
mkdir -p "${root}/crate/src" "${root}/crate/styles/org"
touch "${root}/WORKSPACE"
printf 'pub fn current() {}\n' > "${root}/crate/src/lib.rs"
printf 'edition = "2021"\n' > "${root}/crate/rustfmt.toml"
printf '[lints]\n' > "${root}/crate/clippy.toml"
printf 'StylesPath = styles\n' > "${root}/crate/.vale.ini"
printf 'extends: existence\n' > "${root}/crate/styles/org/Example.yml"

(cd "${root}" && "${gazelle}" -repo_root="${root}")
build="${root}/crate/BUILD.bazel"
for want in 'name = "rustfmt_config"' 'name = "clippy_config"' 'name = "vale_config"' 'aspect_hints = [' '":clippy_config"' '":rustfmt_config"' 'data = ["crate/styles/org/Example.yml"]' '//crate:__subpackages__'; do
  if ! grep -qF "${want}" "${build}"; then
    echo "fresh generation missing ${want}" >&2
    cat "${build}" >&2
    exit 1
  fi
done

sha256sum "${build}" > "${TEST_TMPDIR}/first.sums"
(cd "${root}" && "${gazelle}" -repo_root="${root}")
sha256sum -c "${TEST_TMPDIR}/first.sums" > /dev/null || {
  echo "rerun was not idempotent" >&2
  exit 1
}

rm "${root}/crate/rustfmt.toml"
(cd "${root}" && "${gazelle}" -repo_root="${root}")
if grep -q "rustfmt_config" "${build}"; then
  echo "removed config file left its target behind" >&2
  exit 1
fi
if grep -q "rustfmt" "${build}"; then
  echo "removed config file left a stale hint behind" >&2
  exit 1
fi
if ! grep -q '":clippy_config"' "${build}"; then
  echo "removal dropped the surviving tool hint" >&2
  exit 1
fi
if ! grep -q 'name = "clippy_config"' "${build}"; then
  echo "removal dropped an unrelated target" >&2
  exit 1
fi

rm -rf "${root:?}/crate"
mkdir -p "${root}/broken"
printf '# gazelle:dx_native_tools rustfmt bogus\n' > "${root}/broken/BUILD.bazel"
set +e
output="$(cd "${root}" && "${gazelle}" -repo_root="${root}" 2>&1)"
status=$?
set -e
if [[ ${status} -eq 0 || "${output}" != *'unknown native tool "bogus"'* ]]; then
  echo "unknown tool did not fail closed" >&2
  echo "${output}" >&2
  exit 1
fi
if [[ -e "${root}/broken/BUILD" ]]; then
  echo "failed generation wrote a BUILD file" >&2
  exit 1
fi

rm -rf "${root:?}/broken"
mkdir -p "${root}/ambiguous"
cat > "${root}/ambiguous/BUILD.bazel" <<'EOF'
load("@rules_dx//quality:native_config.bzl", "clippy_config")

clippy_config(
    name = "clippy_cfg",
    src = "custom.toml",
)

clippy_config(
    name = "clippy_extra",
    src = "extra.toml",
)
EOF
set +e
output="$(cd "${root}" && "${gazelle}" -repo_root="${root}" 2>&1)"
status=$?
set -e
if [[ ${status} -eq 0 || "${output}" != *'2 config targets'* ]]; then
  echo "ambiguous configs did not fail closed" >&2
  echo "${output}" >&2
  exit 1
fi
