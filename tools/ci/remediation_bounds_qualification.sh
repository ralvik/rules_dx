#!/usr/bin/env bash
# Bounded remediation qualification harness.
#
# Defines plus proves the bounded-remediation slice of the native baseline
# with fixture evidence, without claiming a qualified hermetic-llvm backend,
# qualified coverage beyond, qualified floors beyond,
# qualified routes beyond, or Supported:
# - bounded definition: reproduce each defect, estimate the upstream fix,
#   name the actual owner, record patch plus upstream-issue plus upgrade
#   tracking with complete-workflow evidence; scope only.
# - allowed forms: focused tested pinned patches, bounded integration,
#   small missing pieces with clearly bounded scope plus maintenance
#   responsibility, preserving upstream semantics plus verified provider
#   boundaries, prefer upstreaming, compare established alternatives with
#   explicit contract plus API review before adoption.
# - defect inventory: CC opt-out linker plus Windows transport rebasing
#   plus ABI constraint plus Windows acquisition plus coverage workaround
#   plus generation strictness plus IDE snapshot plus PIE plus
#   ELF-dependency plus glibc-symbol via existing upstream constraints
#   plus cross-product expansion only after the cohort passes; affected
#   capability fails closed until a compliant remedy passes evidence.
# - owner plus tracking: actual owners named per defect with narrow
#   estimates plus patch plus upstream-issue plus upgrade tracking.
# - stop slice: replacement acquisition engine, compiler backend, Cargo
#   graph or coverage engine stops the slice plus revisits alternatives;
#   never defer Rust Windows support, prematurely admit C/C++, or weaken
#   hermeticity.
# - cross-product: never implement missing infra merely to fill the
#   cross-product; Linux cross first priority not every-target mandate;
#   broader routes only when upstream config keeps maintenance bounded.
# - rejected: unbounded fork plus engine replacements plus owned backend
#   plus substitutions plus fallbacks plus cross-product infra fill plus
#   weakened natives.
# - open with honest records: backends stay provisional, floors qualified
# seed-only, coverage qualified seed-only under issue
# , corpus qualified seed-only, routes qualified
# seed-only.
#
# Versioned here, run by CI via `bazel run //tools/ci:remediation_bounds_qualification`,
# following //tools/ci:cross_routes_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

pins="cc/tests/fixtures/remediation_bounds/pins.bzl"
pins_build="cc/tests/fixtures/remediation_bounds/BUILD.bazel"
bounds="cc/tests/fixtures/remediation_bounds/bounds.expected"
defects="cc/tests/fixtures/remediation_bounds/defects.txt"
owners="cc/tests/fixtures/remediation_bounds/owners.txt"
native="docs/native-toolchains.md"
matrix="docs/product/support-matrix.md"
build="tools/ci/BUILD.bazel"
ci=".github/workflows/ci.yml"
verify="docs/testing/verification-matrix.md"

# Fixture files stay present.
if [[ -f "$pins" && -f "$pins_build" && -f "$bounds" && -f "$defects" && -f "$owners" ]]; then
  ok
else
  bad "bounded remediation fixture missing (want $pins plus $pins_build plus bounds.expected plus defects.txt plus owners.txt)"
fi

# Pins record the bounded-remediation definition with scope only.
if grep -q -F -e 'Reproduce defects, estimate each upstream fix, name actual owners, record patch plus upstream issue plus upgrade tracking and complete-workflow evidence' "$pins" &&
  grep -q -F -e 'complete-workflow evidence' "$pins" &&
  grep -q -F -e 'Scope only' "$pins"; then
  ok
else
  bad "pins.bzl lost its bounded-remediation definition plus scope-only under issue #505"
fi

# Pins record the allowed remediation forms with upstreaming plus review.
if grep -q -F -e 'Focused upstream rules patches are allowed when pinned reproducibly and tested against the accepted contracts' "$pins" &&
  grep -q -F -e 'focused, tested, pinned patches' "$pins" &&
  grep -q -F -e 'bounded integration' "$pins" &&
  grep -q -F -e 'small missing integration pieces with clearly bounded scope and maintenance responsibility' "$pins" &&
  grep -q -F -e 'preserving upstream language semantics and verified public provider boundaries' "$pins" &&
  grep -q -F -e 'Prefer upstreaming those fixes' "$pins" &&
  grep -q -F -e 'compare established alternative upstream rules before considering replacement' "$pins" &&
  grep -q -F -e 'explicit contract plus API review before adoption' "$pins"; then
  ok
else
  bad "pins.bzl lost its allowed remediation forms plus upstreaming plus review under issue #505"
fi

