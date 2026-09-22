#!/usr/bin/env bash
# Paket lock wiring qualification harness.
#
# Qualifies the owned gap from support-matrix provisional dependency locks:
# Paket (`paket.dependencies` plus `paket.lock`) via `paket2bazel`, closed
# owner only. pinned the xUnit v3 runner closure, not the lock
# wiring itself.
# - wired: `third_party/dotnet/paket.dependencies` (FSharp.Core 10.1.201 plus
#   xunit.v3 4.0.0 plus xunit.analyzers 2.0.0, source nuget.org, framework
#   net10.0) plus `paket.lock` (MTP/Microsoft.Extensions transitive closure)
#   via `paket2bazel` into `third_party/dotnet/deps/paket.main.bzl` carrying
#   per-package sha512, loaded from MODULE.bazel (`paket.main` hub via
#   `paket.main_extension.bzl`). Single shared lock for C# plus F#; pins
#   recorded in `csharp/tests/fixtures/paket/pins.bzl` (F# references it, no
#   duplicate). Every entry carries sha512 so the Bazel downloader verifies
#   each artifact; stale locks fail closed.
# - fixtures: F# hello consumes `@paket.main//fsharp.core`, xUnit fixtures
#   consume the pinned hub labels, depcheck testdata proves offline
#   lockfile-consistency plus usage authority (`paket.lock` via paket2bazel).
# - generation consumes never writes: the common generation contract owns
#   the never-writes rule; Gazelle never reads or writes the Paket files
#   (handwritten BUILD deps name `@paket.main` labels directly).
# - rejected: NuGet native `packages.lock.json` (one file per project, hashes
#   incompatible with Bazel's downloader per rules_dotnet issue 444).
# - scope: lock only. Per-platform SDK acquisition, platform plus consumer
#   plus release evidence stay open; no Supported claim.
#
# Versioned here, run by CI via `bazel run //tools/ci:paket_qualification`,
# following //tools/ci:scalatest_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

dx_bash_pin

pins="csharp/tests/fixtures/paket/pins.bzl"
pins_build="csharp/tests/fixtures/paket/BUILD.bazel"
deps="third_party/dotnet/paket.dependencies"
lock="third_party/dotnet/paket.lock"
hub="third_party/dotnet/deps/paket.main.bzl"
ext="third_party/dotnet/deps/paket.main_extension.bzl"
deps_build="third_party/dotnet/BUILD.bazel"
hub_build="third_party/dotnet/deps/BUILD.bazel"
module="MODULE.bazel"
fs_hello="fsharp/tests/fixtures/hello/BUILD.bazel"
cs_xunit="csharp/tests/fixtures/xunit/BUILD.bazel"
fs_xunit="fsharp/tests/fixtures/xunit/BUILD.bazel"
common="docs/generation/common.md"
matrix="docs/product/support-matrix.md"
gen_readme="docs/generation/foundation-qualification.md"
build="tools/ci/BUILD.bazel"
ci=".github/workflows/ci.yml"
checker="tools/depcheck/src/lib.rs"

# Pins fixture stays present as the single shared owner.
if [[ -f "$pins" && -f "$pins_build" ]]; then
  ok
else
  bad "paket pins fixture missing (want $pins plus $pins_build)"
fi

# Pins record the direct versions plus source plus framework plus hub plus rejected.
if grep -q -F -e 'FSHARP_CORE_VERSION = "10.1.201"' "$pins" &&
  grep -q -F -e 'XUNIT_V3_VERSION = "4.0.0"' "$pins" &&
  grep -q -F -e 'XUNIT_ANALYZERS_VERSION = "2.0.0"' "$pins" &&
  grep -q -F -e 'https://api.nuget.org/v3/index.json' "$pins" &&
  grep -q -F -e 'PAKET_FRAMEWORK = "net10.0"' "$pins" &&
  grep -q -F -e 'PAKET_HUB_REPO = "@paket.main"' "$pins" &&
  grep -q -F -e 'packages.lock.json rejected' "$pins" &&
  grep -q -F -e 'consumes never writes' "$pins"; then
  ok
else
  bad "pins.bzl lost its Paket direct pins plus source plus framework plus hub plus rejected under issue #482"
fi

# paket.dependencies pins the direct entries exactly.
if grep -q -F -e 'nuget FSharp.Core 10.1.201' "$deps" &&
  grep -q -F -e 'nuget xunit.v3 4.0.0' "$deps" &&
  grep -q -F -e 'nuget xunit.analyzers 2.0.0' "$deps" &&
  grep -q -F -e 'source https://api.nuget.org/v3/index.json' "$deps" &&
  grep -q -F -e 'framework: net10.0' "$deps"; then
  ok
else
  bad "paket.dependencies lost its FSharp.Core plus xUnit pins with source plus framework under issue #482"
fi

# paket.lock pins the transitive closure (FSharp.Core plus xUnit MTP).
if grep -q -F -e 'FSharp.Core (10.1.201)' "$lock" &&
  grep -q -F -e 'xunit.v3 (4.0)' "$lock" &&
  grep -q -F -e 'xunit.analyzers (2.0)' "$lock" &&
  grep -q -F -e 'xunit.v3.core.mtp-v2' "$lock" &&
  grep -q -F -e 'xunit.v3.runner.inproc.console' "$lock" &&
  grep -q -F -e 'Microsoft.Testing.Platform (' "$lock"; then
  ok
else
  bad "paket.lock lost its FSharp.Core plus xUnit MTP transitive pins under issue #482"
