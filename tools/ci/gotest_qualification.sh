#!/usr/bin/env bash
# Go test wiring qualification harness.
#
# Qualifies the owned gap from closed: provisional go test via
# rules_go, closed owner only. covers from_file, not the runner.
# - pinned: rules_go 0.63.0 plus Go SDK 1.26.6 in MODULE.bazel (runner
#   follows the toolchain pin, no separate version) recorded in
#   `go/tests/fixtures/gotest/pins.bzl`; Go hello is stdlib-only with no
#   ecosystem lock.
# - runner: one package-level `go_test` per directory owning every
#   `*_test.go` via `embed` (internal plus external test packages
#   coexist); the library reaches the test via `embed`, never an import
#   edge. The `go_test` wrapper in `go/rules/defs.bzl` preserves the
#   upstream providers plus QualitySourcesInfo. Implicit runner (bare
#   `go test` without the wrapper `go_test` plus `embed` mapping) stays
#   rejected. Gazelle emits the package-level shape (`gazelle/go/lang.go`
#   plus `lang_test.go`); the live proof is
#   `//go/tests/fixtures/hello:hello_test`.
# - open owned gaps: platform plus consumer plus release evidence, no
#   `Supported` claim. Compatibility is test mapping only.
#
# Versioned here, run by CI via `bazel run //tools/ci:gotest_qualification`,
# following //tools/ci:junit_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

pins="go/tests/fixtures/gotest/pins.bzl"
gotest_build="go/tests/fixtures/gotest/BUILD.bazel"
hello_build="go/tests/fixtures/hello/BUILD.bazel"
hello_test="go/tests/fixtures/hello/hello_test.go"
wrapper="go/rules/defs.bzl"
lang="gazelle/go/lang.go"
lang_test="gazelle/go/lang_test.go"
module="MODULE.bazel"
matrix="docs/product/support-matrix.md"
gen_readme="docs/generation/README.md"
build="tools/ci/BUILD.bazel"
ci=".github/workflows/ci.yml"
verify="docs/testing/verification-matrix.md"

# Fixture pins plus hello consumer stay present.
if [[ -f "$pins" && -f "$gotest_build" && -f "$hello_build" && -f "$hello_test" ]]; then
  ok
else
  bad "gotest fixture missing (want $pins plus $gotest_build plus $hello_build plus $hello_test)"
fi

# Pins record the ruleset plus toolchain identities plus runner mapping.
if grep -q -F -e 'RULES_GO_VERSION = "0.63.0"' "$pins" &&
  grep -q -F -e 'GO_SDK_VERSION = "1.26.6"' "$pins" &&
  grep -q -F -e 'GOTEST_KIND = "go_test"' "$pins" &&
  grep -q -F -e 'GOTEST_RUNNER = "go test"' "$pins" &&
  grep -q -F -e 'GOTEST_EMBED_ATTR = "embed"' "$pins"; then
  ok
else
  bad "pins.bzl lost its rules_go plus Go SDK plus go_test embed mapping under issue #478"
fi

# Pins record the live fixture label plus rejected implicit runner.
if grep -q -F -e '//go/tests/fixtures/hello:hello_test' "$pins" &&
  grep -q -F -e 'implicit runner rejected' "$pins"; then
  ok
else
  bad "pins.bzl lost its hello fixture label plus rejected implicit runner under issue #478"
fi

# MODULE pins the ruleset plus toolchain (runner follows the toolchain).
if grep -q -F -e 'bazel_dep(name = "rules_go", version = "0.63.0")' "$module" &&
  grep -q -F -e 'go_sdk.download(version = "1.26.6")' "$module"; then
  ok
else
  bad "MODULE.bazel lost its rules_go 0.63.0 plus Go SDK 1.26.6 pins under issue #478"
fi

# Wrapper preserves upstream providers plus QualitySourcesInfo with a go_test def.
if grep -q -F -e 'GoInfo' "$wrapper" &&
  grep -q -F -e 'GoArchive' "$wrapper" &&
  grep -q -F -e 'QualitySourcesInfo' "$wrapper" &&
  grep -q -F -e 'def go_test' "$wrapper" &&
  grep -q -F -e 'rules_go 0.63.0' "$wrapper"; then
  ok
