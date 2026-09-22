#!/usr/bin/env bash
# Hello-smoke-as-test qualification harness.
#
# Qualifies the as-built hello smoke promotion with fixture evidence and
# owned gaps, without claiming Supported:
# - delivered: every binary-capable hello fixture carries a `hello_output_test`
#   `sh_test` that runs its seed binary and asserts the greeting under
#   `bazel test //...` (cc plus csharp plus fsharp plus go plus java plus
#   javascript plus kotlin plus python plus rust plus scala plus typescript;
#   component-only astro/svelte/vue/mdx have no binary by design, so no
#   output smoke there);
# - wiring: each smoke is `no-coverage` process-spawning Linux-only with the
#   runfiles-first `//tools/sh:lib` bootstrap plus `dx_realpath`, skipped
#   only under `bazel coverage` via the `-no-coverage` preset with the skip
#   proved by `//tools/ci:target_tags`;
# - open owned gaps: platform plus consumer plus release evidence, exact
#   per-language greeting pins beyond the seed host, no Supported claim.
#
# Versioned here, run by CI via `bazel run //tools/ci:hello_smoke_qualification`,
# following //tools/ci:cli_contract_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

dx_bash_pin

ci=".github/workflows/ci.yml"
build="tools/ci/BUILD.bazel"

# Planned work lives in GitHub issues only (docs/roadmap.md removed under #981).
if [[ ! -f "docs/roadmap.md" ]]; then
  ok
else
  bad "docs/roadmap.md still exists (planned work lives in GitHub issues only, #981)"
fi

# BUILD owns the harness target.
if grep -q -F -e 'name = "hello_smoke_qualification"' "$build"; then
  ok
else
  bad "tools/ci/BUILD.bazel lost the hello_smoke_qualification target"
fi

# CI wires the harness in dogfood-freshness.
if grep -q -F -e 'bazel run --noshow_progress //tools/ci:hello_smoke_qualification' "$ci"; then
  ok
else
  bad "ci.yml lost the hello_smoke_qualification step (want dogfood-freshness)"
fi

# Every binary-capable hello fixture carries hello_output_test.
smoke_missing=""
for pkg in cc csharp fsharp go java javascript kotlin python rust scala typescript; do
  if ! grep -q -F -e 'name = "hello_output_test"' "$pkg/tests/fixtures/hello/BUILD.bazel"; then
    smoke_missing="$smoke_missing $pkg:missing"
  fi
done
if [[ -z "$smoke_missing" ]]; then
  ok
else
  bad "hello_output_test missing in:$smoke_missing (want all 11 binary hellos)"
fi

# Every smoke carries the no-coverage Linux-only runfiles wiring.
wiring_missing=""
for pkg in cc csharp fsharp go java javascript kotlin python rust scala typescript; do
  f="$pkg/tests/fixtures/hello/BUILD.bazel"
  if ! grep -q -F -e '$(rootpath :hello)' "$f" ||
    ! grep -q -F -e '"//tools/sh:lib"' "$f" ||
    ! grep -q -F -e 'tags = ["no-coverage"]' "$f" ||
    ! grep -q -F -e 'target_compatible_with = ["@platforms//os:linux"]' "$f"; then
    wiring_missing="$wiring_missing $pkg:wiring"
  fi
done
if [[ -z "$wiring_missing" ]]; then
  ok
else
  bad "hello smoke wiring broke in:$wiring_missing (want rootpath plus sh lib plus no-coverage plus linux-only)"
fi

# Every smoke script carries the bash bootstrap plus dx_realpath.
script_missing=""
for pkg in cc csharp fsharp go java javascript kotlin python rust scala typescript; do
  f="$pkg/tests/fixtures/hello/hello_output_test.sh"
  if [[ ! -f "$f" ]] ||
    ! grep -q -F -e '#!/usr/bin/env bash' "$f" ||
    ! grep -q -F -e 'set -euo pipefail' "$f" ||
    ! grep -q -F -e 'RUNFILES_DIR' "$f" ||
    ! grep -q -F -e 'BASH_SOURCE' "$f" ||
    ! grep -q -F -e 'dx_realpath' "$f"; then
    script_missing="$script_missing $pkg:script"
  fi
done
if [[ -z "$script_missing" ]]; then
  ok
else
  bad "hello smoke script broke in:$script_missing (want bash plus bootstrap plus dx_realpath)"
fi