fi

# Generated hub carries per-package sha512 plus exact versions.
if grep -q -F -e '"name": "FSharp.Core", "id": "FSharp.Core", "version": "10.1.201"' "$hub" &&
  grep -q -F -e '"name": "xunit.v3", "id": "xunit.v3", "version": "4.0.0"' "$hub" &&
  grep -q -F -e '"name": "xunit.analyzers", "id": "xunit.analyzers", "version": "2.0.0"' "$hub" &&
  grep -q -F -e 'sha512-' "$hub"; then
  ok
else
  bad "paket.main.bzl lost its FSharp.Core plus xUnit sha512 pins under issue #482"
fi

# MODULE wires the paket.main hub with the qualified lock record.
if grep -q -F -e 'use_extension("//third_party/dotnet/deps:paket.main_extension.bzl"' "$module" &&
  grep -q -F -e 'use_repo(dotnet_nuget, "paket.main")' "$module" &&
  grep -q -F -e 'issue #482' "$module"; then
  ok
else
  bad "MODULE.bazel lost its paket.main hub wiring plus #482 qualified record"
fi

# Lock packages stay maintainer-owned with GENERATED hub output.
if grep -q -F -e 'paket.dependencies' "$deps_build" &&
  grep -q -F -e 'paket.lock' "$deps_build" &&
  grep -q -F -e 'GENERATED' "$hub" &&
  grep -q -F -e 'Generated' "$ext" &&
  grep -q -F -e 'paket.main.bzl' "$hub_build"; then
  ok
else
  bad "third_party/dotnet lost its lock exports plus GENERATED hub wiring under issue #482"
fi

# Fixtures consume the hub: F# hello needs FSharp.Core, xUnit names hub labels.
if grep -q -F -e '@paket.main//fsharp.core' "$fs_hello" &&
  grep -q -F -e '@paket.main//xunit.v3' "$cs_xunit" &&
  grep -q -F -e '@paket.main//xunit.v3' "$fs_xunit" &&
  grep -q -F -e '@paket.main//fsharp.core' "$fs_xunit"; then
  ok
else
  bad "hello/xunit fixtures lost their @paket.main hub consumption under issue #482"
fi

# Generation consumes never writes: contract owns it, Gazelle never touches Paket files
# (hermetic tree search: BSD grep lacks --include, issue #1006).
if grep -q -F -e 'or edits manifests or lockfiles' "$common" &&
  dx_tree_absent --include='*.go' 'paket.dependencies' -- gazelle/csharp gazelle/fsharp &&
  dx_tree_absent --include='*.go' 'paket.lock' -- gazelle/csharp gazelle/fsharp &&
  dx_tree_absent --include='*.go' 'paket2bazel' -- gazelle/csharp gazelle/fsharp; then
  ok
else
  bad "generation lost its consumes-never-writes proof (want common contract plus no Paket refs in gazelle/csharp plus gazelle/fsharp)"
fi

# packages.lock.json stays rejected: none lands, docs own the rules_dotnet 444 note.
if ! find . -name 'packages.lock.json' -not -path './bazel-*' 2>/dev/null | grep -q . &&
  grep -q -F -e 'packages.lock.json' "$matrix" &&
  grep -q -F -e 'rules_dotnet issue 444' "$matrix"; then
  ok
else
  bad "packages.lock.json rejection lost (want no such file plus support-matrix rules_dotnet 444 note)"
fi

# Support matrix owns the qualified lock with fixtures and harness.
if grep -q -F -e 'Paket' "$matrix" &&
  grep -q -F -e 'qualified seed-only under issue #482' "$matrix" &&
  grep -q -F -e 'paket_qualification' "$matrix" &&
  grep -q -F -e 'csharp/tests/fixtures/paket/pins.bzl' "$matrix"; then
  ok
else
  bad "support-matrix lost its #482 qualified Paket lock record with fixtures"
fi

# Generation README owns the qualified lock alongside the other gaps.
if grep -q -F -e 'qualified seed-only under issue #482' "$gen_readme" &&
  grep -q -F -e 'paket_qualification' "$gen_readme" &&
  grep -q -F -e 'paket.lock' "$gen_readme"; then
  ok
else
  bad "generation README lost its #482 qualified Paket lock record"
fi

# BUILD owns the harness target plus CI wires it in dogfood-freshness.
if grep -q -F -e 'name = "paket_qualification"' "$build" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:paket_qualification' "$ci"; then
  ok
else
  bad "tools/ci/BUILD.bazel or ci.yml lost the paket_qualification wiring (want target plus dogfood-freshness)"
fi

# Depcheck proves offline lock authority with paket fixtures plus parser.
if [[ -f "tools/depcheck/testdata/csharp/ok_used/paket.dependencies" &&
  -f "tools/depcheck/testdata/csharp/ok_used/paket.lock" &&
  -f "tools/depcheck/testdata/fsharp/ok_used/paket.dependencies" &&
  -f "tools/depcheck/testdata/fsharp/ok_used/paket.lock" ]] &&
  grep -q -F -e 'paket.dependencies' "$checker" &&
  grep -q -F -e 'paket.lock' "$checker"; then
  ok
else
  bad "depcheck lost its csharp/fsharp paket lock authority fixtures plus parser under issue #482"
fi

# Live proof: the pinned hub resolves on the seed host.
if bazel query @paket.main//xunit.v3 --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "paket live proof failed (want @paket.main//xunit.v3 resolvable)"
fi

dx_test_summary "paket qualification harness"
