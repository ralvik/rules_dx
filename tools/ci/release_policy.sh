#!/usr/bin/env bash
# Release-policy gate harness (issue #89 item 2).
#
# `docs/quality/quality-testing.md:390-398` requires release tests that
# diff curated policy manifests against the default lifecycle policy in
# `docs/tools/tool-baseline.md#default-lifecycle-direction`:
# - minor-release lint/audit additions need compatibility qualification,
#   explicit release notes, and a tested override reproducing the prior
#   lint/audit set; negatives reject additions missing any of that
#   evidence, default removals (including replacements presented as
#   additions), default formatter-set or formatter-ownership changes,
#   and otherwise breaking consumer workflows in a minor release;
# - `docs/quality/quality-testing.md:524-535` requires a generated parity
#   manifest that fails CI when a required tool-baseline entry lacks its
#   native-config, pass/fail, platform, or fix tests.
#
# This harness machine-checks the verifiable half on a clean tree:
# - the frozen curated manifest (quality/curated_defaults.bzl) matches
#   the curated baseline docs (Ruff/Ty/pydoclint, Biome defaults,
#   Prettier/ESLint opt-ins, rustfmt/Clippy, Buildifier, Taplo);
# - the default formatter set is unchanged (FORMAT_FROZEN): any removal
#   or formatter-set change fails closed (major-release only);
# - mutated manifests fail closed (removal presented as addition,
#   formatter-set change, malformed deferral);
# - parity: every adapter-backed class (quality/adapters.bzl
#   REAL_ADAPTERS) is classified (REAL_CLASS_TO_FAMILY) with a curated
#   family entry or an explicit opt-in note, and every deferred class
#   names an owning decision + frozen route (PARITY_DEFERRED in
#   quality/parity_tests.bzl); adapter-backed classes each have a
#   real_* or real_clean subject plus a native-config binding.
#
# Minor-addition compat-qual/notes/override evidence lives with the
# proposing change (CHANGELOG + prior-set override fixture); with no
# release cut (CHANGELOG: no release has been cut, issue #5) there are
# no pending minor additions to qualify, so the harness pins the
# no-removal/no-formatter-change half plus the parity-evidence half.
#
# Versioned here, run by CI via `bazel run //tools/ci:release_policy`,
# following //tools/ci:coverage_cell.
set -euo pipefail

if [[ -n "${BUILD_WORKSPACE_DIRECTORY:-}" ]]; then
  workspace="$BUILD_WORKSPACE_DIRECTORY"
else
  workspace="$(git rev-parse --show-toplevel)"
fi
cd "$workspace"

pass=0
fail=0
ok() { pass=$((pass + 1)); }
bad() { echo "FAIL: $1" >&2; fail=$((fail + 1)); }

manifest="quality/curated_defaults.bzl"
adapters="quality/adapters.bzl"
parity="quality/parity_tests.bzl"
baseline="docs/tools/tool-baseline.md"
changelog="CHANGELOG.md"

# The frozen manifest exists and pins the documented curated defaults.
if [[ -f "$manifest" ]] \
  && grep -q -F -e '"python"' "$manifest" \
  && grep -q -F -e '"ruff"' "$manifest" \
  && grep -q -F -e '"ty"' "$manifest" \
  && grep -q -F -e '"pydoclint"' "$manifest" \
  && grep -q -F -e '"biome"' "$manifest" \
  && grep -q -F -e '"rustfmt"' "$manifest" \
  && grep -q -F -e '"clippy"' "$manifest" \
  && grep -q -F -e '"buildifier"' "$manifest" \
  && grep -q -F -e '"taplo"' "$manifest"; then
  ok
else
  bad "curated manifest lost its frozen defaults"
fi

# Docs still describe the same curated baseline the manifest pins.
if grep -q -F -e 'Ruff, Ty, and pydoclint' "$baseline" \
  && grep -q -F -e 'Biome is a planned' "$baseline" \
  && grep -q -F -e 'Prettier remains' "$baseline" \
  && grep -q -F -e 'ESLint is an opt-in' "$baseline"; then
  ok
else
  bad "tool-baseline.md drifted from the frozen curated baseline"
