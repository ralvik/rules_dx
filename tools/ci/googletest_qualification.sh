#!/usr/bin/env bash
# GoogleTest v1.18.0 plus C++17 floor qualification harness.
#
# Qualifies the owned gap from closed: provisional GoogleTest v1.18.0,
# closed owner only. covers adapters not the runner version.
# - pinned: GoogleTest 1.18.0 (Bazel Central Registry module `googletest`
#   1.18.0, verified against Bazel 9.2.0 on the seed host) in MODULE.bazel
#   plus `cc/tests/fixtures/googletest/pins.bzl`; the 1.18.x line requires
#   C++17 or newer per the upstream v1.18.0 release notes. Living at head
#   (floating, unpinned) stays rejected per the dx pin policy.
# - floor: explicit `-std=c++17` on the fixture library plus test (never the
#   compiler default); the test source adds `static_assert(__cplusplus)`
#   plus `std::optional` plus structured-bindings plus `if constexpr` floor
#   proofs that fail to compile below C++17.
# - mapping: plain `cc_test` over `@googletest//:gtest_main` with `TEST()`
#   plus `EXPECT_*` sources; the library under test stays its ordinary
#   owner via `deps`. Plain assert seeds stay in `hello/` fixtures.
# - open owned gaps: platform plus consumer plus release evidence, MSVC
#   interop plus SDK licensing, no `Supported` claim. Compatibility is test
#   mapping only.
#
# Versioned here, run by CI via `bazel run //tools/ci:googletest_qualification`,
# following //tools/ci:exact_target_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

dx_bash_pin

pins="cc/tests/fixtures/googletest/pins.bzl"
build="cc/tests/fixtures/googletest/BUILD.bazel"
header="cc/tests/fixtures/googletest/greeter.h"
lib="cc/tests/fixtures/googletest/greeter.cc"
test="cc/tests/fixtures/googletest/greeter_test.cc"
module="MODULE.bazel"
matrix="docs/product/support-matrix.md"
gen_readme="docs/generation/foundation-qualification.md"
ci="tools/ci/dogfood_freshness.sh"
tools_build="tools/ci/ci_targets_d.bzl"
defs="cc/rules/defs.bzl"

# Fixture quad plus pins stay present.
if [[ -f "$pins" && -f "$build" && -f "$header" && -f "$lib" && -f "$test" ]]; then
  ok
else
  bad "googletest fixture missing (want $pins plus $build plus $header plus $lib plus $test)"
fi

# Pins record the version plus floor plus rejected runner.
if grep -q -F -e 'GTEST_VERSION = "1.18.0"' "$pins" &&
  grep -q -F -e 'GTEST_CXX_FLOOR = "17"' "$pins" &&
  grep -q -F -e '"-std=c++17"' "$pins" &&
  grep -q -F -e 'unpinned runner rejected' "$pins"; then
  ok
else
  bad "pins.bzl lost its GoogleTest 1.18.0 plus C++17 floor plus rejected head under issue #479"
fi

# Pins record the gtest_main labels plus fixture target.
if grep -q -F -e '@googletest//:gtest_main' "$pins" &&
  grep -q -F -e '@googletest//:gtest"' "$pins" &&
  grep -q -F -e 'cc/tests/fixtures/googletest:greeter_test' "$pins"; then
  ok
else
  bad "pins.bzl lost its gtest_main labels plus fixture target under issue #479"
fi

# MODULE pins the 1.18.0 line (qualification record in foundation-qualification).
if grep -q -F -e 'bazel_dep(name = "googletest", version = "1.18.0")' "$module" &&
  grep -q -F -e 'googletest_qualification' "$module"; then
  ok
else
  bad "MODULE.bazel lost its googletest 1.18.0 pin plus qualification wiring"
fi

