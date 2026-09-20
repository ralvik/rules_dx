#!/usr/bin/env bash
# Apple plus Microsoft acquisition rights qualification.
#
# Records the rights-review slice of the native baseline with fixture
# evidence, without claiming a qualified hermetic-llvm Apple-SDK backend,
# a qualified toolchains_msvc backend, or Supported:
# - Apple: hermetic-llvm v0.8.19 pinned MacOSX26.5 SDK extraction reviewed
#   against the Apple SDK agreement
#   (https://www.apple.com/legal/sla/docs/xcode.pdf); Apple-hosted
#   execution does not by itself authorize separate extraction or
#   unrestricted caching. Pinned in
#   `cc/tests/fixtures/acquisition_rights/pins.bzl`.
# - Microsoft: toolchains_msvc head `8e2aa4624bbb5a53a94f135e90995f307875d1ad`
#   with module `0.0.0` (never a release) plus windows_support `v0.4.1`
#   with MSVC `14.50.35717` plus redist `14.50.35710` plus SDK package
#   `10.0.26100.7705` tracked separately; deliberate EULA repository-env
#   with README mismatch, never automatic; windows_support does not
#   enforce the SDK EULA variable documented by hermetic-llvm, so
#   applicable terms are reviewed rather than inferred.
# - usage vs redistribution stay distinct: acceptance is not permission
#   to redistribute; direct downloads, permission to use, and permission
#   to redistribute are separate checks.
# - cache/mirror/remote: mirrors, redistribution, internal caches, and
#   remote workers need license-approved boundaries recorded separately
#   from technical download success. Official download availability is
#   not permission. Assume rights stays rejected.
# - open with honest records: full backends, transport plus ABI (issue
# , prebuilt interop, corpus plus floors plus coverage
# , release evidence. Backends stay provisional.
#
# Versioned here, run by CI via `bazel run //tools/ci:acquisition_rights_qualification`,
# following //tools/ci:windows_acquisition_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

dx_cd_workspace

dx_test_init

pins="cc/tests/fixtures/acquisition_rights/pins.bzl"
pins_build="cc/tests/fixtures/acquisition_rights/BUILD.bazel"
module="MODULE.bazel"
lock="MODULE.bazel.lock"
native="docs/native-toolchains.md"
matrix="docs/product/support-matrix.md"
build="tools/ci/BUILD.bazel"
ci=".github/workflows/ci.yml"
verify="docs/testing/verification-matrix.md"

# Fixture pair stays present.
if [[ -f "$pins" && -f "$pins_build" ]]; then
  ok
else
  bad "acquisition rights fixture missing (want $pins plus $pins_build)"
fi

# Pins record the Apple SDK identity plus the Apple agreement reference.
if grep -q -F -e 'APPLE_SDK_IDENTITY = "MacOSX26.5"' "$pins" &&
  grep -q -F -e 'APPLE_SDK_SOURCE = "hermetic-llvm v0.8.19 pinned extraction"' "$pins" &&
  grep -q -F -e '6314688712edf3a95f78642d80393868256b4ef2' "$pins" &&
  grep -q -F -e 'APPLE_SDK_AGREEMENT = "https://www.apple.com/legal/sla/docs/xcode.pdf"' "$pins"; then
  ok
else
  bad "pins.bzl lost its Apple MacOSX26.5 plus hermetic-llvm v0.8.19 plus Xcode agreement identity under issue #496"
fi

# Pins record that Apple-hosted execution authorizes neither separate extraction nor unrestricted caching.
if grep -q -F -e 'Apple-hosted execution does not by itself authorize separate extraction or unrestricted caching' "$pins" &&
  grep -q -F -e 'separate extraction needs terms review' "$pins" &&
  grep -q -F -e 'unrestricted caching needs terms review' "$pins"; then
  ok
else
  bad "pins.bzl lost its Apple hosted-execution plus extraction/caching restriction under issue #496"
fi

# Pins record the Microsoft prototype plus windows_support identities for the rights scope.
if grep -q -F -e 'TOOLCHAINS_MSVC_COMMIT = "8e2aa4624bbb5a53a94f135e90995f307875d1ad"' "$pins" &&
  grep -q -F -e 'TOOLCHAINS_MSVC_MODULE = "0.0.0"' "$pins" &&
  grep -q -F -e 'WINDOWS_SUPPORT_VERSION = "v0.4.1"' "$pins" &&
  grep -q -F -e '43dce21da77d8cb4a486709e34f1d887e69058c6' "$pins" &&
  grep -q -F -e 'WINDOWS_SUPPORT_MSVC = "14.50.35717"' "$pins" &&
  grep -q -F -e 'WINDOWS_SUPPORT_REDIST = "14.50.35710"' "$pins" &&
  grep -q -F -e 'WINDOWS_SUPPORT_SDK_PACKAGE = "10.0.26100.7705"' "$pins"; then
  ok
else
  bad "pins.bzl lost its toolchains_msvc plus windows_support v0.4.1 identities under issue #496"
fi

# Pins record the deliberate EULA mechanism plus the README mismatch plus the SDK EULA gap.
if grep -q -F -e 'BAZEL_TOOLCHAINS_MSVC_ACCEPT_MICROSOFT_VISUAL_STUDIO_BUILDTOOLS_EULA' "$pins" &&
  grep -q -F -e 'EULA_MECHANISM = "repository-env"' "$pins" &&
  grep -q -F -e 'EULA_README_MISMATCH = True' "$pins" &&
  grep -q -F -e 'never automatic' "$pins" &&
  grep -q -F -e 'windows_support does not enforce the SDK EULA variable documented by hermetic-llvm' "$pins" &&
  grep -q -F -e 'must be reviewed rather than inferred from setting a variable' "$pins"; then
  ok
