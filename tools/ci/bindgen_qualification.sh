#!/usr/bin/env bash
# Bindgen LLVM-22-vs-23 compat qualification harness.
#
# Qualifies the owned gap from closed: unpinned LLVM rejected, both
# LLVM identities pinned with a fixture proof of identical bindings.
# - pinned: the parser baseline is LLVM-22 (`rules_rs` v0.0.109 declaring
#   LLVM rules `0.8.18`/LLVM `22.1.8`); the qualified header/flag target is
#   LLVM-23 (`hermetic-llvm` v0.8.19/LLVM `23.1.0`); self-contained bindgen
#   `0.72.1` prebuilts (`v0.0.2`, six platform sha256s) link libclang
#   statically. All identities live in
#   `rust/tests/fixtures/bindgen/pins.bzl` and the native plan.
# - routes: the standalone `rust_bindgen` route derives the target compiler
#   context with `--no-include-path-detection --formatter=none` plus
#   explicit flags while the Rust consumer separately links the native
#   library; the build-script route needs an explicit execution-platform
#   libclang closure and target parsing flags.
# - fixtures: `rust/tests/fixtures/bindgen/bindgen.h` (C11-only stable
#   constructs; C23-only spellings excluded) plus `bindgen.expected` must
#   yield the identical 15-symbol set under both LLVM identities, with
#   `gcc -fsyntax-only -std=c11 -Wall -Werror` proving the header
#   well-formed on the seed host.
# - open owned gaps: platform plus consumer plus release evidence, no
#   `Supported` claim.
#
# Versioned here, run by CI via `bazel run //tools/ci:bindgen_qualification`,
# following //tools/ci:shell_env_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

dx_bash_pin

header="rust/tests/fixtures/bindgen/bindgen.h"
expected="rust/tests/fixtures/bindgen/bindgen.expected"
pins="rust/tests/fixtures/bindgen/pins.bzl"
fixture_build="rust/tests/fixtures/bindgen/BUILD.bazel"
native="docs/native-toolchains.md"
matrix="docs/product/support-matrix.md"
contract="docs/generation/rust.md"
gen_readme="docs/generation/foundation-qualification.md"
build="tools/ci/BUILD.bazel"
ci=".github/workflows/ci.yml"

# Fixture pair plus pins plus fixture BUILD stay present.
if [[ -f "$header" && -f "$expected" && -f "$pins" && -f "$fixture_build" ]]; then
  ok
else
  bad "bindgen fixture pair missing (want $header plus $expected plus $pins plus $fixture_build)"
fi

# Pins record the exact LLVM-22 baseline vs LLVM-23 target identities.
if grep -q -F -e 'RULES_RS_VERSION = "v0.0.109"' "$pins" &&
  grep -q -F -e 'b55b132af0c9951807c926768e40222330348632' "$pins" &&
  grep -q -F -e 'LLVM_BASELINE_MODULE = "0.8.18"' "$pins" &&
  grep -q -F -e 'LLVM_BASELINE_LLVM = "22.1.8"' "$pins" &&
  grep -q -F -e 'LLVM_TARGET_MODULE = "0.8.19"' "$pins" &&
  grep -q -F -e 'LLVM_TARGET_LLVM = "23.1.0"' "$pins" &&
  grep -q -F -e '6314688712edf3a95f78642d80393868256b4ef2' "$pins"; then
  ok
else
  bad "pins.bzl lost its LLVM-22 baseline vs LLVM-23 target identities under issue #473"
fi

# Pins record the self-contained bindgen prebuilts plus route flags.
if grep -q -F -e 'BINDGEN_CRATE_VERSION = "0.72.1"' "$pins" &&
  grep -q -F -e 'BINDGEN_PREBUILT_TAG = "v0.0.2"' "$pins" &&
  grep -q -F -e 'ec2b39a56443142a34dc76ec32a17cb099c6c09137c3fbac893310c623cb10ac' "$pins" &&
  grep -q -F -e 'd4da684d021d1ebf40bb3819c1521c96c3ca51d22d911ee71731a358bd768594' "$pins" &&
  grep -q -F -e 'beede8f802ab96a2cce0de84c560150dcbc89c96db94f7c17519dd164ac983fd' "$pins" &&
  grep -q -F -e '0885047b40b119e58fcca4491a2bd00331de131f41183de2a9b563ca407f9109' "$pins" &&
  grep -q -F -e '"--no-include-path-detection"' "$pins" &&
  grep -q -F -e '"--formatter=none"' "$pins"; then
  ok
else
  bad "pins.bzl lost its bindgen 0.72.1 prebuilts plus route flags under issue #473"
fi

# Header carries the C11-only stable constructs both LLVM parsers must accept.
if grep -q -F -e '#ifndef DX_BINDGEN_H' "$header" &&
  grep -q -F -e '#include <stdint.h>' "$header" &&
  grep -q -F -e 'DX_BINDGEN_MAX_LABELS' "$header" &&
  grep -q -F -e 'typedef enum DxStatus' "$header" &&
  grep -q -F -e 'typedef struct DxPoint' "$header" &&
  grep -q -F -e 'typedef struct DxTagged' "$header" &&
  grep -q -F -e 'DxMeasureFn' "$header" &&
  grep -q -F -e 'typedef struct DxStore DxStore;' "$header" &&
  grep -q -F -e 'dx_store_create' "$header"; then
  ok