# Fixture maps cc_test over the pinned gtest_main with the C++17 floor.
if grep -q -F -e 'cc_test' "$build" &&
  grep -q -F -e '@googletest//:gtest_main' "$build" &&
  grep -q -F -e '"-std=c++17"' "$build" &&
  grep -q -F -e ':greeter_lib' "$build" &&
  grep -q -F -e 'cc_library' "$build"; then
  ok
else
  bad "cc/tests/fixtures/googletest lost its cc_test plus gtest_main plus -std=c++17 mapping under issue #479"
fi

# Header plus library prove the C++17 floor with std::optional.
if grep -q -F -e 'std::optional' "$header" &&
  grep -q -F -e 'MaybeGreet' "$header" &&
  grep -q -F -e 'std::nullopt' "$lib" &&
  grep -q -F -e 'MaybeGreet' "$lib"; then
  ok
else
  bad "greeter header plus library lost its std::optional C++17 floor proof under issue #479"
fi

# Test sources use TEST plus EXPECT with floor proofs.
if grep -q -F -e '#include <gtest/gtest.h>' "$test" &&
  grep -q -F -e 'TEST(GreeterTest' "$test" &&
  grep -q -F -e 'EXPECT_EQ' "$test" &&
  grep -q -F -e 'static_assert(__cplusplus >= 201703L' "$test" &&
  grep -q -F -e 'make_tuple' "$test" &&
  grep -q -F -e 'if constexpr' "$test"; then
  ok
else
  bad "greeter_test.cc lost its TEST/EXPECT plus C++17 floor proofs under issue #479"
fi

# Plain assert seed stays green via the wrapper (hello fixtures).
if grep -q -F -e 'cc_test' cc/tests/fixtures/hello/BUILD.bazel &&
  grep -q -F -e 'assert' cc/tests/fixtures/hello/hello_test.cc; then
  ok
else
  bad "plain cc seed lost its assert hello mapping under issue #479"
fi

# Unpinned runner stays rejected: no head or floating googletest dep lands
# (exact 1.18.0 only; the qualified "living at head rejected" record in
# pins/wrapper/docs is the rejection, not a head dep)
# (hermetic tree search: BSD grep lacks --include, issue #1006).
if DX_TREE_RE=1 dx_tree_absent --include='*.bzl' --include='BUILD.bazel' 'googletest[^"]*(head|master|latest|1\.\+)' -- cc third_party MODULE.bazel; then
  ok
else
  bad "unpinned GoogleTest runner detected (head or floating; want exact 1.18.0 only)"
fi

# Wrapper owns the qualified mapping, not an open selection.
if grep -q -F -e 'GoogleTest' "$defs" &&
  grep -q -F -e 'googletest_qualification' "$defs"; then
  ok
else
  bad "cc wrapper lost its qualified GoogleTest mapping record"
fi

# Generation README pins the qualified runner alongside the other gaps.
if grep -q -F -e 'qualified GoogleTest v1.18.0' "$gen_readme" &&
  grep -q -F -e 'cc/tests/fixtures/googletest/' "$gen_readme" &&
  grep -q -F -e 'googletest_qualification' "$gen_readme" &&
  grep -q -F -e 'issue #479' "$gen_readme"; then
  ok
else
  bad "docs/generation/foundation-qualification.md lost its qualified GoogleTest record under issue #479"
fi

# ci_targets_d owns the harness target.
if grep -q -F -e 'name = "googletest_qualification"' "$tools_build"; then
  ok
else
  bad "tools/ci/ci_targets_d.bzl lost the googletest_qualification target"
fi

# dogfood-freshness wires the harness run.
if grep -q -F -e 'bazel run --noshow_progress //tools/ci:googletest_qualification' "$ci"; then
  ok
else
  bad "dogfood_freshness.sh lost the googletest_qualification step"
fi

# Live proof: GoogleTest plus plain seed mappings execute green on the seed host.
if bazel test //cc/tests/fixtures/googletest:greeter_test //cc/tests/fixtures/hello:hello_test --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "googletest live proof failed (want greeter_test plus hello seed green)"
fi

dx_test_summary "googletest qualification harness"
