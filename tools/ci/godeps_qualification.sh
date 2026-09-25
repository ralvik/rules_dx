#!/usr/bin/env bash
# Go from_file lock wiring qualification harness.
#
# Qualifies the owned gap from support-matrix provisional dependency locks:
# `go.mod`/`go.sum` via `go_deps.from_file`, closed owner only. 
# pinned the `go test` runner mapping, not the lock wiring itself.
# - wired: `third_party/go/go.mod` (buildtools pseudo-20250930 plus go-cmp
#   v0.6.0 plus difflib v1.0.0, matching gazelle 0.52.2 go.mod so the shared
#   go_deps extension sees no version conflict) plus `go.sum` (h1 plus
#   /go.mod hashes) via `go_deps.from_file(go_mod = ...)` into
#   `@com_github_*` repos carrying go.sum verification, loaded from
#   MODULE.bazel. Single-module layout: no `go.work` lands here; `go.work`
#   stays only for multi-module layouts. Stale locks fail closed.
# - fixtures: `go/tests/fixtures/godeps/` library plus test consume
#   `@com_github_google_go_cmp//cmp:cmp` via handwritten deps plus the exact
#   `# gazelle:resolve` mapping, depcheck testdata proves offline
#   lockfile-consistency plus usage authority (`go.mod` plus `go.sum`).
# - generation consumes never writes: the common generation contract owns
#   the never-writes rule; Gazelle never reads or writes the Go lock files
#   (handwritten BUILD deps name `@com_github_*` labels directly).
# - rejected: hand-written `go_deps.module` tags (Gazelle documents
#   `from_file` as preferred over hand tags); `go.work` for single-module
#   layouts.
# - scope: lock only. Per-platform SDK acquisition, platform plus consumer
#   plus release evidence stay open; no Supported claim.
#
# Versioned here, run by CI via `bazel run //tools/ci:godeps_qualification`,
# following //tools/ci:paket_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

dx_bash_pin

pins="go/tests/fixtures/godeps/pins.bzl"
pins_build="go/tests/fixtures/godeps/BUILD.bazel"
lib="go/tests/fixtures/godeps/godeps.go"
test="go/tests/fixtures/godeps/godeps_test.go"
gomod="third_party/go/go.mod"
gosum="third_party/go/go.sum"
gomod_build="third_party/go/BUILD.bazel"
module="MODULE.bazel"
toolchains="modules/toolchains.bzl"
lock="MODULE.bazel.lock"
common="docs/generation/common.md"
matrix="docs/product/support-matrix.md"
gen_readme="docs/generation/foundation-qualification.md"
build="tools/ci/BUILD.bazel"
ci=".github/workflows/ci.yml"
checker="tools/depcheck/src/lib.rs"

# Pins fixture stays present as the single owner.
if [[ -f "$pins" && -f "$pins_build" && -f "$lib" && -f "$test" ]]; then
  ok
else
  bad "godeps pins fixture missing (want $pins plus $pins_build plus $lib plus $test)"
fi

# Pins record the ruleset plus toolchain plus gazelle identities.
if grep -q -F -e 'RULES_GO_VERSION = "0.63.0"' "$pins" &&
  grep -q -F -e 'GO_SDK_VERSION = "1.26.6"' "$pins" &&
  grep -q -F -e 'GO_LANGUAGE_FLOOR = "1.24.12"' "$pins" &&
  grep -q -F -e 'GAZELLE_VERSION = "0.52.2"' "$pins"; then
  ok
else
  bad "pins.bzl lost its rules_go plus Go SDK plus language floor plus gazelle pins under issue #483"
fi

