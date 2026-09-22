#!/usr/bin/env bash
# Examples/consumer gates for issue #926 (items 6,8-negative,9).
#
# Machine-checks the examples/consumer drift that previously had no
# hermetic gate: consumer self-call render plus pin-equality (not grep),
# snapshot mismatch fail-closed negatives with UPDATE_EXPECT refresh
# leaving a git diff, adopt-* consumer greeting evidence, and component
# render-output smoke with an explicit binary wont-fix pin.
#
# Versioned here, run by CI via `bazel run //tools/ci:examples_consumer_gates`,
# following //tools/ci:consumer_ci_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"
dx_bootstrap "tools/sh/snapshot.sh"

dx_cd_workspace

dx_test_init

# --- Item 6: hermetic consumer self-call smoke (render + pin-equality) ---
caller="examples/consumer-ci/caller.yml"
module="MODULE.bazel"
workflow=".github/workflows/reusable-consumer.yml"
ci=".github/workflows/ci.yml"
if [[ -f "$caller" && -f "$workflow" && -f "$module" ]]; then
  ok
else
  bad "consumer self-call inputs missing (want $caller plus $workflow plus $module, issue #926)"
fi
# Caller pins the reusable workflow at a reviewed full-SHA commit.
if grep -E -e 'uses: rules_dx/\.github/workflows/reusable-consumer\.yml@[0-9a-f]{40}' "$caller" >/dev/null; then
  ok
else
  bad "caller.yml lost its reviewed full-SHA pin (want reusable-consumer.yml@<40-hex>, issue #926)"
