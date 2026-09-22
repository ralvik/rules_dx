#!/usr/bin/env bash
# Review-thread limit plus accounting qualification harness.
#
# Freezes the per-PR review-thread limit plus deterministic accounting
# left unfrozen under:
# - frozen: exactly 50 open integration-owned threads per PR across
#   checks and platforms, no consumer setting, no fresh allowance per
#   job/rerun/retry/parallel completion;
# - priority: failure-contributing first with check/file/line/rule
#   ordering, never job completion order;
# - preservation: existing threads kept without rotation of
#   still-present findings;
# - slots: bot-only deletion frees a slot, resolved-with-replies history
#   stays retained outside the open count, skipped/disabled/cancelled/
#   incomplete/missing/outside-diff never prove gone;
# - reports: full reports retain every finding, summary distinguishes
#   limit-omitted from unmappable, intentional truncation never changes
#   outcomes while publication failures still fail reporting;
# - concurrency: late callbacks never overwrite current threads/summaries;
# - fixtures: `tools/ci/tests/fixtures/review_threads/` (`pins.bzl`
#   plus `review_threads.expected`) pins the frozen limit plus accounting
#   plus rejected routes plus honesty;
# - scope: CI-only. Seed only: no live thread runs; no Supported claim.
#
# Versioned here, run by CI via `bazel run //tools/ci:review_threads_qualification`,
# following //tools/ci:consumer_ci_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

dx_bash_pin

contract="docs/github-ci.md"
matrix="docs/testing/github-ci.md"
pins="tools/ci/tests/fixtures/review_threads/pins.bzl"
expected="tools/ci/tests/fixtures/review_threads/review_threads.expected"
fixture_build="tools/ci/tests/fixtures/review_threads/BUILD.bazel"
build="tools/ci/BUILD.bazel"
ci=".github/workflows/ci.yml"

# Contract freezes the numeric limit at 50 with fixture plus qualification proof.
if grep -q -F -e 'frozen at 50 open integration-owned threads' "$contract" &&
  grep -q -F -e 'qualified under issue #592' "$contract" &&
  grep -q -F -e 'tools/ci/tests/fixtures/review_threads/pins.bzl' "$contract" &&
  grep -q -F -e 'bazel run //tools/ci:review_threads_qualification' "$contract"; then
  ok
else
  bad "github-ci.md lost its frozen 50-thread limit with fixture plus qualification proof under #592"
fi

# Contract qualification bullet records the frozen limit with remainder open.
if grep -q -F -e 'numeric limit plus thread-accounting frozen at 50 open threads under issue #592' "$contract" &&
  grep -q -F -e 'remainder' "$contract"; then
  ok
else
  bad "github-ci.md qualification lost its frozen-limit plus remainder-open record under #592"
fi

# Contract footer owns the freeze tracker with fixture proof.
if grep -q -F -e 'Review-thread limit plus accounting frozen at 50 open' "$contract" &&
  grep -q -F -e 'tools/ci/tests/fixtures/review_threads/pins.bzl' "$contract" &&
  grep -q -F -e 'issue #592' "$contract"; then
  ok
else
  bad "github-ci.md lost its #592 freeze tracker with fixture proof"
fi

# Matrix Review-Thread Limit exercises below/at/above the frozen 50 with priority proof.
if grep -q -F -e 'frozen per-PR review-thread limit of 50' "$matrix" &&
  grep -q -F -e 'tools/ci/tests/fixtures/review_threads/pins.bzl' "$matrix" &&
  grep -q -F -e 'bazel run //tools/ci:review_threads_qualification' "$matrix" &&
  grep -q -F -e 'retained resolved discussions outside the open count' "$matrix" &&
  grep -q -F -e 'late callbacks never overwriting current threads' "$matrix"; then
  ok
else
  bad "testing/github-ci.md lost its frozen-50 review-thread limit case with accounting proof under #592"
fi

# Matrix qualification footer owns the frozen record with honesty.
if grep -q -F -e 'Review-thread limit plus accounting frozen at 50 open threads seed-only under issue #592' "$matrix" &&
  grep -q -F -e 'tools/ci/tests/fixtures/review_threads/pins.bzl' "$matrix" &&
  grep -q -F -e 'bazel run //tools/ci:review_threads_qualification' "$matrix" &&
  grep -q -F -e 'no Supported claim' "$matrix"; then
  ok
else
  bad "testing/github-ci.md lost its #592 frozen-limit qualification record with honesty"
fi

# Fixture pins stay present with the frozen 50 plus scope plus honesty.
if [[ -f "$pins" && -f "$expected" && -f "$fixture_build" ]] &&
  grep -q -F -e 'REVIEW_THREAD_LIMIT = 50' "$pins" &&
  grep -q -F -e 'frozen at 50 open integration-owned threads under issue #592' "$pins" &&
  grep -q -F -e 'one fixed per-PR limit across checks and platforms' "$pins" &&
  grep -q -F -e 'without a consumer setting' "$pins" &&
  grep -q -F -e 'no fresh allowance per job/rerun' "$pins" &&
  grep -q -F -e 'qualified seed-only under issue #592' "$pins" &&
  grep -q -F -e 'no Supported claim' "$pins"; then
  ok
