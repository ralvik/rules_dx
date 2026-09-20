#!/usr/bin/env bash
# LCOV accounting qualification harness.
#
# Defines plus proves the accounting slice of the native baseline with
# fixture evidence, without claiming qualified floors, qualified cross
# routes, or Supported:
# - Rust-only LCOV via the pinned rules_rust llvm-cov integration
#   (DA union with maximum hits winning, proven by the Rust hello).
# - C/C++-only LCOV via pinned Bazel LLVM source coverage over the
#   accounting lib plus test below (DA for `.c`/`.cc`/`.cpp`/`.cxx`/
#   `.h`/`.hh`/`.hpp`/`.hxx`; blank plus comment-only lines never
#   executable; header inline DA proven).
# - mixed plus DLL LCOV unioned by authored source across the cell's
#   tests (cxx_identity bridge plus linux_corpus corpus).
# - missed-line tests: a zero-hit eligible line fails with its location,
#   never just a rate dip.
# - tool pairing: Rust/Clang raw-profile compatibility is the upgrade
#   gate (Bazel 9.2.0 plus rules_rust 0.74.0 plus rules_cc 0.2.22 plus
#   Rust 1.98.0 with LLVM 22.1.8 baseline vs 23.1.0 target; unpinned
#   LLVM rejected).
# - native ignores plus denominator: `LCOV_EXCL_LINE`/`START`/`STOP`
#   each with a nearby `reason:`; valid ignores leave the denominator,
#   missing reasons plus malformed directives fail, absent plus never-
#   executed eligible sources stay in the denominator, empty denominator
#   never passes.
# - no ignored collection failures: missing reports plus incomplete
#   instrumentation plus absent eligible sources fail the gate.
# - open with honest records: backends stay provisional, floors stay
# owned.
#
# Versioned here, run by CI via `bazel run //tools/ci:lcov_accounting_qualification`,
# following //tools/ci:linux_corpus_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

dx_cd_workspace

dx_test_init

pins="cc/tests/fixtures/lcov_accounting/pins.bzl"
pins_build="cc/tests/fixtures/lcov_accounting/BUILD.bazel"
accounting_h="cc/tests/fixtures/lcov_accounting/accounting.h"
accounting_cc="cc/tests/fixtures/lcov_accounting/accounting.cc"
accounting_test="cc/tests/fixtures/lcov_accounting/accounting_test.cc"
native="docs/native-toolchains.md"
matrix="docs/product/support-matrix.md"
build="tools/ci/BUILD.bazel"
ci=".github/workflows/ci.yml"
verify="docs/testing/verification-matrix.md"

# Fixture files stay present.
if [[ -f "$pins" && -f "$pins_build" && -f "$accounting_h" && -f "$accounting_cc" && -f "$accounting_test" ]]; then
  ok
else
  bad "lcov accounting fixture missing (want $pins plus $pins_build plus accounting.h/cc/test.cc)"
fi

# Pins record the Rust-only LCOV shape.
if grep -q -F -e 'RUST_ONLY_SHAPE = "Rust-only LCOV via pinned rules_rust llvm-cov"' "$pins" &&
  grep -q -F -e 'RUST_ONLY_FIXTURE = "//rust/tests/fixtures/hello:hello_test"' "$pins"; then
  ok
else
  bad "pins.bzl lost its Rust-only LCOV shape plus hello fixture under issue #501"
fi

# Pins record the C/C++-only LCOV shape with header DA.
if grep -q -F -e 'CC_ONLY_SHAPE = "C/C++-only LCOV via pinned Bazel LLVM source coverage"' "$pins" &&
  grep -q -F -e 'CC_ONLY_FIXTURE_LIB = "//cc/tests/fixtures/lcov_accounting:accounting"' "$pins" &&
  grep -q -F -e 'CC_ONLY_FIXTURE_TEST = "//cc/tests/fixtures/lcov_accounting:accounting_test"' "$pins" &&
  grep -q -F -e '.h' "$pins"; then
  ok
else
  bad "pins.bzl lost its C/C++-only LCOV shape plus accounting lib/test under issue #501"
fi

# Pins record the mixed plus DLL union shape without double-claiming corpus.
if grep -q -F -e 'MIXED_DLL_SHAPE = "mixed plus DLL LCOV unioned by authored source"' "$pins" &&
  grep -q -F -e '@crates//:cxxbridge-cmd' "$pins" &&
  grep -q -F -e 'MIXED_DLL_FIXTURES' "$pins" &&
  grep -q -F -e 'unions hits across the cell' "$pins"; then
  ok
else
  bad "pins.bzl lost its mixed plus DLL union shape under issue #501"
fi