# Pins record the CC opt-out plus transport plus ABI defects with owners and evidence.
if grep -q -F -e 'kept CC opt-out linker failure path with sysroot rust-lld fallback plus no_cc stubs' "$pins" &&
  grep -q -F -e 'cargo/private/cargo_build_script.bzl no-linker failure path' "$pins" &&
  grep -q -F -e 'rust/tests/fixtures/cc_optout/ via cc_optout_qualification under issue #471' "$pins" &&
  grep -q -F -e 'Windows transport rebasing with /external:I plus .lib plus .obj bare-path handling' "$pins" &&
  grep -q -F -e 'Windows ABI constraint fix so toolchains_msvc cannot satisfy GNU plus GNULVM targets' "$pins" &&
  grep -q -F -e 'private/msvc_toolchains_repo.bzl plus rs/platforms/triples.bzl LLVM MSVC ABI identity' "$pins" &&
  grep -q -F -e 'cc/tests/fixtures/windows_transport/pins.bzl via windows_transport_qualification under issue #497' "$pins"; then
  ok
else
  bad "pins.bzl lost its CC opt-out plus transport plus ABI defects with owners and evidence under issue #505"
fi

# Pins record the acquisition plus coverage plus generation plus IDE defects with evidence.
if grep -q -F -e 'Windows acquisition fixed-manifest plus package-index inputs' "$pins" &&
  grep -q -F -e 'private/vs_channel_manifest.bzl live VS channel manifests plus minor-version selectors' "$pins" &&
  grep -q -F -e 'cc/tests/fixtures/windows_acquisition/pins.bzl via windows_acquisition_qualification under issue #495' "$pins" &&
  grep -q -F -e 'Coverage workaround with Bazel 9 LLVM flags plus raw-output workaround plus llvm-cov plus llvm-profdata selection' "$pins" &&
  grep -q -F -e 'cc/tests/fixtures/lcov_accounting/pins.bzl via lcov_accounting_qualification under issue #501' "$pins" &&
  grep -q -F -e 'Generation strictness with gazelle_cc resolve plus module index plus test grouping plus generated headers plus assembly plus modules plus PCH' "$pins" &&
  grep -q -F -e 'cc/tests/fixtures/strict_generation/pins.bzl via strict_generation_qualification under issue #503' "$pins" &&
  grep -q -F -e 'IDE snapshot adaptation with action-derived commands plus managed clangd plus generated-output materialization plus multi-context headers' "$pins" &&
  grep -q -F -e 'exact-target discovery qualified seed-only under issue #475 with C++ snapshot staying open proof' "$pins"; then
  ok
else
  bad "pins.bzl lost its acquisition plus coverage plus generation plus IDE defects with evidence under issue #505"
fi

# Pins record the PIE plus ELF plus glibc-symbol bound via existing upstream constraints.
if grep -q -F -e 'PIE plus ELF-dependency plus glibc-symbol uses existing upstream constraints' "$pins" &&
  grep -q -F -e 'PIE qualification uses existing upstream constraints' "$pins" &&
  grep -q -F -e 'glibc 2.28 symbol floor plus musl 1.2.6 static closure plus deployment 14.0 qualified seed-only under issue #500' "$pins" &&
  grep -q -F -e 'Cross-product expansion only after the initial cohort passes with bounded upstream configuration' "$pins" &&
  grep -q -F -e 'cc/tests/fixtures/cross_routes/pins.bzl via cross_routes_qualification under issue #504' "$pins" &&
  grep -q -F -e 'the affected capability fails closed until a compliant remedy passes the required evidence' "$pins"; then
  ok
else
  bad "pins.bzl lost its PIE plus ELF plus glibc-symbol plus cross-product plus fail-closed bound under issue #505"
fi

# Pins record the owner plus estimate plus tracking contract.
if grep -q -F -e 'name actual owners' "$pins" &&
  grep -q -F -e 'estimate each upstream fix' "$pins" &&
  grep -q -F -e 'record patch plus upstream issue plus upgrade tracking' "$pins" &&
  grep -q -F -e 'hermeticbuild/rules_rust' "$pins" &&
  grep -q -F -e 'toolchains_msvc plus patched rules_rust' "$pins"; then
  ok
else
  bad "pins.bzl lost its owner plus estimate plus tracking contract under issue #505"
fi

# Pins record the stop-slice bound with no shortcuts.
if grep -q -F -e 'If fixes require a replacement acquisition engine, compiler backend, Cargo graph or coverage engine, stop that slice and revisit alternatives under the maintenance policy' "$pins" &&
  grep -q -F -e 'replacement acquisition engine' "$pins" &&
  grep -q -F -e 'Cargo graph' "$pins" &&
  grep -q -F -e 'coverage engine' "$pins" &&
  grep -q -F -e 'Do not defer required Rust Windows support, admit complete C/C++ prematurely, or weaken hermeticity to make the table green' "$pins"; then
  ok
else
  bad "pins.bzl lost its stop-slice plus no-shortcut bound under issue #505"
fi