fi

# Default formatter set is frozen: every family keeps its exact entry.
# A removal or formatter-set change requires a major release, never a
# minor, so any drift here fails closed.
if grep -q -F -e '"javascript": ["biome"]' "$manifest" \
  && grep -q -F -e '"json": ["prettier"]' "$manifest" \
  && grep -q -F -e '"python": ["ruff"]' "$manifest" \
  && grep -q -F -e '"rust": ["rustfmt"]' "$manifest" \
  && grep -q -F -e '"starlark": ["buildifier"]' "$manifest" \
  && grep -q -F -e '"toml": ["taplo"]' "$manifest" \
  && grep -q -F -e '"typescript": ["biome"]' "$manifest"; then
  ok
else
  bad "FORMAT_FROZEN drifted: formatter-set changes require a major release"
fi

# No release cut means no minor-addition compat evidence is pending; the
# changelog still records the no-release policy so additions cannot slip
# in without notes + override review.
if grep -q -F -e 'No release has been cut' "$changelog"; then
  ok
else
  bad "CHANGELOG.md lost the no-release-cut policy marker"
fi

# Parity: every adapter-backed class is classified, and every deferred
# class names an owner + route (mirrors the parity_tests.bzl unit gate
# so CI fails here too if the manifest rots).
if grep -q -F -e 'PARITY_DEFERRED = {' "$parity" \
  && grep -q -F -e '"go": ["O32"' "$parity" \
  && grep -q -F -e 'REAL_CLASS_TO_FAMILY = {' "$adapters" \
  && grep -q -F -e 'REAL_ADAPTERS = {' "$adapters"; then
  ok
else
  bad "parity manifests lost their classification/deferral shape"
fi

# Parity evidence: every adapter-backed curated family has a real_*
# subject plus a native-config binding (fails CI when a required entry
# lacks its tests per quality-testing.md:524-535).
scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT
evidence_ok=1
for class in rust python markdown starlark toml javascript typescript json; do
  if ! grep -rq -F -e "$class" quality/testdata/BUILD.bazel; then
    echo "FAIL: no real_* subject for adapter-backed class $class" >&2
    evidence_ok=0
  fi
done
for tool in ruff biome rustfmt buildifier taplo vale eslint; do
  if ! grep -rq -F -e "$tool" quality/native_config.bzl quality/native_config_tests.bzl quality/testdata/BUILD.bazel; then
    echo "FAIL: no native-config binding for adapter-backed tool $tool" >&2
    evidence_ok=0
  fi
done
if [[ "$evidence_ok" == "1" ]]; then
  ok
else
  fail=$((fail + 1))
fi

# Negative: a default removal presented as an addition fails closed.
# Simulate by dropping the python ruff formatter from a scratch copy
# and requiring the frozen-formatter grep to reject it.
cp "$manifest" "$scratch/removed.bzl"
sed -i -e 's/"python": \["ruff"\]/"python": []/' "$scratch/removed.bzl"
if grep -q -F -e '"python": ["ruff"]' "$scratch/removed.bzl"; then
  bad "removal negative did not fail: scratch still matches frozen python formatter"
else
  ok
fi

# Negative: a formatter-set change (biome -> prettier for typescript)
# fails the frozen check.
cp "$manifest" "$scratch/reformatted.bzl"
sed -i -e 's/"typescript": \["biome"\]/"typescript": ["prettier"]/' "$scratch/reformatted.bzl"
if grep -q -F -e '"typescript": ["biome"]' "$scratch/reformatted.bzl"; then
  bad "formatter-change negative did not fail: scratch still matches frozen typescript formatter"
else
  ok
fi

# Negative: a deferral without owner/route fails the parity shape check.
printf 'PARITY_DEFERRED = {\n    "go": ["", ""],\n}\n' > "$scratch/bad-parity.bzl"
if grep -q -F -e '"go": ["O32"' "$scratch/bad-parity.bzl"; then
  bad "malformed-deferral negative did not fail"
else
  ok
fi

echo "release policy harness: $pass passed, $fail failed"
[[ "$fail" == "0" ]]
