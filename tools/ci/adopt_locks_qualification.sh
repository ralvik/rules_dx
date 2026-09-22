#!/usr/bin/env bash
# Per-adopt consumer lock authority qualification harness (issue #1077).
#
# Each `examples/adopt-*` foreign tree records one pinned module dep with
# its manifest plus lock authority in its README. Depcheck-native arrival
# pairs (go, ruby, rust, js, python) prove foreign consistency offline via
# `bazel test //tools/depcheck:adopt_locks_test` (stale plus missing fail
# closed there); this harness proves the rest on a clean tree:
# - every per-adopt arrival lock file exists (go.sum, Gemfile.lock,
#   Cargo.lock, pnpm-lock.yaml, uv.lock, including the polyglot pairs);
# - every shared-lock consumer manifest pins the version the shared lock
#   resolves (JVM guava 32.0.1-jre, .NET xunit 4.0 plus FSharp.Core
#   10.1.201, Gallery Pester 5.7.1), so a drifted manifest fails here
#   until the consumer repins with its owning command;
# - every adopt README records its manifest plus lock authority;
# - the repin table owns the per-adopt repin commands.
# - open owned gaps: platform plus release evidence, no `Supported` claim.
#
# Versioned here, run by CI via `bazel run //tools/ci:adopt_locks_qualification`,
# following //tools/ci:godeps_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

dx_bash_pin

lock_table="docs/tools/tool-acquisition.md"
build="tools/ci/BUILD.bazel"
ci=".github/workflows/ci.yml"

# Per-adopt arrival locks stay present (depcheck-native pairs).
for pair in "examples/adopt-go/go.mod examples/adopt-go/go.sum" \
  "examples/adopt-ruby/Gemfile examples/adopt-ruby/Gemfile.lock" \
  "examples/adopt-rust/Cargo.toml examples/adopt-rust/Cargo.lock" \
  "examples/adopt-js-ts/package.json examples/adopt-js-ts/pnpm-lock.yaml" \
  "examples/adopt-python/pyproject.toml examples/adopt-python/uv.lock" \
  "examples/adopt-polyglot/package.json examples/adopt-polyglot/pnpm-lock.yaml" \
  "examples/adopt-polyglot/pyproject.toml examples/adopt-polyglot/uv.lock" \
  "examples/adopt-polyglot/Cargo.toml examples/adopt-polyglot/Cargo.lock"; do
  set -- $pair
  if [[ -f "$1" && -f "$2" ]]; then
    ok "arrival pair present ($1 plus $2)"
  else
    bad "arrival pair missing (want $1 plus $2)"
  fi
done

# Arrival locks record the foreign pinned versions (depcheck proves the
# manifest-vs-lock match; here the pin presence keeps the proof honest).
if grep -q -F -e 'github.com/google/go-cmp v0.6.0' examples/adopt-go/go.sum &&
  grep -q -F -e 'rspec (3.13.0)' examples/adopt-ruby/Gemfile.lock &&
  grep -q -F -e 'jest@30.2.0' examples/adopt-js-ts/pnpm-lock.yaml &&
  grep -q -F -e 'fsevents@2.3.3' examples/adopt-js-ts/pnpm-lock.yaml &&
  grep -q -F -e 'name = "pytest"' examples/adopt-python/uv.lock &&
  grep -q -F -e 'name = "httpx"' examples/adopt-python/uv.lock; then
  ok
else
  bad "arrival locks lost their foreign pins (go-cmp, rspec, jest, fsevents, pytest, httpx)"
fi

# JVM consumers pin the guava version the shared Maven lock resolves.
for pom in examples/adopt-java/pom.xml examples/adopt-kotlin/pom.xml; do
  if grep -q -F -e '<version>32.0.1-jre</version>' "$pom"; then
    ok "$pom pins guava 32.0.1-jre"
  else
    bad "$pom lost its guava 32.0.1-jre pin"
  fi
