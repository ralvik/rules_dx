#!/usr/bin/env bash
# Strict preset qualification harness (issue #615).
#
# Decides default-loose vs strict-opt-in for users with fixture plus docs
# evidence:
# - decision: default stays loose (curated defaults plus pinned upstream
#   built-in defaults, no hidden presets); strict is opt-in via checked-in
#   native configs copied from the documented examples; forcing strict by
#   default is rejected (upgrade break, needs a major release).
# - fixtures: `quality/tests/fixtures/strict_preset/` loose vs strict
#   pairs (ruff E4/E7/E9/F vs E/F/W/I/N/UP/B/SIM, biome {} vs recommended
#   plus noExplicitAny plus useConst plus noUnusedVariables, tsconfig
#   strict false vs true) plus illustrative loose-pass strict-fail sources.
# - docs: `docs/quality/strict-preset.md` owns the decision with copy-paste
#   examples and the gate; index plus corpus plus baseline link it.
# - open owned gaps: platform plus consumer plus release evidence, no
#   `Supported` claim. Compatibility is quality only.
#
# Versioned here, run by CI via `bazel run //tools/ci:strict_preset_qualification`,
# following //tools/ci:quality_taxonomy_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers (issue #319).
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

dx_cd_workspace

dx_test_init

pins="quality/tests/fixtures/strict_preset/pins.bzl"
pins_build="quality/tests/fixtures/strict_preset/BUILD.bazel"
ruff_loose="quality/tests/fixtures/strict_preset/ruff_loose.toml"
ruff_strict="quality/tests/fixtures/strict_preset/ruff_strict.toml"
biome_loose="quality/tests/fixtures/strict_preset/biome_loose.json"
biome_strict="quality/tests/fixtures/strict_preset/biome_strict.json"
ts_loose="quality/tests/fixtures/strict_preset/tsconfig_loose.json"
ts_strict="quality/tests/fixtures/strict_preset/tsconfig_strict.json"
ex_py="quality/tests/fixtures/strict_preset/loose_pass_strict_fail.py"
ex_ts="quality/tests/fixtures/strict_preset/loose_pass_strict_fail.ts"
doc="docs/quality/strict-preset.md"
index="docs/quality/README.md"
corpus="docs/BUILD.bazel"
baseline="docs/tools/tool-baseline.md"
native="quality/native_config.bzl"
curated="quality/curated_defaults.bzl"
build="tools/ci/BUILD.bazel"
ci=".github/workflows/ci.yml"
verify="docs/testing/verification-matrix.md"
root_ruff="ruff.toml"
root_biome="biome.json"

# Fixture set stays present (issue #615).
if [[ -f "$pins" && -f "$pins_build" && -f "$ruff_loose" && -f "$ruff_strict" && -f "$biome_loose" && -f "$biome_strict" && -f "$ts_loose" && -f "$ts_strict" && -f "$ex_py" && -f "$ex_ts" ]]; then
  ok
else
  bad "strict preset fixture missing (want pins plus BUILD plus ruff/biome/tsconfig loose/strict pairs plus example sources, issue #615)"
fi

# Pins record the default-loose plus strict-opt-in decision plus rejection.
if grep -q -F -e 'DEFAULT_POLICY = "default stays loose' "$pins" &&
  grep -q -F -e 'STRICT_OPT_IN = "strict is opt-in via checked-in native configs' "$pins" &&
  grep -q -F -e 'REJECTED_FORCING_STRICT = "forcing strict by default rejected' "$pins" &&
  grep -q -F -e 'issue #615' "$pins"; then
  ok
else
  bad "pins.bzl lost its default-loose plus strict-opt-in decision plus rejection under issue #615"
fi

# Pins record the ruff loose vs strict selections.
if grep -q -F -e 'RUFF_LOOSE_SELECT' "$pins" &&
  grep -q -F -e 'E4' "$pins" &&
  grep -q -F -e 'RUFF_STRICT_SELECT' "$pins" &&
  grep -q -F -e '"SIM"' "$pins" &&
  grep -q -F -e 'RUFF_STRICT_SUPERSET' "$pins"; then
  ok
else
  bad "pins.bzl lost its ruff loose vs strict selections under issue #615"
fi

# Pins record the biome plus tsc plus vale strict resolutions.
if grep -q -F -e 'BIOME_STRICT_RECOMMENDED' "$pins" &&
  grep -q -F -e 'BIOME_STRICT_NO_EXPLICIT_ANY' "$pins" &&
  grep -q -F -e 'BIOME_STRICT_USE_CONST' "$pins" &&
  grep -q -F -e 'TSC_STRICT' "$pins" &&
  grep -q -F -e 'VALE_STRICT' "$pins" &&
  grep -q -F -e 'prose wont-fix' "$pins"; then
  ok
else
  bad "pins.bzl lost its biome plus tsc plus vale strict resolutions under issue #615"
fi

# Ruff strict is a strict superset of loose.
if grep -q -F -e 'select = ["E4", "E7", "E9", "F"]' "$ruff_loose" &&
  grep -q -F -e 'select = ["E", "F", "W", "I", "N", "UP", "B", "SIM"]' "$ruff_strict"; then
  ok
else
  bad "ruff loose/strict pair lost its superset shape (want E4/E7/E9/F loose plus E/F/W/I/N/UP/B/SIM strict, issue #615)"
fi

