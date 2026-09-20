#!/usr/bin/env bash
# Strict generation qualification harness (issue #503).
#
# Defines plus proves the strict-generation slice of the native baseline with
# fixture evidence, without claiming a qualified hermetic-llvm backend,
# qualified cross routes, qualified coverage, or Supported:
# - quoted includes contribute basename identities and resolve strictly or
#   fail; angle includes are toolchain-provided and never produce an edge.
# - ambiguous basenames fail (first-candidate selection rejected); macro
#   includes contribute no edge and synthesize no ignore; comments, string
#   and character literals plus raw strings stay inert; backslash-newline
#   continuations join before matching.
# - authoritative metadata is the local rule index plus exact resolve
#   mappings plus exact ignores, never the gazelle_cc module index; C/C++
#   carries no ecosystem lockfile.
# - generated headers have no checked-in owner and require an exact resolve
#   mapping; inferred producers stay rejected.
# - test grouping keeps *_test sources out of the generated library.
# - assembly dialects (.s/.S/.asm) stay undiscovered and handwritten.
# - named modules, header units and PCH stay handwritten; compiler flags
#   alone are not complete support.
# - union of literal identities with no select() and no preprocessor-derived
#   platform selection (not the approved Go-only exception).
# - deterministic names with collisions failing; stale targets clean through
#   normal merge with keep protection, never deleting a BUILD file.
# - bounded strictness: unresolved plus ambiguous plus stale-ignore plus
#   mapping-plus-ignore plus kind-mismatch plus main failures; log parsing
#   plus a replacement preprocessor stay rejected. Loose generation rejected.
# - open with honest records: backends stay provisional, floors qualified
#   seed-only under issue #500, coverage qualified seed-only under issue
#   #501, linux corpus qualified seed-only under issue #499.
#
# Versioned here, run by CI via `bazel run //tools/ci:strict_generation_qualification`,
# following //tools/ci:lcov_accounting_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers (issue #319).
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

dx_cd_workspace

dx_test_init

pins="cc/tests/fixtures/strict_generation/pins.bzl"
pins_build="cc/tests/fixtures/strict_generation/BUILD.bazel"
strict_h="cc/tests/fixtures/strict_generation/strict.h"
strict_cc="cc/tests/fixtures/strict_generation/strict.cc"
strict_test="cc/tests/fixtures/strict_generation/strict_test.cc"
includes_expected="cc/tests/fixtures/strict_generation/includes.expected"
resolution="cc/tests/fixtures/strict_generation/resolution.txt"
disposition="cc/tests/fixtures/strict_generation/disposition.txt"
native="docs/native-toolchains.md"
build="tools/ci/BUILD.bazel"
ci=".github/workflows/ci.yml"
verify="docs/testing/verification-matrix.md"

# Fixture files stay present (issue #503).
if [[ -f "$pins" && -f "$pins_build" && -f "$strict_h" && -f "$strict_cc" && -f "$strict_test" && -f "$includes_expected" && -f "$resolution" && -f "$disposition" ]]; then
  ok
else
  bad "strict generation fixture missing (want $pins plus $pins_build plus strict.h/cc/test.cc plus includes.expected plus resolution.txt plus disposition.txt)"
fi

# Pins record quoted plus angle include identities.
if grep -q -F -e 'every quoted include contributes its basename and resolves strictly or fails generation' "$pins" &&
  grep -q -F -e 'cc/tests/fixtures/strict_generation/strict.h -> strict.h' "$pins" &&
  grep -q -F -e 'angle includes are toolchain-provided and never produce an edge' "$pins" &&
  grep -q -F -e 'GAZELLE_CC_VERSION = "v0.6.0"' "$pins" &&
  grep -q -F -e '50dbcbcfd9199c19a50522695c568b9380caabe5' "$pins"; then
  ok
else
  bad "pins.bzl lost its quoted plus angle include identities plus gazelle_cc v0.6.0 pin under issue #503"
fi