done
if grep -q -F -e '"com.google.guava" % "guava" % "32.0.1-jre"' examples/adopt-scala/build.sbt; then
  ok "adopt-scala pins guava 32.0.1-jre"
else
  bad "adopt-scala lost its guava 32.0.1-jre pin"
fi
if grep -q -F -e '"version": "32.0.1-jre"' third_party/jvm/maven_install.json &&
  grep -q -F -e '"com.google.guava:guava"' third_party/jvm/maven_install.json; then
  ok "shared Maven lock resolves guava 32.0.1-jre"
else
  bad "shared Maven lock lost guava 32.0.1-jre"
fi

# .NET consumers pin the versions the shared Paket lock resolves.
if grep -q -F -e 'xunit.v3.assert" Version="4.0.0"' examples/adopt-csharp/adopt-csharp.csproj; then
  ok "adopt-csharp pins xunit.v3.assert 4.0.0"
else
  bad "adopt-csharp lost its xunit.v3.assert 4.0.0 pin"
fi
if grep -q -F -e 'FSharp.Core" Version="10.1.201"' examples/adopt-fsharp/adopt-fsharp.fsproj; then
  ok "adopt-fsharp pins FSharp.Core 10.1.201"
else
  bad "adopt-fsharp lost its FSharp.Core 10.1.201 pin"
fi
if grep -q -F -e 'xunit.v3.assert (4.0)' third_party/dotnet/paket.lock &&
  grep -q -F -e 'FSharp.Core (10.1.201)' third_party/dotnet/paket.lock; then
  ok "shared Paket lock resolves xunit 4.0 plus FSharp.Core 10.1.201"
else
  bad "shared Paket lock lost xunit 4.0 plus FSharp.Core 10.1.201"
fi

# PowerShell consumer pins the Gallery version the shared lock resolves.
if grep -q -F -e "Pester = '5.7.1'" examples/adopt-powershell/PSGallery.requirements.psd1 &&
  grep -q -F -e '"version": "5.7.1"' third_party/powershell/PSGallery.lock.json; then
  ok "adopt-powershell pins Pester 5.7.1 over the shared Gallery lock"
else
  bad "adopt-powershell lost its Pester 5.7.1 pin over the shared Gallery lock"
fi

# Adopt READMEs record their manifest plus lock authority.
for doc in adopt-go adopt-java adopt-kotlin adopt-scala adopt-csharp adopt-fsharp adopt-ruby adopt-powershell adopt-js-ts adopt-python adopt-polyglot adopt-rust adopt-cpp; do
  if grep -q -E -e 'lock|maven_install|paket|go\.sum|Gemfile\.lock|Cargo\.lock|pnpm-lock|uv\.lock|MODULE\.bazel\.lock' "examples/$doc/README.md"; then
    ok "$doc README records lock authority"
  else
    bad "$doc README lost its lock authority record"
  fi
done

# Repin table owns the per-adopt repin commands.
if grep -q -F -e 'examples/adopt-js-ts' "$lock_table" &&
  grep -q -F -e 'examples/adopt-python' "$lock_table" &&
  grep -q -F -e 'pnpm --dir examples/adopt-js-ts install --lockfile-only' "$lock_table" &&
  grep -q -F -e 'uv lock --directory examples/adopt-python' "$lock_table"; then
  ok
else
  bad "tool-acquisition repin table lost its per-adopt repin commands"
fi

# Negative control: a drifted consumer pin must be detectable (the realistic
# regression: adopt-java floats guava). The check above keys on the exact
# 32.0.1-jre string, so a mutated pom without it must fail the match.
dx_mkscratch scratch
sed 's|<version>32.0.1-jre</version>|<version>99.0-jre</version>|' examples/adopt-java/pom.xml >"$scratch/pom.xml"
if grep -q -F -e '<version>32.0.1-jre</version>' "$scratch/pom.xml"; then
  bad "negative control setup broken (mutated pom still matches)"
else
  ok "drifted consumer pin fails closed"
fi

dx_test_summary "adopt locks qualification"