# Go version single-source (issue #1003): the SDK is the toolchain floor,
# go.mod carries the language floor tracking gazelle 0.52.2's go 1.24.12
# so the shared go_deps extension sees no version conflict; the SDK minor
# stays >= the floor. Canonical pins live in modules/toolchains.bzl
# (hermetic first-match extraction: host grep -o quirks diverge, #1006).
pins_sdk="$(dx_extract_re "$pins" '^GO_SDK_VERSION = "[^"]+"' | cut -d'"' -f2 || true)"
pins_floor="$(dx_extract_re "$pins" '^GO_LANGUAGE_FLOOR = "[^"]+"' | cut -d'"' -f2 || true)"
gomod_directive="$(dx_extract_re "$gomod" '^go [0-9]+\.[0-9]+(\.[0-9]+)?' | cut -d' ' -f2 || true)"
wrapper_sdk="$(dx_extract_re "$toolchains" '^GO_SDK_VERSION = "[^"]+"' | cut -d'"' -f2 || true)"
wrapper_floor="$(dx_extract_re "$toolchains" '^GO_LANGUAGE_FLOOR = "[^"]+"' | cut -d'"' -f2 || true)"
module_sdk="$(sed -n '/go_sdk.download(/,/)/p' "$module" | dx_hermetic_grep extract-re /dev/stdin 'version = "[^"]+"' | dx_hermetic_grep extract-re /dev/stdin '"[^"]+"$' | tr -d '"' || true)"
if [[ -n "$pins_sdk" && "$pins_sdk" == "$wrapper_sdk" && "$pins_sdk" == "$module_sdk" ]]; then
  ok
else
  bad "Go SDK drifts (pins $pins_sdk vs wrapper $wrapper_sdk vs MODULE $module_sdk; want single-source 1.26.6, issue #1003)"
fi
if [[ -n "$pins_floor" && "$pins_floor" == "$wrapper_floor" && "$pins_floor" == "$gomod_directive" ]]; then
  ok
else
  bad "Go language floor drifts (pins $pins_floor vs wrapper $wrapper_floor vs go.mod $gomod_directive; want single-source 1.24.12, issue #1003)"
fi
if [[ "$gomod_directive" == "1.24.12" ]]; then
  ok
else
  bad "third_party/go/go.mod go directive is $gomod_directive, want 1.24.12 tracking gazelle 0.52.2 (issue #1003)"
fi
if [[ -n "$pins_sdk" && -n "$pins_floor" ]]; then
  sdk_mm="$(echo "$pins_sdk" | cut -d. -f1,2)"
  floor_mm="$(echo "$pins_floor" | cut -d. -f1,2)"
  s_maj="${sdk_mm%%.*}"; s_min="${sdk_mm#*.}"
  f_maj="${floor_mm%%.*}"; f_min="${floor_mm#*.}"
  if [[ "$s_maj" -gt "$f_maj" ]] || { [[ "$s_maj" == "$f_maj" ]] && [[ "$s_min" -ge "$f_min" ]]; }; then
    ok
  else
    bad "Go SDK $pins_sdk predates language floor $pins_floor (want SDK minor >= floor, issue #1003)"
  fi
fi

# Pins record the from_file wiring plus single-module go.work rule.
if grep -q -F -e 'GODEPS_FROM_FILE = "go_deps.from_file"' "$pins" &&
  grep -q -F -e 'GODEPS_GO_MOD = "//third_party/go:go.mod"' "$pins" &&
  grep -q -F -e 'GODEPS_GO_SUM = "//third_party/go:go.sum"' "$pins" &&
  grep -q -F -e 'go.work only for multi-module' "$pins"; then
  ok
else
  bad "pins.bzl lost its from_file plus go.mod/go.sum plus go.work-only-multi-module wiring under issue #483"
fi

# Pins record the direct versions plus repos plus fixture labels plus rejected.
if grep -q -F -e 'BUILDTOOLS_VERSION = "v0.0.0-20250930140053-2eb4fccefb52"' "$pins" &&
  grep -q -F -e 'GO_CMP_VERSION = "v0.6.0"' "$pins" &&
  grep -q -F -e 'DIFLIB_VERSION = "v1.0.0"' "$pins" &&
  grep -q -F -e 'GODEPS_REPO_GO_CMP = "@com_github_google_go_cmp"' "$pins" &&
  grep -q -F -e '//go/tests/fixtures/godeps:godeps_test' "$pins" &&
  grep -q -F -e 'hand module tags rejected' "$pins" &&
  grep -q -F -e 'from_file preferred' "$pins"; then
  ok
else
  bad "pins.bzl lost its direct pins plus repos plus fixture labels plus rejected under issue #483"
fi

# go.mod pins the direct entries exactly.
if grep -q -F -e 'module rules_dx/third_party/go' "$gomod" &&
  grep -q -F -e 'github.com/bazelbuild/buildtools v0.0.0-20250930140053-2eb4fccefb52' "$gomod" &&
  grep -q -F -e 'github.com/google/go-cmp v0.6.0' "$gomod" &&
  grep -q -F -e 'github.com/pmezard/go-difflib v1.0.0' "$gomod"; then
  ok
else
  bad "third_party/go/go.mod lost its buildtools plus go-cmp plus difflib pins under issue #483"
fi

# go.sum pins the h1 plus /go.mod hashes (fail-closed verification).
if grep -q -F -e 'github.com/google/go-cmp v0.6.0 h1:ofyhxvXcZhMsU5ulbFiLKl/XBFqE1GSq7atu8tAmTRI=' "$gosum" &&
  grep -q -F -e 'github.com/google/go-cmp v0.6.0/go.mod h1:17dUlkBOakJ0+DkrSSNjCkIjxS6bF9zb3elmeNGIjoY=' "$gosum" &&
  grep -q -F -e 'github.com/bazelbuild/buildtools v0.0.0-20250930140053-2eb4fccefb52 h1:' "$gosum" &&
  grep -q -F -e 'github.com/pmezard/go-difflib v1.0.0 h1:' "$gosum"; then
  ok
else
  bad "third_party/go/go.sum lost its go-cmp plus buildtools plus difflib hashes under issue #483"
fi

# MODULE wires the go.mod hub with the qualified lock record.
if grep -q -F -e 'gazelle_go_deps.from_file(go_mod = "//third_party/go:go.mod")' "$module" &&
  grep -q -F -e 'use_repo(gazelle_go_deps, "com_github_bazelbuild_buildtools", "com_github_google_go_cmp", "com_github_pmezard_go_difflib")' "$module" &&
  grep -q -F -e 'bazel run //tools/ci:godeps_qualification' "$module"; then
  ok
else
  bad "MODULE.bazel lost its from_file go_mod hub wiring plus qualified-seed annotation (issue #483 rides foundation-qualification.md)"
fi

# Hand module tags stay rejected; go.work stays only for multi-module.
if ! grep -q -F -e 'go_deps.module(' "$module" &&
  ! grep -q -F -e 'go_work' "$module" &&
  ! find . -maxdepth 2 -name 'go.work' -not -path './bazel-*' 2>/dev/null | grep -q . &&
  ! find third_party/go -name 'go.work' 2>/dev/null | grep -q .; then
  ok
else
  bad "hand module tags or go.work detected (want from_file only, no go.work in single-module layout)"
fi

# Fixtures consume the hub: library plus test name the external label with the resolve mapping.
if grep -q -F -e '@com_github_google_go_cmp//cmp:cmp' "$pins_build" &&
  grep -q -F -e 'gazelle:resolve go github.com/google/go-cmp/cmp @com_github_google_go_cmp//cmp:cmp' "$pins_build" &&
  grep -q -F -e 'github.com/google/go-cmp/cmp' "$lib" &&
  grep -q -F -e 'github.com/google/go-cmp/cmp' "$test"; then
  ok
else
  bad "godeps fixtures lost their @com_github_google_go_cmp consumption plus resolve mapping under issue #483"
fi

# Generation consumes never writes: contract owns it, Gazelle never touches the lock files
# (hermetic tree search, issue #1006).
if grep -q -F -e 'or edits manifests or lockfiles' "$common" &&
  dx_tree_absent --include='*.go' 'third_party/go/go.mod' -- gazelle/go &&
  dx_tree_absent --include='*.go' 'third_party/go/go.sum' -- gazelle/go &&
  dx_tree_absent --include='*.go' 'go_deps.module' -- gazelle/go; then
  ok
else
  bad "generation lost its consumes-never-writes proof (want common contract plus no lock refs in gazelle/go)"
fi

# Generation README owns the qualified lock alongside the other gaps.
if grep -q -F -e 'qualified seed-only under issue #483' "$gen_readme" &&
  grep -q -F -e 'godeps_qualification' "$gen_readme" &&
  grep -q -F -e 'go.sum' "$gen_readme"; then
  ok
else
  bad "generation README lost its #483 qualified Go from_file lock record"
fi

# ci_targets_d.bzl owns the harness target plus dogfood-freshness wires it.
if grep -q -F -e 'name = "godeps_qualification"' "tools/ci/ci_targets_d.bzl" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:godeps_qualification' tools/ci/dogfood_freshness.sh; then
  ok
else
  bad "tools/ci/ci_targets_d.bzl or dogfood_freshness.sh lost the godeps_qualification wiring (want target plus dogfood-freshness)"
fi

# Depcheck proves offline lock authority with go fixtures plus parser.
if [[ -f "tools/depcheck/testdata/go/ok_used/go.mod" &&
  -f "tools/depcheck/testdata/go/ok_used/go.sum" ]] &&
  grep -q -F -e 'go.mod' "$checker" &&
  grep -q -F -e 'go.sum' "$checker"; then
  ok
else
  bad "depcheck lost its go lock authority fixtures plus parser under issue #483"
fi

# Live proof: the pinned hub resolves and the fixture executes green on the seed host.
if bazel query @com_github_google_go_cmp//cmp:cmp --noshow_progress >/dev/null 2>&1 &&
  bazel test //go/tests/fixtures/godeps:godeps_test --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "godeps live proof failed (want @com_github_google_go_cmp//cmp:cmp plus //go/tests/fixtures/godeps:godeps_test green)"
fi

dx_test_summary "godeps qualification harness"