# Pins record ambiguous plus macro plus inert plus continuation behavior.
if grep -q -F -e 'two libraries owning the same header basename are ambiguous and fail resolution' "$pins" &&
  grep -q -F -e 'ambiguity can select a first candidate' "$pins" &&
  grep -q -F -e 'macro include contributes no edge and synthesizes no ignore' "$pins" &&
  grep -q -F -e 'comments, string literals, character literals, and raw strings are inert' "$pins" &&
  grep -q -F -e 'backslash-newline continuations are joined before matching' "$pins"; then
  ok
else
  bad "pins.bzl lost its ambiguous plus macro plus inert plus continuation strictness under issue #503"
fi

# Pins record authoritative metadata: local index plus exact mappings, never the module index.
if grep -q -F -e 'exact # gazelle:resolve mapping' "$pins" &&
  grep -q -F -e 'local rule index' "$pins" &&
  grep -q -F -e 'exact # gazelle:dx_ignore_import exception' "$pins" &&
  grep -q -F -e "its module index is not the consumer's resolved graph" "$pins" &&
  grep -q -F -e 'authoritative metadata is the local rule index plus exact mappings, never the module index' "$pins"; then
  ok
else
  bad "pins.bzl lost its authoritative resolution order plus module-index rejection under issue #503"
fi

# Pins record generated-header disposition (exact mapping required, no inferred producer).
if grep -q -F -e 'generated headers have no checked-in owner and require an exact resolve mapping' "$pins" &&
  grep -q -F -e 'inferred generated producer' "$pins"; then
  ok
else
  bad "pins.bzl lost its generated-header exact-mapping disposition under issue #503"
fi

# Pins record test grouping plus assembly dialects.
if grep -q -F -e 'test-owned sources never enter the generated library' "$pins" &&
  grep -q -F -e 'TEST_SUFFIX = "_test"' "$pins" &&
  grep -q -F -e 'assembly sources are undiscovered and stay handwritten' "$pins" &&
  grep -q -F -e '".S"' "$pins" &&
  grep -q -F -e '".asm"' "$pins"; then
  ok
else
  bad "pins.bzl lost its test grouping plus assembly-dialect disposition under issue #503"
fi

# Pins record module plus PCH disposition (handwritten, flags not support).
if grep -q -F -e 'named C++ modules, header units and PCH have no complete generation route and stay handwritten' "$pins" &&
  grep -q -F -e 'compiler flags as complete support' "$pins"; then
  ok
else
  bad "pins.bzl lost its module plus PCH handwritten disposition under issue #503"
fi

# Pins record union-of-literals with no select and no preprocessor platform selection.
if grep -q -F -e 'generated dependencies are the deduplicated union of literal identities' "$pins" &&
  grep -q -F -e 'preprocessor-derived platform selections' "$pins"; then
  ok
else
  bad "pins.bzl lost its union-of-literals plus preprocessor-selection rejection under issue #503"
fi

# Pins record names plus lifecycle (deterministic names, collisions fail, keep plus stale merge).
if grep -q -F -e 'basename-derived names normalize deterministically and collisions fail with every claimant' "$pins" &&
  grep -q -F -e 'stale generated targets clean through normal merge with keep protection' "$pins" &&
  grep -q -F -e 'deleting a BUILD file' "$pins"; then
  ok
else
  bad "pins.bzl lost its naming plus lifecycle conformance gates under issue #503"
fi

# Pins record bounded strictness (fail-closed diagnostics, no log parsing or replacement preprocessor).
if grep -q -F -e 'every unknown or ambiguous literal reference fails with actionable context' "$pins" &&
  grep -q -F -e 'log parsing' "$pins" &&
  grep -q -F -e 'a replacement C++ preprocessor' "$pins" &&
  grep -q -F -e 'unknown angle includes may disappear' "$pins" &&
  grep -q -F -e 'missing module mappings can warn and omit edges' "$pins"; then
  ok
else
  bad "pins.bzl lost its bounded strictness plus log-parsing and preprocessor rejection under issue #503"