else
  bad "review_threads pins fixture lost its frozen-50 plus scope plus honesty wiring under #592"
fi

# Pins record deterministic failure-first priority, never completion order.
if grep -q -F -e 'failure-contributing findings first' "$pins" &&
  grep -q -F -e 'check/file/line/rule ordering' "$pins" &&
  grep -q -F -e 'not job completion order' "$pins"; then
  ok
else
  bad "review_threads pins.bzl lost its failure-first deterministic priority pins under #592"
fi

# Pins record preservation without rotation.
if grep -q -F -e 'do not delete or resolve still-present findings to rotate' "$pins" &&
  grep -q -F -e 'Keep a thread while its finding remains' "$pins"; then
  ok
else
  bad "review_threads pins.bzl lost its no-rotation preservation pins under #592"
fi

# Pins record slot accounting: bot-only frees, resolved retained outside, never-proves-gone.
if grep -q -F -e 'delete bot-only threads without replies frees a slot' "$pins" &&
  grep -q -F -e 'resolve threads with human replies instead, retained outside the open count' "$pins" &&
  grep -q -F -e 'skipped, disabled, cancelled, or incomplete checks never prove a finding is gone' "$pins" &&
  grep -q -F -e 'locations moving outside the diff do not prove a finding is gone' "$pins"; then
  ok
else
  bad "review_threads pins.bzl lost its slot-accounting pins under #592"
fi

# Pins record full reports plus summary distinction plus truncation honesty.
if grep -q -F -e 'Full reports retain every finding' "$pins" &&
  grep -q -F -e 'summary distinguishes findings omitted due to the limit' "$pins" &&
  grep -q -F -e 'Intentional truncation is not incomplete analysis or publication failure' "$pins" &&
  grep -q -F -e 'never changes CI outcomes' "$pins"; then
  ok
else
  bad "review_threads pins.bzl lost its full-reports plus summary plus truncation pins under #592"
fi

# Pins record concurrency plus rejected substitutes.
if grep -q -F -e 'late callbacks never overwrite current threads/summaries' "$pins" &&
  grep -q -F -e 'leaving unfrozen rejected' "$pins" &&
  grep -q -F -e 'consumer limit setting rejected' "$pins" &&
  grep -q -F -e 'fresh allowance per job/rerun rejected' "$pins" &&
  grep -q -F -e 'job completion order rejected' "$pins" &&
  grep -q -F -e 'rotation of still-present findings rejected' "$pins"; then
  ok
else
  bad "review_threads pins.bzl lost its concurrency plus rejected pins under #592"
fi

# Expected fixture pins the frozen 50 plus accounting plus rejected plus honesty lines.
if grep -q -F -e 'frozen at 50 open threads (issue #592)' "$expected" &&
  grep -q -F -e 'Failure-contributing findings win with check/file/line/rule' "$expected" &&
  grep -q -F -e 'not job completion order' "$expected" &&
  grep -q -F -e 'without rotation' "$expected" &&
  grep -q -F -e 'Bot-only deletion frees a slot' "$expected" &&
  grep -q -F -e 'resolved-with-replies history stays retained outside the open count' "$expected" &&
  grep -q -F -e 'never prove a finding is gone' "$expected" &&
  grep -q -F -e 'reports retain every finding' "$expected" &&
  grep -q -F -e 'summary distinguishes findings omitted' "$expected" &&
  grep -q -F -e 'truncation is not incomplete' "$expected" &&
  grep -q -F -e 'changes CI outcomes' "$expected" &&
  grep -q -F -e 'Late callbacks never overwrite' "$expected" &&
  grep -q -F -e 'Leaving unfrozen rejected' "$expected" &&
  grep -q -F -e 'Qualified seed-only under issue #592' "$expected" &&
  grep -q -F -e 'no Supported claim' "$expected"; then
  ok
else
  bad "review_threads.expected lost its frozen-50 plus accounting plus rejected plus honesty lines under #592"
fi

# Fixture BUILD exports pins plus expected with corpus coverage.
if grep -q -F -e 'pins.bzl' "$fixture_build" &&
  grep -q -F -e 'review_threads.expected' "$fixture_build" &&
  grep -q -F -e 'corpus_starlark' "$fixture_build"; then
  ok
else
  bad "review_threads BUILD.bazel lost its pins plus expected exports with corpus under #592"
fi

# BUILD owns the harness target plus CI wires it in dogfood-freshness.
if grep -q -F -e 'name = "review_threads_qualification"' "$build" &&
  grep -q -F -e 'review_threads_qualification.sh' "$build" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:review_threads_qualification' "$ci"; then
  ok
else
  bad "tools/ci/BUILD.bazel or ci.yml lost the review_threads_qualification wiring (want target plus dogfood-freshness)"
fi

# Live proof: the fixture package builds green on the seed host.
if bazel build //tools/ci/tests/fixtures/review_threads/... --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "review-threads fixture failed to build (want green on the seed host, issue #592)"
fi

dx_test_summary "review threads qualification harness"