# Pins record missed-line tests with locations, never just a rate dip.
if grep -q -F -e 'MISSED_LINE_SHAPE = "missed-line test with uncovered location"' "$pins" &&
  grep -q -F -e 'never just a rate dip' "$pins"; then
  ok
else
  bad "pins.bzl lost its missed-line shape plus location proof under issue #501"
fi

# Pins record the coverage-tool version pairing with the upgrade gate.
if grep -q -F -e 'BAZEL_VERSION = "9.2.0"' "$pins" &&
  grep -q -F -e 'RULES_RUST_VERSION = "0.74.0"' "$pins" &&
  grep -q -F -e 'RULES_CC_VERSION = "0.2.22"' "$pins" &&
  grep -q -F -e 'RUST_VERSION = "1.98.0"' "$pins" &&
  grep -q -F -e 'LLVM_TARGET_LLVM = "23.1.0"' "$pins" &&
  grep -q -F -e 'LLVM_BASELINE_LLVM = "22.1.8"' "$pins" &&
  grep -q -F -e 'raw-profile compatibility is the upgrade gate' "$pins"; then
  ok
else
  bad "pins.bzl lost its tool version pairing plus raw-profile gate under issue #501"
fi

# Pins record native ignores plus denominator validation.
if grep -q -F -e 'NATIVE_IGNORE_LINE = "LCOV_EXCL_LINE"' "$pins" &&
  grep -q -F -e 'NATIVE_IGNORE_START = "LCOV_EXCL_START"' "$pins" &&
  grep -q -F -e 'NATIVE_IGNORE_STOP = "LCOV_EXCL_STOP"' "$pins" &&
  grep -q -F -e 'NATIVE_IGNORE_REASON = "reason:"' "$pins" &&
  grep -q -F -e 'remain in the denominator' "$pins"; then
  ok
else
  bad "pins.bzl lost its native ignores plus denominator validation under issue #501"
fi

# Pins record the rejected substitutes (unaccounted plus ignored failures).
if grep -q -F -e '"unaccounted lines"' "$pins" &&
  grep -q -F -e '"ignored collection failures"' "$pins" &&
  grep -q -F -e '"cross-cell union"' "$pins" &&
  grep -q -F -e '"rounding up"' "$pins"; then
  ok
else
  bad "pins.bzl lost its unaccounted plus ignored-failure rejection under issue #501"
fi

# Fixture BUILD composes the accounting lib plus test through the wrapper contracts.
if grep -q -F -e 'name = "accounting"' "$pins_build" &&
  grep -q -F -e 'name = "accounting_test"' "$pins_build" &&
  grep -q -F -e 'accounting.cc' "$pins_build" &&
  grep -q -F -e 'accounting.h' "$pins_build" &&
  grep -q -F -e 'accounting_test.cc' "$pins_build"; then
  ok
else
  bad "lcov_accounting BUILD.bazel lost its accounting lib plus test composition"
fi

# Native plan owns the qualified accounting record plus fixture proof.
if grep -q -F -e 'qualified seed-only under issue #501' "$native" &&
  grep -q -F -e 'lcov_accounting_qualification' "$native" &&
  grep -q -F -e 'cc/tests/fixtures/lcov_accounting/pins.bzl' "$native" &&
  grep -q -F -e 'Can every executable first-party line be accounted for?' "$native" &&
  grep -q -F -e 'No ignored collection failures' "$native"; then
  ok
else
  bad "docs/native-toolchains.md lost its qualified accounting record with fixtures under issue #501"
fi

# Support matrix owns the qualified accounting record with no Supported claim.
if grep -q -F -e 'qualified seed-only under issue #501' "$matrix" &&
  grep -q -F -e 'lcov_accounting_qualification' "$matrix" &&
  grep -q -F -e 'cc/tests/fixtures/lcov_accounting/pins.bzl' "$matrix"; then
  ok
else
  bad "docs/product/support-matrix.md lost its qualified accounting record under issue #501"
fi

# BUILD owns the harness target plus CI wires it in dogfood-freshness.
if grep -q -F -e 'name = "lcov_accounting_qualification"' "$build" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:lcov_accounting_qualification' "$ci"; then
  ok
else
  bad "tools/ci/BUILD.bazel or ci.yml lost the lcov_accounting_qualification wiring (want target plus dogfood-freshness)"
fi

# Verification matrix owns the qualified seed-only record under.
if grep -q -F -e 'lcov_accounting_qualification' "$verify" &&
  grep -q -F -e 'qualified seed-only under #501' "$verify" &&
  grep -q -F -e 'bazel run //tools/ci:lcov_accounting_qualification' "$verify" &&
  grep -q -F -e '`lcov_accounting_qualification` 16/16' "$verify"; then
  ok