# Pins record the cross-product bound plus the rejected unbounded fork.
if grep -q -F -e 'Do not implement missing infra merely to fill cross-product' "$pins" &&
  grep -q -F -e 'Linux cross first priority not mandate every target from every host' "$pins" &&
  grep -q -F -e 'Broader cross-builds are desirable when upstream configuration keeps maintenance bounded' "$pins" &&
  grep -q -F -e '"unbounded fork"' "$pins" &&
  grep -q -F -e '"replacement language engine"' "$pins" &&
  grep -q -F -e '"project-owned compiler backend around windows_support"' "$pins" &&
  grep -q -F -e '"cargo-zigbuild substitution"' "$pins" &&
  grep -q -F -e '"missing infra to fill cross-product"' "$pins"; then
  ok
else
  bad "pins.bzl lost its cross-product bound plus unbounded-fork rejection under issue #505"
fi

# Fixture bounds plus defects plus owners texts cover the full bound.
if grep -q -F -e 'Reproduce defects, estimate each upstream fix, name actual owners, record patch plus upstream issue plus upgrade tracking and complete-workflow evidence' "$bounds" &&
  grep -q -F -e 'If fixes require a replacement acquisition engine, compiler backend, Cargo graph or coverage engine, stop that slice and revisit alternatives under the maintenance policy' "$bounds" &&
  grep -q -F -e 'Do not implement missing infra merely to fill cross-product' "$bounds" &&
  grep -q -F -e 'kept CC opt-out linker failure path with sysroot rust-lld fallback plus no_cc stubs' "$defects" &&
  grep -q -F -e 'Windows transport rebasing with /external:I plus .lib plus .obj bare-path handling' "$defects" &&
  grep -q -F -e 'PIE plus ELF-dependency plus glibc-symbol uses existing upstream constraints' "$defects" &&
  grep -q -F -e 'name actual owners' "$owners" &&
  grep -q -F -e 'hermeticbuild/rules_rust' "$owners" &&
  grep -q -F -e 'Do not defer required Rust Windows support, admit complete C/C++ prematurely, or weaken hermeticity to make the table green' "$owners"; then
  ok
else
  bad "bounds.expected plus defects.txt plus owners.txt lost bound coverage (want definition plus stop-slice plus cross-product plus defects plus owners, issue #505)"
fi

# Native plan owns the qualified bounded-remediation record plus fixture proof.
if grep -q -F -e 'qualified seed-only under issue #505' "$native" &&
  grep -q -F -e 'remediation_bounds_qualification' "$native" &&
  grep -q -F -e 'cc/tests/fixtures/remediation_bounds/pins.bzl' "$native" &&
  grep -q -F -e 'Is the remediation bounded enough for admission?' "$native" &&
  grep -q -F -e 'Do not implement missing infra merely to fill cross-product' "$native"; then
  ok
else
  bad "docs/native-toolchains.md lost its qualified bounded-remediation record with fixtures under issue #505"
fi

# Support matrix owns the qualified bounded-remediation record with no Supported claim.
if grep -q -F -e 'qualified seed-only under issue #505' "$matrix" &&
  grep -q -F -e 'remediation_bounds_qualification' "$matrix" &&
  grep -q -F -e 'cc/tests/fixtures/remediation_bounds/pins.bzl' "$matrix" &&
  grep -q -F -e 'Do not implement missing infra merely to fill cross-product' "$matrix"; then
  ok
else
  bad "docs/product/support-matrix.md lost its qualified bounded-remediation record under issue #505"
fi

# BUILD owns the harness target plus CI wires it in dogfood-freshness.
if grep -q -F -e 'name = "remediation_bounds_qualification"' "$build" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:remediation_bounds_qualification' "$ci"; then
  ok
else
  bad "tools/ci/BUILD.bazel or ci.yml lost the remediation_bounds_qualification wiring (want target plus dogfood-freshness)"
fi

# Verification matrix owns the qualified seed-only record under.
if grep -q -F -e 'remediation_bounds_qualification' "$verify" &&
  grep -q -F -e 'qualified seed-only under #505' "$verify" &&
  grep -q -F -e 'bazel run //tools/ci:remediation_bounds_qualification' "$verify" &&
  grep -q -F -e '`remediation_bounds_qualification` 16/16' "$verify"; then
  ok
else
  bad "verification-matrix lost its #505 bounded remediation qualified record"
fi

# Live proof: the seed hello plus the fixture corpus build green on the
# seed host with no backend, coverage, or cross-host requirement.
if bazel build //cc/tests/fixtures/hello:hello //cc/tests/fixtures/remediation_bounds/... --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "seed hello plus remediation bounds fixture build failed (want green on the seed host, issue #505)"
fi

# Live proof: the Linux corpus plus floors fixtures stay green, proving
# the referenced slices without double-claiming them here.
if bazel build //cc/tests/fixtures/linux_corpus:corpus //cc/tests/fixtures/deployment_floors:corpus_starlark --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "linux corpus plus floors proof failed (want corpus plus floors green, issue #505)"
fi

dx_test_summary "bounded remediation qualification harness"