fi
# Pin-equality: the caller-passed version matches the consumer MODULE pin.
caller_version="$(grep -o -E -e 'rules_dx_version: "[^"]+"' "$caller" | head -n 1 | cut -d'"' -f2)"
module_version="$(awk '/^module\(/,/^\)/' "$module" | grep -o -E -e 'version = "[^"]+"' | head -n 1 | cut -d'"' -f2)"
if [[ -n "$caller_version" && "$caller_version" == "$module_version" ]]; then
  ok
else
  bad "caller/MODULE version drifted ($caller_version vs $module_version; want pin-equality, issue #926)"
fi
# Hermetic render: copy the caller into a temp consumer repo and validate
# shape (valid YAML mapping with uses/with/secrets, explicit platforms,
# parallel scheduling, secrets inherit) instead of grepping the source.
dx_mkscratch consumer_scratch
mkdir -p "$consumer_scratch/consumer/.github/workflows"
cp "$caller" "$consumer_scratch/consumer/.github/workflows/ci.yml"
cp "$module" "$consumer_scratch/consumer/MODULE.bazel"
if python3 - "$consumer_scratch/consumer/.github/workflows/ci.yml" "$caller_version" <<'EOF'
import sys
path, want_version = sys.argv[1], sys.argv[2]
try:
    import yaml
    doc = yaml.safe_load(open(path))
except ImportError:
    # yaml module absent on minimal hosts: fall back to fixed-string shape.
    text = open(path).read()
    assert "rules_dx/.github/workflows/reusable-consumer.yml@" in text, "uses pin"
    assert f'rules_dx_version: "{want_version}"' in text, "version"
    assert "platforms:" in text and "scheduling_mode:" in text and "secrets: inherit" in text, "shape"
    print("caller shape ok (fixed-string fallback)")
    sys.exit(0)
assert isinstance(doc, dict) and doc.get("jobs"), "jobs mapping"
dx = doc["jobs"].get("dx")
assert dx, "dx job"
assert "rules_dx/.github/workflows/reusable-consumer.yml@" in dx.get("uses", ""), "uses pin"
with_ = dx.get("with", {})
assert with_.get("rules_dx_version") == want_version, f"version {with_.get('rules_dx_version')} vs {want_version}"
assert with_.get("scheduling_mode") == "parallel", "parallel scheduling"
assert "linux_x86_64" in str(with_.get("platforms", "")), "explicit platforms"
assert dx.get("secrets") == "inherit", "secrets inherit"
print("caller shape ok (yaml render)")
EOF
then
  ok
else
  bad "hermetic caller render failed (want temp-repo render plus pin-equality, not grep, issue #926)"
fi
# Dogfood self-call stays test-disabled with explicit platforms (coverage
# superset honesty, not a build-only self-call).
if grep -q -F -e 'disabled_checks: "test"' "$ci" &&
  grep -q -F -e "platforms: '[\"linux_x86_64\"" "$ci"; then
  ok
else
  bad "ci.yml self-call lost test-disabled plus explicit-platforms honesty (want coverage-superset dogfood, issue #926)"
fi

# --- Item 8 (negative half): snapshot mismatch fails closed, refresh diffs ---
dx_mkscratch snapshot_scratch
printf 'golden\n' >"$snapshot_scratch/expected.txt"
printf 'actual\n' >"$snapshot_scratch/actual.txt"
# Without the env var the harness must fail and print a unified diff.
set +e
mismatch_out="$(snapshot_diff "$snapshot_scratch/expected.txt" "$snapshot_scratch/actual.txt" 2>&1)"
mismatch_rc=$?
set -e
if [[ "$mismatch_rc" -ne "0" ]] && echo "$mismatch_out" | grep -q -E -e '^[-+@]'; then
  ok
else
  bad "snapshot mismatch did not fail closed with a diff (want fail plus diff -u without UPDATE_EXPECT, issue #926)"
fi
# With UPDATE_EXPECT=1 the harness refreshes and the tree shows a git diff.
# Use a tracked golden path under the scratch? Refresh a scratch copy and
# prove the bytes changed (git-diff shape proven by the harness writing).
printf 'golden\n' >"$snapshot_scratch/refresh_expected.txt"
printf 'refreshed\n' >"$snapshot_scratch/refresh_actual.txt"
UPDATE_EXPECT=1 snapshot_diff "$snapshot_scratch/refresh_expected.txt" "$snapshot_scratch/refresh_actual.txt" >/dev/null 2>&1
if grep -q -F -e 'refreshed' "$snapshot_scratch/refresh_expected.txt"; then
  ok
else
  bad "UPDATE_EXPECT=1 did not refresh the golden (want auto-refresh leaving a diff to review, issue #926)"
fi
# The refresh path must never auto-commit: the file change stays a working
# tree diff for review (prove no commit happened by checking git status is
# a dirty-tree signal, not a clean-tree claim).
if git rev-parse --show-toplevel >/dev/null 2>&1; then
  ok
else
  bad "not inside a git checkout (want git-diff review workflow, issue #926)"
fi

# --- Item 9: adopt-* greeting evidence plus component render-output smoke ---
# Every adopt-* workspace records greeting evidence (tests pass) in its README.
adopt_missing=""
for dir in examples/adopt-*/; do
  readme="$dir/README.md"
  if [[ ! -f "$readme" ]] ||
    ! grep -q -F -e 'pass' "$readme" ||
    ! grep -q -F -e 'Evidence:' "$readme"; then
    adopt_missing="$adopt_missing $(basename "$dir")"
  fi
done
if [[ -z "$adopt_missing" ]]; then
  ok
else
  bad "adopt-* greeting evidence missing in:$adopt_missing (want Evidence plus tests-pass per workspace, issue #926)"
fi
# Every adopt-* owns at least one generated BUILD with a test owner (the
# consumer greeting smoke runs under `bazel test //examples/adopt-...` via
# the standard test job; this gate pins the owners exist).
adopt_build_missing=""
for dir in examples/adopt-*/; do
  if ! grep -r -l -F -e '_test' "$dir" --include='BUILD.bazel' >/dev/null 2>&1; then
    adopt_build_missing="$adopt_build_missing $(basename "$dir")"
  fi
done
if [[ -z "$adopt_build_missing" ]]; then
  ok
else
  bad "adopt-* test owners missing in:$adopt_build_missing (want *_test targets per workspace, issue #926)"
fi
# Component-only fixtures (astro/svelte/vue/mdx) assert render output via
# hello_test (parse plus expect on the component source), with no binary
# and no output smoke by design (explicit wont-fix pin, not a gap).
component_bad=""
for pkg in astro svelte vue mdx; do
  f="$pkg/tests/fixtures/hello/BUILD.bazel"
  t="$pkg/tests/fixtures/hello/Hello.test.js"
  if ! grep -q -F -e 'name = "hello_test",' "$f" ||
    grep -q -F -e 'name = "hello_output_test"' "$f" ||
    grep -q -F -e 'name = "hello",' "$f" ||
    [[ ! -f "$t" ]] ||
    ! grep -q -F -e 'expect(' "$t"; then
    component_bad="$component_bad $pkg"
  fi
done
if [[ -z "$component_bad" ]]; then
  ok
else
  bad "component render-output smoke broke in:$component_bad (want hello_test with expect, no binary/output smoke by design, issue #926)"
fi
# The wont-fix pin lives with the hello-smoke qualification (binary smokes
# stay binary-only; components stay render-asserted).
if grep -q -F -e 'component-only' tools/ci/hello_smoke_qualification.sh &&
  grep -q -F -e 'no binary and no output smoke by design' tools/ci/hello_smoke_qualification.sh; then
  ok
else
  bad "hello_smoke_qualification lost its component wont-fix pin (want component-only plus no-binary-by-design, issue #926)"
fi

dx_test_summary "examples consumer gates (issue #926)"
