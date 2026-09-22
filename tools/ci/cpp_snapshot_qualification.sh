#!/usr/bin/env bash
# C++ exact-target snapshot qualification harness.
#
# Qualifies the C++ slice of the exact-target discovery gap left open by
# issue #475 (Rust `gen_rust_project`/`flycheck` exact isolation only):
# resolver-owned exact labels stay the discovery input, while the C++
# snapshot is action-derived with generated sources plus multi-context
# headers plus managed host tools plus Bazel-9 compatibility.
# - decided: resolver-owned exact labels flow to the snapshot input,
#   preserving exact context. Files resolve through one unconfigured
#   `bazel query` (`kind('rule', rdeps(//..., set(...), 1))`, depth
#   exactly 1, deterministic sorted set, nearest enclosing package file
#   labels); dirs become recursive `//path/...` patterns without
#   filesystem enumeration; explicit labels pass through untouched with
#   no query. Those exact owners are the snapshot input, never raw paths
#   to package-wide widening.
# - snapshot shape: compile commands come from Bazel action inspection
#   (`bazel aquery` `CppCompile` with command line plus inputs), never
#   inferred from `CcInfo`. Generated outputs materialize through exact
#   mappings, never inferred producers. One header keeps each target
#   context apart (multi-context), never merged to one entry. Host-native
#   clangd resolves from the qualified toolchain, never unrestricted
#   query-driver execution. Proof runs on Bazel 9.2.0 plus rules_cc 0.2.22.
# - fixtures: `cc/tests/fixtures/cpp_snapshot/pins.bzl` pins the
#   identities; `cc/tests/fixtures/hello` proves exact isolation
#   (`hello.cc` to `hello_lib` only, `main.cc` to `hello` only) plus
#   multi-context headers (`hello.h` in lib plus bin plus test compiles)
#   plus action-derived commands (`CppCompile` via aquery).
# - rejected: `CcInfo` inference, unrestricted query-driver, refresh
#   running Bazel plus preprocessors, workspace writes, changing
#   extraction features, continued-after-failures, package-wide widening.
# - open owned gaps: platform plus consumer plus release evidence, no
#   `Supported` claim. Compatibility is resolution plus snapshot only.
#
# Versioned here, run by CI via `bazel run //tools/ci:cpp_snapshot_qualification`,
# following //tools/ci:exact_target_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

pins="cc/tests/fixtures/cpp_snapshot/pins.bzl"
fixture_build="cc/tests/fixtures/cpp_snapshot/BUILD.bazel"
snapshot="cc/tests/fixtures/cpp_snapshot/snapshot.expected"
targets="cc/tests/fixtures/cpp_snapshot/exact_targets.txt"
contract="docs/cli/target-resolution.md"
native="docs/native-toolchains.md"
matrix="docs/product/support-matrix.md"
remediation="cc/tests/fixtures/remediation_bounds/pins.bzl"
build="tools/ci/ci_targets_d.bzl"
ci="tools/ci/dogfood_freshness.sh"

# Fixture files stay present.
if [[ -f "$pins" && -f "$fixture_build" && -f "$snapshot" && -f "$targets" ]]; then
  ok
else
  bad "C++ snapshot fixture missing (want $pins plus $fixture_build plus snapshot.expected plus exact_targets.txt)"
fi

# Pins record the inspected snapshot plus native stack identities.
if grep -q -F -e 'HEDRON_EXTRACTOR_VERSION = "abb61a688167623088f8768cc9264798df6a9d10"' "$pins" &&
  grep -q -F -e 'refresh.template.py' "$pins" &&
  grep -q -F -e 'RULES_CC_VERSION = "0.2.22"' "$pins" &&
  grep -q -F -e 'BAZEL_VERSION = "9.2.0"' "$pins" &&
  grep -q -F -e 'uses actual action commands' "$pins"; then
  ok
else
  bad "pins.bzl lost its Hedron plus rules_cc plus Bazel snapshot identities under issue #754"
fi

# Pins record the resolver-owned exact-target expression plus owners.
if grep -q -F -e "kind('rule', rdeps(//..., set(<file-labels>), 1))" "$pins" &&
  grep -q -F -e '//cc/tests/fixtures/hello:hello_lib' "$pins" &&
  grep -q -F -e '//cc/tests/fixtures/hello:hello' "$pins" &&
  grep -q -F -e 'cc/tests/fixtures/hello/hello.cc' "$pins" &&
  grep -q -F -e 'cc/tests/fixtures/hello/main.cc' "$pins"; then
  ok
else
  bad "pins.bzl lost its resolver-owned exact-target expression plus hello owners under issue #754"
fi

# Pins record the action-derived shape with CcInfo rejection.
if grep -q -F -e 'action-derived CppCompile commands via aquery' "$pins" &&
  grep -q -F -e 'infer compile commands from CcInfo' "$pins"; then
  ok
else
  bad "pins.bzl lost its action-derived snapshot shape plus CcInfo rejection under issue #754"
fi

# Pins record generated plus multi-context plus managed plus Bazel-9.
if grep -q -F -e 'generated-output materialization through exact mappings' "$pins" &&
  grep -q -F -e 'multiple header contexts kept apart per target' "$pins" &&
  grep -q -F -e 'managed host-native clangd from the qualified toolchain' "$pins" &&
  grep -q -F -e 'unrestricted clangd query-driver execution' "$pins" &&
  grep -q -F -e 'Bazel-9 compatibility on Bazel 9.2.0 plus rules_cc 0.2.22' "$pins"; then
  ok
else
  bad "pins.bzl lost its generated plus multi-context plus managed plus Bazel-9 contract under issue #754"
fi