else
  bad "bindgen.h lost its C11-only stable constructs under issue #473"
fi

# Header excludes C23-only spellings needing separate qualification
# (comment lines excluded: the doc comment above names them explicitly).
if ! grep -v -e '^[[:space:]]*//' "$header" | grep -q -F -e 'typeof' &&
  ! grep -v -e '^[[:space:]]*//' "$header" | grep -q -F -e 'constexpr' &&
  ! grep -v -e '^[[:space:]]*//' "$header" | grep -q -F -e '_BitInt' &&
  ! grep -v -e '^[[:space:]]*//' "$header" | grep -q -F -e '#embed'; then
  ok
else
  bad "bindgen.h carries C23-only spellings outside the LLVM-22-vs-23 proof"
fi

# Every expected symbol has C source in the header (identical-set pairing).
expected_missing=""
while IFS= read -r sym; do
  case "$sym" in
    ''|'#'*) continue ;;
  esac
  if ! grep -q -F -e "$sym" "$header"; then
    expected_missing="$expected_missing $sym"
  fi
done <"$expected"
if [[ -z "$expected_missing" ]]; then
  ok
else
  bad "bindgen.expected symbols without C source in bindgen.h:$expected_missing"
fi

# The expected set stays exactly the 15 pinned symbols (no silent widening).
expected_count="$(grep -v -e '^[[:space:]]*#' -e '^[[:space:]]*$' "$expected" | wc -l | tr -d ' ')"
if [[ "$expected_count" == "15" ]]; then
  ok
else
  bad "bindgen.expected carries $expected_count symbols (want 15, identical-set proof)"
fi

# Unpinned LLVM stays rejected: no floating version override or dep lands.
# (This harness plus the pins.bzl doc comment name the rejected form, so
# they are excluded from the scan; only build inputs count)
# (hermetic tree search: BSD grep lacks --include/--exclude, issue #1006).
if ! grep -q -F -e 'llvm.version(' MODULE.bazel .bazelrc 2>/dev/null &&
  ! grep -q -F -e 'bazel_dep(name = "llvm"' MODULE.bazel &&
  dx_tree_absent --include='*.bzl' --include='BUILD.bazel' --exclude='bindgen_qualification.sh' --exclude='pins.bzl' 'llvm.version(' -- rust cc tools third_party; then
  ok
else
  bad "unpinned LLVM detected (llvm.version override or MODULE llvm dep; want pins.bzl pins only)"
fi

# Native plan owns the qualified bindgen record plus the question row.
if grep -q -F -e 'qualified under issue #473' "$native" &&
  grep -q -F -e 'rust/tests/fixtures/bindgen/pins.bzl' "$native" &&
  grep -q -F -e 'bindgen_qualification' "$native" &&
  grep -q -F -e 'unpinned LLVM rejected' "$native" &&
  grep -q -F -e 'Can bindgen/CXX use one upstream graph?' "$native" &&
  grep -q -F -e 'issues #473, #474' "$native"; then
  ok
else
  bad "docs/native-toolchains.md lost its qualified bindgen record with fixtures plus pins under issue #473"
fi

# Contract doc owns the binding-generation routes plus fixture proof.
if grep -q -F -e '## Binding Generation' "$contract" &&
  grep -q -F -e 'qualified under issue #473' "$contract" &&
  grep -q -F -e 'bindgen.h' "$contract" &&
  grep -q -F -e 'bindgen.expected' "$contract" &&
  grep -q -F -e 'unpinned LLVM' "$contract" &&
  grep -q -F -e 'bindgen_qualification' "$contract"; then
  ok
else
  bad "docs/generation/rust.md lost its binding-generation contract under issue #473"
fi

# Generation README pins the qualified compat alongside the other gaps.
if grep -q -F -e 'qualified under' "$gen_readme" &&
  grep -q -F -e 'issue #473' "$gen_readme" &&
  grep -q -F -e 'bindgen.expected' "$gen_readme"; then
  ok
else
  bad "docs/generation/foundation-qualification.md lost its qualified bindgen record under issue #473"
fi

# BUILD owns the harness target.
if grep -q -F -e 'name = "bindgen_qualification"' "$build"; then
  ok
else
  bad "tools/ci/BUILD.bazel lost the bindgen_qualification target"
fi

# CI wires the harness in dogfood-freshness.
if grep -q -F -e 'bazel run --noshow_progress //tools/ci:bindgen_qualification' "$ci"; then
  ok
else
  bad "ci.yml lost the bindgen_qualification step (want dogfood-freshness)"
fi

# Live proof: the fixture header is well-formed C both LLVM parsers must accept.
if gcc -fsyntax-only -std=c11 -Wall -Werror "$header"; then
  ok
else
  bad "bindgen.h fails gcc -fsyntax-only -std=c11 -Wall -Werror (want well-formed C11)"
fi

dx_test_summary "bindgen qualification harness"
