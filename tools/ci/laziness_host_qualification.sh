#!/usr/bin/env bash
# Per-host strict-laziness plus Windows host-compat qualification.
#
# Qualifies the three scope items of issue #917 with fixture evidence,
# without claiming a qualified hermetic backend, per-host execution, or
# Supported:
# - per-host no-fetch in the consumer graph: adding an unused foundation
#   pulls no EULA/SDK payload on any required host; override-matrix
#   (default plus root-override graphs stay lazy with no special CLI
#   policy); Bzlmod eager-resolution boundary (version-resolution cost,
#   not payload; lock size is metadata; extension manifests fetch during
#   evaluation; deferred failure never proves laziness);
# - symlink-privilege UX: probe before mutation with an actionable error
#   (Developer Mode or SeBackupPrivilege guidance); Enterprise hosts that
#   cannot grant it are unsupported with failure before mutation, never a
#   fallback; junction/launcher/copy fallback evaluated and rejected
#   (breaks atomic replacement and ownership validation);
# - backend adoption: hermetic-llvm Apple-SDK plus toolchains_msvc
#   clang-cl/Microsoft-STL stay provisional with immutable
#   lazy fetch; hosts are Platform-qualified with an explicit
#   provisional qualifier, never a shared status; full adoption stays
#   owned under issues #494-#505 with release evidence under #803-#807.
# Seed-executed with static per-host pins; no Supported claim.
#
# Versioned here, run by CI via `bazel run //tools/ci:laziness_host_qualification`,
# following //tools/ci:windows_eula_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

dx_bash_pin

pins="cc/tests/fixtures/laziness_host/pins.bzl"
pins_build="cc/tests/fixtures/laziness_host/BUILD.bazel"
expected="cc/tests/fixtures/laziness_host/laziness_host.expected"
module="MODULE.bazel"
lock="MODULE.bazel.lock"
env_lib="cli/env/src/lib.rs"
managed="docs/environments/managed-state.md"
arch="docs/architecture/lifecycle.md"
adr14="docs/decisions/0014-tested-platform-release-stack.md"
native="docs/native-toolchains.md"
matrix="docs/product/support-matrix.md"
targets_d="tools/ci/ci_targets_d.bzl"
freshness="tools/ci/dogfood_freshness.sh"
laziness_query="tools/ci/examples_laziness_query.sh"

# Fixture set stays present.
if [[ -f "$pins" && -f "$pins_build" && -f "$expected" ]]; then
  ok
else
  bad "laziness host fixture missing (want $pins plus $pins_build plus $expected)"
fi

# Pins record the Bzlmod eager-resolution boundary.
if grep -q -F -e 'version-resolution cost' "$pins" &&
  grep -q -F -e 'MODULE.bazel.lock size is version-resolution metadata' "$pins" &&
  grep -q -F -e 'fixed-manifest plus package-index inputs' "$pins" &&
  grep -q -F -e 'deferred EULA failure never proves laziness' "$pins" &&
  grep -q -F -e 'extension evaluation already fetches manifests' "$pins"; then
  ok
else
  bad "pins.bzl lost its Bzlmod eager-resolution boundary under issue #917"
fi

# Pins record per-host no-fetch in the consumer graph plus override matrix.
if grep -q -F -e 'linux_x86_64' "$pins" &&
  grep -q -F -e 'windows_x86_64' "$pins" &&
  grep -q -F -e 'examples/adopt-rust' "$pins" &&
  grep -q -F -e 'adding unused foundation pulls no EULA payload' "$pins" &&
  grep -q -F -e 'adding unused foundation pulls no SDK payload' "$pins" &&
  grep -q -F -e 'default graph stays lazy' "$pins" &&
  grep -q -F -e 'root-override graph stays lazy' "$pins" &&
  grep -q -F -e 'no special CLI policy for overrides' "$pins"; then
  ok
else
  bad "pins.bzl lost its per-host consumer-graph no-fetch plus override-matrix record under issue #917"
fi

# Pins record symlink probe plus guidance plus Enterprise unsupported plus no fallback.
if grep -q -F -e 'probe_symlink' "$pins" &&
  grep -q -F -e 'SymlinkUnsupported' "$pins" &&
  grep -q -F -e 'enable Developer Mode or grant SeBackupPrivilege' "$pins" &&
  grep -q -F -e 'hosts that cannot grant it are unsupported' "$pins" &&
  grep -q -F -e 'no junction or copy fallback' "$pins" &&
  grep -q -F -e 'fallback breaks atomic replacement and ownership validation' "$pins"; then
  ok
else
  bad "pins.bzl lost its symlink probe plus Enterprise-unsupported plus no-fallback record under issue #917"
fi

# Pins record provisional backends with qualifier plus owned gaps and no Supported claim.
if grep -q -F -e 'hermetic-llvm Apple-SDK provisional' "$pins" &&
  grep -q -F -e 'toolchains_msvc clang-cl/Microsoft-STL provisional' "$pins" &&
  grep -q -F -e 'provisional-backend exception' "$pins" &&
  grep -q -F -e 'no Supported claim' "$pins" &&
  grep -q -F -e 'issues #494-#505 plus release evidence #803-#807' "$pins"; then
  ok
else
  bad "pins.bzl lost its provisional-backend plus qualifier plus owned-gap record under issue #917"
fi