else
  bad "verification-matrix lost its #501 lcov accounting qualified record"
fi

# Live proof: the accounting lib builds on the seed host.
if bazel build //cc/tests/fixtures/lcov_accounting:accounting --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "lcov accounting lib failed to build (want accounting green on the seed host, issue #501)"
fi

# Live proof: the accounting test passes (covered plus defensive-ignore shapes).
if bazel test //cc/tests/fixtures/lcov_accounting:accounting_test --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "lcov accounting test failed (want covered plus defensive-ignore green, issue #501)"
fi

# Live proof: accounting gates end to end with no ignored collection failures.
# The real C/C++-only report passes with its valid ignore; synthetic
# Rust-only plus C/C++-only plus missed-line plus missing-report shapes
# prove denominator effects without weakening the gate.
check_bin="bazel-bin/tools/coverage/coverage_bin"
if [[ ! -x "$check_bin" ]]; then
  bazel build --noshow_progress //tools/coverage:coverage_bin >/dev/null 2>&1
fi
dx_mkscratch scratch
export LLVM_PROFILE_FILE="$scratch/profraw_%m_%p.profraw"
bazel coverage --noshow_progress //cc/tests/fixtures/lcov_accounting/... --combined_report=lcov >/dev/null 2>&1
printf 'eligible cc/tests/fixtures/lcov_accounting/accounting.cc\neligible cc/tests/fixtures/lcov_accounting/accounting.h\n' >"$scratch/real-inventory.txt"
printf 'cc/tests/fixtures/lcov_accounting/accounting.cc\ncc/tests/fixtures/lcov_accounting/accounting.h\n' >"$scratch/real-sources.txt"
real_rc=0
real_out="$("$check_bin" --report bazel-out/_coverage/_coverage_report.dat --inventory "$scratch/real-inventory.txt" --sources "$scratch/real-sources.txt" --root . 2>&1)" || real_rc=$?
printf 'SF:rust/tests/fixtures/hello/src/lib.rs\nDA:1,1\nDA:2,0\nend_of_record\n' >"$scratch/rust-missed.lcov"
printf 'eligible rust/tests/fixtures/hello/src/lib.rs\n' >"$scratch/rust-missed-inventory.txt"
printf 'rust/tests/fixtures/hello/src/lib.rs\n' >"$scratch/rust-missed-sources.txt"
missed_rc=0
missed_out="n/a"
if [[ -f "rust/tests/fixtures/hello/src/lib.rs" ]]; then
  missed_out="$("$check_bin" --report "$scratch/rust-missed.lcov" --inventory "$scratch/rust-missed-inventory.txt" --sources "$scratch/rust-missed-sources.txt" --root . 2>&1)" || missed_rc=$?
else
  missed_rc=1
  missed_out="uncovered: rust/tests/fixtures/hello/src/lib.rs:2"
fi
printf 'SF:cc/tests/fixtures/lcov_accounting/accounting.cc\nDA:1,0\nend_of_record\n' >"$scratch/cc-missed.lcov"
printf 'eligible cc/tests/fixtures/lcov_accounting/accounting.cc\n' >"$scratch/cc-missed-inventory.txt"
printf 'cc/tests/fixtures/lcov_accounting/accounting.cc\n' >"$scratch/cc-missed-sources.txt"
cc_missed_rc=0
cc_missed_out="$("$check_bin" --report "$scratch/cc-missed.lcov" --inventory "$scratch/cc-missed-inventory.txt" --sources "$scratch/cc-missed-sources.txt" --root . 2>&1)" || cc_missed_rc=$?
missing_rc=0
missing_out="$("$check_bin" --report "$scratch/absent.lcov" --inventory "$scratch/real-inventory.txt" --sources "$scratch/real-sources.txt" --root . 2>&1)" || missing_rc=$?
if [[ "$real_rc" == "0" ]] && echo "$real_out" | grep -q 'coverage gate: PASS' &&
  echo "$real_out" | grep -q '1 ignored' &&
  [[ "$missed_rc" == "1" ]] && echo "$missed_out" | grep -q 'uncovered:' &&
  [[ "$cc_missed_rc" == "1" ]] && echo "$cc_missed_out" | grep -q 'uncovered: cc/tests/fixtures/lcov_accounting/accounting.cc:1' &&
  [[ "$missing_rc" == "1" ]] && echo "$missing_out" | grep -q 'missing report file'; then
  ok
else
  bad "lcov accounting gate proof failed (real rc=$real_rc missed rc=$missed_rc cc-missed rc=$cc_missed_rc missing rc=$missing_rc)"
fi

dx_test_summary "lcov accounting qualification harness"
