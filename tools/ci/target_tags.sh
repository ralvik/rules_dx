#!/usr/bin/env bash
# Target-tag semantics harness (issue #99 item 3).
#
# Four tags change what CI runs, and none of their semantics was proven
# end to end:
# - `flaky`: wrappers must forward it untouched to the private upstream
#   test (passthrough, off by default per docs/testing/starlark.md).
# - `no-coverage`: the coverage preset must skip tagged tests while
#   still running untagged ones (tools/bazelrc/preset.bazelrc).
# - `no-lint` / `no-typecheck`: the quality pipelines must suppress
#   only the named capability for tagged owners.
# - `manual`: enumerated and guarded by //tools/ci:manual_negatives.
#
# Versioned here, run by CI via `bazel run //tools/ci:target_tags`,
# following //tools/ci:corpus_audit.
set -euo pipefail

# Shared workspace + runfiles helpers (issue #319).
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

dx_cd_workspace

dx_test_init

# flaky is off by default: no test in the tree opts into retries.
flaky_tests="$(bazel query "attr('flaky', 1, kind(test, //...))" 2>/dev/null | grep -v -F -e 'flaky_passthrough_fixture_upstream' || true)"
if [[ -z "$flaky_tests" ]]; then
  ok
else
  bad "unexpected flaky tests outside the passthrough fixture: $flaky_tests"
fi

# flaky passes through: the fixture's upstream carries `flaky = True`
# while the public forwarding wrapper stays an ordinary test.
upstream_build="$(bazel query --output=build '//python/tests/fixtures/hello:flaky_passthrough_fixture_upstream' 2>/dev/null)"
if echo "$upstream_build" | grep -q -E '^  flaky = True,$'; then
  ok
else
  bad "flaky did not reach flaky_passthrough_fixture_upstream"
fi
public_build="$(bazel query --output=build '//python/tests/fixtures/hello:flaky_passthrough_fixture' 2>/dev/null)"
if echo "$public_build" | grep -q -E '^  flaky = '; then
  bad "flaky leaked onto the public forwarding wrapper"
else
  ok
fi

# No wrapper family strips test kwargs: every language forwards the
# full kwargs dict to its upstream rule, so the fixture exemplifies a
# mechanism shared by all families rather than a Python special case.
if grep -rn -F -e '.pop(' --include='defs.bzl' ./*/rules/ | grep -v '\.git/' >/dev/null; then
  bad "a wrapper strips test kwargs; flaky passthrough no longer holds by construction"
else
  ok
fi

# no-coverage is behavioral: under `bazel coverage` the tagged
# process-spawning hello_output_test is skipped while untagged tests
# run and emit coverage data. (The manual upstream is absent too.)
cov_out="$(bazel coverage --noshow_progress --nocache_test_results //python/tests/fixtures/hello/... 2>&1)"
if echo "$cov_out" | grep -q -E '//python/tests/fixtures/hello:hello_test +PASSED'; then
  ok
else
  bad "untagged hello_test did not run under bazel coverage"
fi
if echo "$cov_out" | grep -q -F -e 'hello_output_test'; then
  bad "no-coverage hello_output_test ran under bazel coverage"
else
  ok
fi
if echo "$cov_out" | grep -q -F -e 'hello_test_upstream'; then
  bad "manual hello_test_upstream ran under bazel coverage"
else
  ok
fi

# no-lint / no-typecheck wiring: the opt-out fixtures stay tagged, and
# the presence suites that assert capability suppression still pass.
if [[ -n "$(bazel query 'attr(tags, no-lint, //quality/testdata/...)' 2>/dev/null)" ]]; then
  ok
else
  bad "no-lint opt-out fixtures disappeared from //quality/testdata"
fi
if [[ -n "$(bazel query 'attr(tags, no-typecheck, //quality/testdata/...)' 2>/dev/null)" ]]; then
  ok
else
  bad "no-typecheck opt-out fixtures disappeared from //quality/testdata"
fi
if bazel test --noshow_progress //quality/testdata:aspect_presence //quality/testdata:real_aspect_presence //quality/testdata:real_typecheck_presence >/dev/null 2>&1; then
  ok
else
  bad "capability presence suites failed"
fi

dx_test_summary "target tags harness"