# Expected fixture pins all three scope items.
if grep -q -F -e 'version-resolution cost' "$expected" &&
  grep -q -F -e 'adding unused foundation pulls no EULA/SDK payload' "$expected" &&
  grep -q -F -e 'probe_symlink before mutation' "$expected" &&
  grep -q -F -e 'fallback breaks atomic replacement and ownership validation' "$expected" &&
  grep -q -F -e 'stay provisional with provisional-backend exception' "$expected" &&
  grep -q -F -e 'no Supported claim' "$expected"; then
  ok
else
  bad "laziness_host.expected lost its boundary plus no-fetch plus symlink plus backend lines under #917"
fi

# Laziness proof: MODULE.bazel wires no toolchains_msvc backend, so merely
# adding the module fetches no restricted payloads on any host.
if ! grep -q -F -e 'toolchains_msvc' "$module"; then
  ok
else
  bad "MODULE.bazel wires toolchains_msvc (want no backend dep: adding the module must fetch nothing, issue #917)"
fi

# Laziness proof: the committed lock carries no Windows MSVC payload.
if [[ -f "$lock" ]] && ! grep -q -F -e 'toolchains_msvc' "$lock"; then
  ok
else
  bad "MODULE.bazel.lock carries a toolchains_msvc payload (want none: unused foundations fetch nothing, issue #917)"
fi

# Symlink UX proof: the env installer probes before mutation with Developer
# Mode guidance and carries no junction fallback
# (hermetic tree search: host grep -rn variance, issue #1006).
if grep -q -F -e 'pub fn probe_symlink' "$env_lib" &&
  grep -q -F -e 'SymlinkUnsupported' "$env_lib" &&
  grep -q -F -e 'enable Developer Mode or grant SeBackupPrivilege' "$env_lib" &&
  dx_tree_absent 'junction' -- cli/env/src/; then
  ok
else
  bad "cli/env/src lost its probe_symlink plus SymlinkUnsupported plus Developer-Mode guidance with no junction fallback under issue #917"
fi

# Managed state stays symlink-only with Enterprise-unsupported refusal before mutation.
if grep -q -F -e 'There is no launcher, junction, copy' "$managed" &&
  grep -q -F -e 'missing capability fails before mutation' "$managed" &&
  grep -q -F -e 'hosts that cannot grant it are unsupported' "$managed" &&
  grep -q -F -e 'fallback would break atomic' "$managed"; then
  ok
else
  bad "docs/environments/managed-state.md lost its symlink-only plus Enterprise-unsupported plus fallback-rejection record under issue #917"
fi

# Architecture activation owns the laziness boundary with deferred-failure caveat.
if grep -q -F -e 'deferred EULA' "$arch" &&
  grep -q -F -e 'never proves laziness' "$arch" &&
  grep -q -F -e 'Accepted fit (2026-09-21, #959)' "$arch"; then
  ok
else
  bad "docs/architecture/lifecycle.md lost its activation plus deferred-failure plus #959 fit record (want boundary, issue #917)"
fi

# ADR 0014 owns the strictly-lazy decision with the version-resolution boundary.
if grep -q -F -e 'strictly lazy' "$adr14" &&
  grep -q -F -e 'MODULE.bazel.lock' "$adr14" &&
  grep -q -F -e 'version-resolution' "$adr14" &&
  grep -q -F -e 'Accepted fit 2026-09-21 (#959)' "$adr14"; then
  ok
else
  bad "docs/decisions/0014-tested-platform-release-stack.md lost its strictly-lazy plus version-resolution boundary (want decision, issue #917)"
fi

# Native plan owns the qualified per-host laziness record with fixture proof.
if grep -q -F -e 'qualified seed-only under issue #917' "$native" &&
  grep -q -F -e 'laziness_host_qualification' "$native" &&
  grep -q -F -e 'cc/tests/fixtures/laziness_host/pins.bzl' "$native" &&
  grep -q -F -e 'Backend stays provisional' "$native"; then
  ok
else
  bad "docs/native-toolchains.md lost its qualified per-host laziness record with fixtures under issue #917"
fi

# Consumer-graph proof rides the per-foundation laziness slices still wired in CI.
if grep -q -F -e 'adopt-rust' "$laziness_query" &&
  grep -q -F -e 'forbidden marker' "$laziness_query" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:examples_laziness_query' "$freshness" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:examples_laziness_runtime' "$freshness"; then
  ok
else
  bad "examples laziness consumer-graph slices lost their adopt plus forbidden-marker wiring (want per-foundation isolation, issue #917)"
fi

# BUILD owns the harness target plus dogfood-freshness wires it.
if grep -q -F -e 'name = "laziness_host_qualification"' "$targets_d" &&
  grep -q -F -e 'laziness_host_qualification.sh' "$targets_d" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:laziness_host_qualification' "$freshness"; then
  ok
else
  bad "tools/ci/ci_targets_d.bzl or dogfood_freshness.sh lost the laziness_host_qualification wiring (want target plus freshness)"
fi

# Live proof: missing acceptance leaves unrelated workflows green (the seed
# hello builds with the EULA variable unset, fetching no Windows payloads).
if [[ -z "${BAZEL_TOOLCHAINS_MSVC_ACCEPT_MICROSOFT_VISUAL_STUDIO_BUILDTOOLS_EULA:-}" ]] &&
  bazel build //cc/tests/fixtures/hello:hello --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "seed hello failed without EULA acceptance (want unrelated workflows green with missing acceptance, issue #917)"
fi

dx_test_summary "laziness host harness"
