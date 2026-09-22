#!/usr/bin/env bash
# Examples laziness runtime proof (slice 5, final): managed-acquisition
# action-command attribution via `bazel aquery` over per-foundation adopt-*
# examples.
#
# The static harness (examples_laziness.sh) proves no installer commands in
# tool-implementation source. The query harness proves deps-closure isolation
# and the aquery harness proves action-graph isolation for unused foundations.
# This harness closes the acquisition/laziness proof on the seed host: every
# declared build action's command line, mnemonic, inputs, and environment over
# each adopt-* consumer must contain none of the prohibited installer commands,
# so the action graph never shells out to an ecosystem installer.
#
# No-install attribution, runtime half: prohibited installer commands must not
# appear in declared action commands. Runtime attribution over aquery action
# commands plus the static source attribution together prove the private tool
# graph never invokes installers.
#
# Covered here (same breadth as slices 2-4): Rust, Python, JS/TS, Go, C++,
# Java, Kotlin, Scala, C#, F#, plus the polyglot composition (no-install holds
# for every consumer; isolation zero-work is proven by the query/aquery slices
# and does not apply to the multi-foundation consumer).
#
# Seed-host scope: Delivered means implemented and verified on the Linux x86_64
# seed host only (platform qualification open). Remote-cache /
# remote-execution and empty-cache download attribution stay owned by issue
# and platform evidence; they are not claimed here. A warm
# local execution log alone is not a cache test, so this harness asserts over
# declared actions (cache-independent), not over executed-vs-cached logs.
#
# Versioned here, run by CI via `bazel run //tools/ci:examples_laziness_runtime`,
# after //tools/ci:examples_laziness_aquery.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

# Same prohibited-installer set as the static half (examples_laziness.sh):
# the contract's "or equivalent installer" clause and the
# laziness matrix (pip, uv, npm, pnpm, Cargo, Maven, NuGet, Bundler,
# PowerShell Gallery).
check_no_installers() { # example
  local example="$1"
  local actions
  if ! actions="$(bazel aquery "//examples/$example/..." --noshow_progress 2>/dev/null)"; then
    bad "$example: bazel aquery failed"
    return
  fi
  if [[ -z "$actions" ]]; then
    bad "$example: empty aquery output"
    return
  fi
  local hits
  hits="$(echo "$actions" | grep -F -e 'pip install' -e 'npm install' -e 'cargo install' -e 'dotnet tool install' -e 'pnpm install' -e 'pnpm add' -e 'uv pip install' -e 'go install' -e 'dotnet add' -e 'bundle install' -e 'nuget install' -e 'Install-Module' -e 'mvn install' -e 'uv add' -e 'cargo add' -e 'gem install' -e 'dotnet restore' -e 'nuget restore' -e 'npm ci' -e 'yarn install' -e 'yarn add' -e 'pipx install' -e 'go get' -e 'dotnet tool restore' -e 'poetry install' -e 'poetry add' -e 'pipenv install' -e 'bun install' -e 'Install-Script' -e 'pip3 install' -e 'uv tool install' -e 'npm add' -e 'bun add' -e 'bundle add' -e 'Install-Package' -e 'pipenv sync' -e 'poetry sync' -e 'uv sync' -e 'uv pip sync' -e 'dotnet tool update' -e 'Install-PSResource' -e 'pnpm dlx' -e 'npx' -e 'cargo update' -e 'npm update' -e 'uv lock' -e 'poetry lock' -e 'pipenv lock' -e 'poetry update' -e 'uv pip compile' -e 'pip compile' -e 'bundle update' -e 'gem update' -e 'pipenv update' -e 'poetry export' -e 'yarn upgrade' -e 'pnpm upgrade' -e 'bun update' -e 'uv export' -e 'yarn dlx' -e 'bunx' -e 'uvx' -e 'go mod download' -e 'cargo fetch' -e 'Update-Module' -e 'pip download' -e 'npm exec' -e 'yarn exec' -e 'pnpm exec' -e 'bun x' -e 'cargo binstall' -e 'mvnw install' -e 'bundler install' -e 'Save-Module' -e 'go mod tidy' -e 'bundle lock' -e 'pip wheel' -e 'cargo upgrade' -e 'pnpm update' -e 'Update-Script' -e 'Save-Script' -e 'bundle exec' -e 'bundle pristine' || true)"
  if [[ -z "$hits" ]]; then
    ok
  else
    bad "$example: prohibited installer in declared actions: $(echo "$hits" | head -n 3)"
  fi
}

check_no_installers adopt-rust
check_no_installers adopt-python
check_no_installers adopt-js-ts
check_no_installers adopt-go
check_no_installers adopt-cpp
check_no_installers adopt-java
check_no_installers adopt-kotlin
check_no_installers adopt-scala
check_no_installers adopt-csharp
check_no_installers adopt-fsharp
check_no_installers adopt-ruby
check_no_installers adopt-polyglot

dx_test_summary "examples laziness runtime"