# Pins record the raw-extractor gaps plus rejected substitutes.
if grep -q -F -e 'refresh runs Bazel plus preprocessors' "$pins" &&
  grep -q -F -e 'workspace writes' "$pins" &&
  grep -q -F -e 'continued-after-failures' "$pins" &&
  grep -q -F -e 'package-wide snapshot widening' "$pins"; then
  ok
else
  bad "pins.bzl lost its refresh-gap plus widening rejections under issue #754"
fi

# Fixture texts cover the exact plus snapshot contract.
if grep -q -F -e 'cc/tests/fixtures/hello/hello.cc -> //cc/tests/fixtures/hello:hello_lib' "$targets" &&
  grep -q -F -e 'cc/tests/fixtures/hello/main.cc -> //cc/tests/fixtures/hello:hello' "$targets" &&
  grep -q -F -e 'action-derived CppCompile commands via aquery' "$targets" &&
  grep -q -F -e 'action-derived CppCompile commands via aquery' "$snapshot" &&
  grep -q -F -e 'multiple header contexts kept apart per target' "$snapshot" &&
  grep -q -F -e 'managed host-native clangd from the qualified toolchain' "$snapshot"; then
  ok
else
  bad "exact_targets.txt plus snapshot.expected lost exact plus snapshot coverage (want hello owners plus action-derived plus headers plus managed, issue #754)"
fi

# Contract doc owns the C++ snapshot proof alongside the Rust proof.
if grep -q -F -e '## Exact-Target Discovery' "$contract" &&
  grep -q -F -e 'cpp_snapshot_qualification' "$contract" &&
  grep -q -F -e 'cc/tests/fixtures/cpp_snapshot/pins.bzl' "$contract" &&
  grep -q -F -e 'infer compile commands from CcInfo' "$contract" &&
  grep -q -F -e 'issue #754' "$contract"; then
  ok
else
  bad "docs/cli/target-resolution.md lost its C++ snapshot proof with fixtures under issue #754"
fi

# Native plan owns the qualified C++ snapshot record plus the question row.
if grep -q -F -e 'qualified seed-only under issue #754' "$native" &&
  grep -q -F -e 'cpp_snapshot_qualification' "$native" &&
  grep -q -F -e 'cc/tests/fixtures/cpp_snapshot/pins.bzl' "$native" &&
  grep -q -F -e 'Can IDE setup preserve exact context' "$native" &&
  grep -q -F -e 'issue #754' "$native"; then
  ok
else
  bad "docs/native-toolchains.md lost its qualified C++ snapshot record with fixtures plus pins under issue #754"
fi

# Remediation bounds no longer leaves the C++ snapshot open under #475 alone.
if grep -q -F -e 'issue #754' "$remediation" &&
  grep -q -F -e 'IDE snapshot adaptation with action-derived commands plus managed clangd plus generated-output materialization plus multi-context headers' "$remediation"; then
  ok
else
  bad "remediation_bounds pins.bzl lost its #754 C++ snapshot evidence (want IDE defect owned by #754, not open under #475)"
fi

# BUILD owns the harness target.
if grep -q -F -e 'name = "cpp_snapshot_qualification"' "$build"; then
  ok
else
  bad "tools/ci/ci_targets_d.bzl lost the cpp_snapshot_qualification target"
fi

# CI wires the harness in dogfood-freshness.
if grep -q -F -e 'bazel run --noshow_progress //tools/ci:cpp_snapshot_qualification' "$ci"; then
  ok
else
  bad "dogfood_freshness.sh lost the cpp_snapshot_qualification step (want dogfood-freshness)"
fi

# Live proof: exact isolation holds (lib source to lib only, bin source to bin only).
lib_owners="$(bazel query "kind('rule', rdeps(//cc/tests/fixtures/hello/..., set(//cc/tests/fixtures/hello:hello.cc), 1))" 2>/dev/null || true)"
bin_owners="$(bazel query "kind('rule', rdeps(//cc/tests/fixtures/hello/..., set(//cc/tests/fixtures/hello:main.cc), 1))" 2>/dev/null || true)"
if echo "$lib_owners" | grep -q -x -F -e '//cc/tests/fixtures/hello:hello_lib' &&
  ! echo "$lib_owners" | grep -q -x -F -e '//cc/tests/fixtures/hello:hello' &&
  echo "$bin_owners" | grep -q -x -F -e '//cc/tests/fixtures/hello:hello' &&
  ! echo "$bin_owners" | grep -q -x -F -e '//cc/tests/fixtures/hello:hello_lib'; then
  ok
else
  bad "C++ exact isolation failed (want hello.cc to hello_lib only plus main.cc to hello only, not widened)"
fi

# Live proof: snapshot is action-derived (CppCompile via aquery with command plus inputs).
aquery_out="$(bazel aquery "mnemonic(CppCompile, //cc/tests/fixtures/hello/...)" 2>/dev/null || true)"
if echo "$aquery_out" | grep -q -F -e 'Mnemonic: CppCompile' &&
  echo "$aquery_out" | grep -q -F -e 'Command Line:' &&
  echo "$aquery_out" | grep -q -F -e 'cc/tests/fixtures/hello/hello.cc' &&
  echo "$aquery_out" | grep -q -F -e 'cc/tests/fixtures/hello/hello.h'; then
  ok
else
  bad "action-derived snapshot failed (want CppCompile via aquery with command plus hello.cc plus hello.h, not CcInfo inference)"
fi

# Live proof: the seed hello plus the snapshot plus strict fixtures build green.
if bazel build //cc/tests/fixtures/hello:hello //cc/tests/fixtures/cpp_snapshot/... //cc/tests/fixtures/strict_generation:strict --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "seed hello plus cpp_snapshot plus strict build failed (want green on the seed host, issue #754)"
fi

dx_test_summary "C++ snapshot qualification harness"