else
  bad "pins.bzl lost its deliberate EULA repository-env plus README mismatch plus SDK EULA gap under issue #496"
fi

# Pins record usage vs redistribution as separate checks (acceptance is not redistribution permission).
if grep -q -F -e 'usage vs redistribution reviewed separately' "$pins" &&
  grep -q -F -e 'acceptance is not permission to redistribute' "$pins" &&
  grep -q -F -e 'Direct SDK downloads, permission to use, and permission to redistribute are separate checks' "$pins"; then
  ok
else
  bad "pins.bzl lost its usage vs redistribution plus acceptance-not-redistribution record under issue #496"
fi

# Pins record cache/mirror/remote-worker boundaries plus the assume-rights rejection.
if grep -q -F -e 'mirrors need terms review' "$pins" &&
  grep -q -F -e 'redistribution needs terms review' "$pins" &&
  grep -q -F -e 'internal caches need terms review' "$pins" &&
  grep -q -F -e 'remote workers need terms review' "$pins" &&
  grep -q -F -e 'Official download availability is not permission' "$pins" &&
  grep -q -F -e 'Record license-approved cache/mirror/remote-worker boundaries separately from technical download success' "$pins" &&
  grep -q -F -e '"assume rights"' "$pins"; then
  ok
else
  bad "pins.bzl lost its cache/mirror/remote boundaries plus official-download plus assume-rights rejection under issue #496"
fi

# Rights laziness proof: MODULE.bazel wires no toolchains_msvc backend, so merely
# adding the module fetches no restricted payloads on the seed host.
if ! grep -q -F -e 'toolchains_msvc' "$module"; then
  ok
else
  bad "MODULE.bazel wires toolchains_msvc (want no backend dep: adding the module must fetch nothing, issue #496)"
fi

# Rights laziness proof: the committed lock carries no Windows MSVC payload.
if [[ -f "$lock" ]] && ! grep -q -F -e 'toolchains_msvc' "$lock"; then
  ok
else
  bad "MODULE.bazel.lock carries a toolchains_msvc payload (want none: unrelated workflows fetch nothing, issue #496)"
fi

# Rights proof: CI sets no EULA variable and leaks no secrets in the
# Windows jobs; missing acceptance leaves unrelated workflows green.
if ! grep -rn -F -e 'BAZEL_TOOLCHAINS_MSVC_ACCEPT' --include='*.yml' .github/ 2>/dev/null | grep -v -F -e 'acquisition_rights_qualification.sh' | grep -v -F -e 'windows_acquisition_qualification.sh' | grep -v -F -e 'windows_qualification.sh' | grep -q . &&
  ! grep -A30 -e 'build-windows-x86_64' "$ci" | grep -E -e 'secrets\.' | grep -q .; then
  ok
else
  bad "windows jobs set the EULA variable or leak secrets (want deliberate acceptance only, missing acceptance leaves unrelated workflows green, issue #496)"
fi

# Native plan owns the qualified rights-review record plus fixture proof.
if grep -q -F -e 'qualified seed-only under issue #496' "$native" &&
  grep -q -F -e 'acquisition_rights_qualification' "$native" &&
  grep -q -F -e 'cc/tests/fixtures/acquisition_rights/pins.bzl' "$native" &&
  grep -q -F -e 'Are Apple/Microsoft acquisition and cache rights adequate?' "$native" &&
  grep -q -F -e 'usage vs redistribution reviewed separately' "$native"; then
  ok
else
  bad "docs/native-toolchains.md lost its qualified rights-review record with fixtures under issue #496"
fi

# Support matrix owns the qualified rights-review record with no Supported claim.
if grep -q -F -e 'qualified seed-only under issue #496' "$matrix" &&
  grep -q -F -e 'acquisition_rights_qualification' "$matrix" &&
  grep -q -F -e 'cc/tests/fixtures/acquisition_rights/pins.bzl' "$matrix" &&
  grep -q -F -e 'usage vs redistribution reviewed separately' "$matrix"; then
  ok
else
  bad "docs/product/support-matrix.md lost its qualified rights-review record under issue #496"
fi

# BUILD owns the harness target plus CI wires it in dogfood-freshness.
if grep -q -F -e 'name = "acquisition_rights_qualification"' "$build" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:acquisition_rights_qualification' "$ci"; then
  ok
else
  bad "tools/ci/BUILD.bazel or ci.yml lost the acquisition_rights_qualification wiring (want target plus dogfood-freshness)"
fi

# Verification matrix owns the qualified seed-only record under.
if grep -q -F -e 'acquisition_rights_qualification' "$verify" &&
  grep -q -F -e 'qualified seed-only under #496' "$verify" &&
  grep -q -F -e 'bazel run //tools/ci:acquisition_rights_qualification' "$verify" &&
  grep -q -F -e '`acquisition_rights_qualification` 15/15' "$verify"; then
  ok
else
  bad "verification-matrix lost its #496 rights-review qualified record"
fi

# Live proof: missing acceptance leaves unrelated workflows green (the seed
# hello builds with the EULA variable unset, fetching no Windows payloads).
if [[ -z "${BAZEL_TOOLCHAINS_MSVC_ACCEPT_MICROSOFT_VISUAL_STUDIO_BUILDTOOLS_EULA:-}" ]] &&
  bazel build //cc/tests/fixtures/hello:hello --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "seed hello failed without EULA acceptance (want unrelated workflows green with missing acceptance, issue #496)"
fi

dx_test_summary "acquisition rights qualification harness"