fi

# Pins record the rejected loose substitutes.
if grep -q -F -e '"loose generation"' "$pins" &&
  grep -q -F -e '"ambiguity can select a first candidate"' "$pins" &&
  grep -q -F -e '"compiler flags as complete support"' "$pins" &&
  grep -q -F -e '"inferred generated producer"' "$pins" &&
  grep -q -F -e '"preprocessor-derived platform selections"' "$pins"; then
  ok
else
  bad "pins.bzl lost its loose-generation rejection list under issue #503"
fi

# Fixture sources prove the buildable strict shape plus expected identities.
if grep -q -F -e '#include "cc/tests/fixtures/strict_generation/strict.h"' "$strict_cc" &&
  grep -q -F -e '#include <string>' "$strict_cc" &&
  grep -q -F -e 'StrictGreet' "$strict_h" &&
  grep -q -F -e 'strict.h' "$includes_expected" &&
  grep -q -F -e 'no edge (toolchain-provided)' "$includes_expected" &&
  grep -q -F -e 'contributes no edge' "$includes_expected" &&
  grep -q -F -e 'authoritative metadata is the local rule index plus exact mappings, never the module index' "$resolution" &&
  grep -q -F -e 'generated headers have no checked-in owner and require an exact resolve mapping' "$resolution" &&
  grep -q -F -e 'test-owned sources never enter the generated library' "$disposition" &&
  grep -q -F -e 'assembly sources are undiscovered and stay handwritten' "$disposition" &&
  grep -q -F -e 'stay handwritten' "$disposition"; then
  ok
else
  bad "strict fixture sources plus includes.expected plus resolution.txt plus disposition.txt lost strict coverage under issue #503"
fi

# Native plan owns the qualified strict-generation record plus fixture proof.
if grep -q -F -e 'qualified seed-only under issue #503' "$native" &&
  grep -q -F -e 'strict_generation_qualification' "$native" &&
  grep -q -F -e 'cc/tests/fixtures/strict_generation/pins.bzl' "$native" &&
  grep -q -F -e 'Can generation satisfy strict ownership and resolution cheaply?' "$native" &&
  grep -q -F -e 'loose generation rejected' "$native"; then
  ok
else
  bad "docs/native-toolchains.md lost its qualified strict-generation record with fixtures under issue #503"
fi

# Verification matrix owns the qualified seed-only record under #503.
if grep -q -F -e 'strict_generation_qualification' "$verify" &&
  grep -q -F -e 'qualified seed-only under #503' "$verify" &&
  grep -q -F -e 'bazel run //tools/ci:strict_generation_qualification' "$verify" &&
  grep -q -F -e '`strict_generation_qualification` 16/16' "$verify"; then
  ok
else
  bad "verification-matrix lost its #503 strict generation qualified record"
fi

# BUILD owns the harness target plus CI wires it in dogfood-freshness.
if grep -q -F -e 'name = "strict_generation_qualification"' "$build" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:strict_generation_qualification' "$ci"; then
  ok
else
  bad "tools/ci/BUILD.bazel or ci.yml lost the strict_generation_qualification wiring (want target plus dogfood-freshness)"
fi

# Live proof: the strict lib builds, the strict test passes, and the Go
# strict suites prove the negative shapes hermetically without checking in
# failing BUILD graphs (unresolved plus ambiguous plus macro-inert plus
# test-grouping plus naming plus resolution).
if bazel build //cc/tests/fixtures/strict_generation:strict --noshow_progress >/dev/null 2>&1 &&
  bazel test //cc/tests/fixtures/strict_generation:strict_test --noshow_progress >/dev/null 2>&1 &&
  bazel test //gazelle/cc:cc_test --noshow_progress >/dev/null 2>&1 &&
  bazel test //gazelle/cc:generation_test --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "strict generation live proof failed (want strict lib build plus strict_test plus cc_test plus generation_test green, issue #503)"
fi

dx_test_summary "strict generation qualification harness"