# Expected greetings stay pinned per fixture shape.
if grep -q -F -e '"hello world"' go/tests/fixtures/hello/hello_output_test.sh &&
  grep -q -F -e '"hello world"' java/tests/fixtures/hello/hello_output_test.sh &&
  grep -q -F -e '"hello world"' csharp/tests/fixtures/hello/hello_output_test.sh &&
  grep -q -F -e '"hello world"' fsharp/tests/fixtures/hello/hello_output_test.sh &&
  grep -q -F -e '"hello world"' kotlin/tests/fixtures/hello/hello_output_test.sh &&
  grep -q -F -e '"hello world"' scala/tests/fixtures/hello/hello_output_test.sh &&
  grep -q -F -e '"hello world"' javascript/tests/fixtures/hello/hello_output_test.sh &&
  grep -q -F -e '"hello world"' typescript/tests/fixtures/hello/hello_output_test.sh &&
  grep -q -F -e '"Hello, world!"' python/tests/fixtures/hello/hello_output_test.sh &&
  grep -q -F -e '"Hello, world!"' rust/tests/fixtures/hello/hello_output_test.sh &&
  grep -q -F -e '"hello 42"' cc/tests/fixtures/hello/hello_output_test.sh; then
  ok
else
  bad "hello smoke greetings drifted (want hello world x8 plus Hello, world! x2 plus hello 42 cc)"
fi

# Every binary-capable hello owns its seed binary.
binary_missing=""
for pkg in cc csharp fsharp go java javascript kotlin python rust scala typescript; do
  if ! grep -q -F -e 'name = "hello",' "$pkg/tests/fixtures/hello/BUILD.bazel"; then
    binary_missing="$binary_missing $pkg:binary"
  fi
done
if [[ -z "$binary_missing" ]]; then
  ok
else
  bad "hello binary missing in:$binary_missing (want :hello in all 11)"
fi

# Every binary-capable hello owns its unit test alongside the smoke.
unit_missing=""
for pkg in cc csharp fsharp go java javascript kotlin python rust scala typescript; do
  if ! grep -q -F -e 'name = "hello_test",' "$pkg/tests/fixtures/hello/BUILD.bazel"; then
    unit_missing="$unit_missing $pkg:unit"
  fi
done
if [[ -z "$unit_missing" ]]; then
  ok
else
  bad "hello_test missing in:$unit_missing (want unit plus smoke in all 11)"
fi

# Component-only fixtures keep hello_test with no binary and no output smoke by design.
component_bad=""
for pkg in astro svelte vue mdx; do
  f="$pkg/tests/fixtures/hello/BUILD.bazel"
  if ! grep -q -F -e 'name = "hello_test",' "$f" ||
    grep -q -F -e 'name = "hello_output_test"' "$f" ||
    grep -q -F -e 'name = "hello",' "$f"; then
    component_bad="$component_bad $pkg:component"
  fi
done
if [[ -z "$component_bad" ]]; then
  ok
else
  bad "component hello scope broke in:$component_bad (want hello_test only, no binary/output smoke)"
fi

# No-coverage cohort wiring stays: preset filter plus target_tags representative proof.
if grep -q -F -e 'coverage --test_tag_filters=-no-coverage' tools/bazelrc/preset.bazelrc &&
  grep -q -F -e 'hello_output_test' tools/ci/target_tags.sh &&
  grep -q -F -e 'no-coverage hello_output_test ran under bazel coverage' tools/ci/target_tags.sh; then
  ok
else
  bad "no-coverage wiring lost (want preset filter plus target_tags hello_output_test proof)"
fi

# CI test job runs the smokes via `bazel test //...` (no separate smoke job).
if grep -q -F -e 'bazel test --noshow_progress //...' "$ci"; then
  ok
else
  bad "ci.yml lost bazel test //... (want smokes via the test job)"
fi

# Every smoke script is listed as its sh_test srcs (shell ownership via deps).
srcs_missing=""
for pkg in cc csharp fsharp go java javascript kotlin python rust scala typescript; do
  if ! grep -q -F -e 'hello_output_test.sh' "$pkg/tests/fixtures/hello/BUILD.bazel"; then
    srcs_missing="$srcs_missing $pkg:srcs"
  fi
done
if [[ -z "$srcs_missing" ]]; then
  ok
else
  bad "hello smoke srcs missing in:$srcs_missing (want hello_output_test.sh in BUILD)"
fi

dx_test_summary "hello smoke qualification harness"