# Biome strict enables recommended plus strict errors; loose stays empty.
if grep -q -F -e '{}' "$biome_loose" &&
  grep -q -F -e '"recommended": true' "$biome_strict" &&
  grep -q -F -e '"noExplicitAny": "error"' "$biome_strict" &&
  grep -q -F -e '"useConst": "error"' "$biome_strict" &&
  grep -q -F -e '"noUnusedVariables": "error"' "$biome_strict"; then
  ok
else
  bad "biome loose/strict pair lost its empty vs recommended-plus-strict shape under issue #615"
fi

# TSConfig strict true vs loose false.
if grep -q -F -e '"strict": true' "$ts_strict" &&
  grep -q -F -e '"strict": false' "$ts_loose"; then
  ok
else
  bad "tsconfig loose/strict pair lost its strict false vs true shape under issue #615"
fi

# Example sources illustrate loose-pass strict-fail.
if grep -q -F -e 'import sys' "$ex_py" &&
  grep -q -F -e 'import os' "$ex_py" &&
  grep -q -F -e 'function greet(name)' "$ex_ts" &&
  grep -q -F -e ': any' "$ex_ts"; then
  ok
else
  bad "example sources lost their loose-pass strict-fail illustrations under issue #615"
fi

# Doc owns the decision with loose plus strict plus rejected plus gate.
if [[ -f "$doc" ]] &&
  grep -q -F -e 'default stays loose' "$doc" &&
  grep -q -F -e 'strict is opt-in' "$doc" &&
  grep -q -F -e 'Forcing strict' "$doc" &&
  grep -q -F -e 'rejected' "$doc" &&
  grep -q -F -e 'select = ["E", "F", "W", "I", "N", "UP", "B", "SIM"]' "$doc" &&
  grep -q -F -e '"noExplicitAny": "error"' "$doc" &&
  grep -q -F -e '"strict": true' "$doc" &&
  grep -q -F -e 'markers-only' "$doc" &&
  grep -q -F -e '//tools/ci:strict_preset_qualification' "$doc" &&
  grep -q -F -e 'issue #615' "$doc"; then
  ok
else
  bad "docs/quality/strict-preset.md lost its decision plus examples plus gate record (want loose plus strict plus rejected plus gate, issue #615)"
fi

# Doc stays linked from the quality index and corpus.
if grep -q -F -e 'strict-preset.md' "$index" &&
  grep -q -F -e 'quality/strict-preset.md' "$corpus"; then
  ok
else
  bad "strict-preset doc lost its index or corpus link (want README plus docs/BUILD.bazel, issue #615)"
fi

# Baseline links the opt-in strict preset without authorizing hidden presets.
if grep -q -e 'does not' "$baseline" &&
  grep -q -F -e 'hidden behavioral presets' "$baseline" &&
  grep -q -F -e 'strict-preset' "$baseline" &&
  grep -q -F -e 'opt-in' "$baseline"; then
  ok
else
  bad "tool-baseline lost its opt-in strict-preset link with no-hidden-preset honesty (want strict-preset plus opt-in, issue #615)"
fi

# No hidden strict preset: no selectable strict preset ID or workspace
# strict flag in the typed native-config rules or curated defaults.
strict_config=""
for token in strict_preset strict-preset STRICT_PRESET; do
  if grep -q -F -e "$token" "$native"; then
    strict_config="$strict_config native:$token"
  fi
  if grep -q -F -e "$token" "$curated"; then
    strict_config="$strict_config curated:$token"
  fi
done
if [[ -z "$strict_config" ]]; then
  ok
else
  bad "native-config or curated defaults carries a hidden strict preset:$strict_config"
fi

# Repository loose pins stay unchanged (defaults change only via user
# checked-in configs, never by upgrading rules_dx).
if grep -q -F -e 'select = ["E4", "E7", "E9", "F"]' "$root_ruff" &&
  grep -q -F -e '{}' "$root_biome"; then
  ok
else
  bad "root ruff.toml or biome.json drifted (want loose E4/E7/E9/F plus {} unchanged, issue #615)"
fi

# Verification matrix owns the qualified seed-only record under #615.
if grep -q -F -e 'strict_preset_qualification' "$verify" &&
  grep -q -F -e 'qualified seed-only under #615' "$verify" &&
  grep -q -F -e 'bazel run //tools/ci:strict_preset_qualification' "$verify" &&
  grep -q -F -e '`strict_preset_qualification` 16/16' "$verify"; then
  ok
else
  bad "verification-matrix lost its #615 strict preset qualified record"
fi

# BUILD owns the harness target plus CI wires it in dogfood-freshness.
if grep -q -F -e 'name = "strict_preset_qualification"' "$build" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:strict_preset_qualification' "$ci"; then
  ok
else
  bad "tools/ci/BUILD.bazel or ci.yml lost the strict_preset_qualification wiring (want target plus dogfood-freshness)"
fi

# Pins record the rejected hidden-preset plus forced-strict substitutes.
if grep -q -F -e 'STRICT_PRESET_REJECTED' "$pins" &&
  grep -q -F -e 'no selectable strict preset ID' "$pins" &&
  grep -q -F -e 'no forced strict default' "$pins"; then
  ok
else
  bad "pins.bzl lost its hidden-preset plus forced-strict rejection list under issue #615"
fi

dx_test_summary "strict preset qualification harness"