else
  bad "go/rules/defs.bzl lost its go_test provider mapping under issue #478"
fi

# Wrapper keeps the package-level test shape (private upstream plus embed).
if grep -q -F -e '_go_forward_test' "$wrapper" &&
  grep -q -F -e '_upstream' "$wrapper" &&
  grep -q -F -e 'embed' "$wrapper"; then
  ok
else
  bad "go/rules/defs.bzl lost its package-level go_test shape (want forwarder plus private upstream plus embed) under issue #478"
fi

# Hello fixture keeps the wrapper consumer shape (go_test plus embed plus srcs).
if grep -q -F -e 'go/rules:defs.bzl' "$hello_build" &&
  grep -q -F -e 'go_test' "$hello_build" &&
  grep -q -F -e 'embed' "$hello_build" &&
  grep -q -F -e 'hello_test.go' "$hello_build" &&
  grep -q -F -e ':hello_lib' "$hello_build"; then
  ok
else
  bad "go/tests/fixtures/hello lost its wrapper go_test plus embed consumer shape under issue #478"
fi

# Hello test source stays a stdlib-only package-level test.
if grep -q -F -e 'import "testing"' "$hello_test" &&
  grep -q -F -e 'func TestHello' "$hello_test"; then
  ok
else
  bad "go/tests/fixtures/hello/hello_test.go lost its stdlib TestHello shape under issue #478"
fi

# Gazelle keeps the go_test kinds plus wrapper loads with embed.
if grep -q -F -e 'go_test' "$lang" &&
  grep -q -F -e '"embed"' "$lang" &&
  grep -q -F -e '//go/rules:defs.bzl' "$lang"; then
  ok
else
  bad "gazelle/go/lang.go lost its go_test plus embed mapping under issue #478"
fi

# Gazelle tests keep the package-level embed proof (no per-file targets).
if grep -q -F -e 'go_test(demo_test)' "$lang_test" &&
  grep -q -F -e '":demo"' "$lang_test"; then
  ok
else
  bad "gazelle/go/lang_test.go lost its package-level go_test embed proof under issue #478"
fi

# Implicit runner stays rejected: fixtures never load upstream go_test directly.
if ! grep -R --include='BUILD.bazel' -F -e '@rules_go//go:def.bzl' -- go/tests/fixtures 2>/dev/null | grep -q .; then
  ok
else
  bad "implicit Go runner detected (want wrapper go_test only, no direct @rules_go load in fixtures)"
fi

# Support matrix keeps the qualified Go gap wording.
if grep -q -F -e 'go test` qualified seed-only under issue #478' "$matrix" &&
  grep -q -F -e 'gotest_qualification' "$matrix" &&
  grep -q -F -e '#478' "$matrix"; then
  ok
else
  bad "docs/product/support-matrix.md lost its qualified Go test wording under issue #478"
fi

# Generation README pins the qualified runner alongside the other gaps.
if grep -q -F -e 'qualified seed-only under issue #478' "$gen_readme" &&
  grep -q -F -e 'gotest_qualification' "$gen_readme" &&
  grep -q -F -e 'issue #478' "$gen_readme"; then
  ok
else
  bad "docs/generation/README.md lost its qualified Go test record under issue #478"
fi

# Verification matrix owns the qualified seed-only record under.
if grep -q -F -e 'gotest_qualification' "$verify" &&
  grep -q -F -e 'qualified seed-only under #478' "$verify" &&
  grep -q -F -e 'bazel run //tools/ci:gotest_qualification' "$verify" &&
  grep -q -F -e '`gotest_qualification` 16/16' "$verify"; then
  ok
else
  bad "verification-matrix lost its #478 Go test qualified record"
fi

# BUILD owns the harness target plus CI wires it in dogfood-freshness.
if grep -q -F -e 'name = "gotest_qualification"' "$build" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:gotest_qualification' "$ci"; then
  ok
else
  bad "tools/ci/BUILD.bazel or ci.yml lost the gotest_qualification wiring (want target plus dogfood-freshness)"
fi

# Live proof: the wrapper go_test executes green on the seed host.
if bazel test //go/tests/fixtures/hello:hello_test --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "gotest live proof failed (want //go/tests/fixtures/hello:hello_test green)"
fi

dx_test_summary "gotest qualification harness"
