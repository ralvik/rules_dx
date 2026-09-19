#!/usr/bin/env bash
# Examples laziness aquery proof (issue #85, slice 4): unused-foundation
# zero-work via `bazel aquery` action graph over single-foundation adopt-*
# examples.
#
# The query harness (examples_laziness_query.sh) proves the
# dependency-closure half via `bazel query deps`. This harness proves the
# action-graph half: each single-foundation example's declared actions
# mention its own ecosystem repo and none of the other foundations'
# ecosystem repos, so unused foundations contribute zero actions to that
# consumer's build.
#
# Covered here (minimum per #85 plus Go + C# + Kotlin + Scala + F# + C++/Java
# follow-ups): Rust, Python, JS/TS, Go, C#, Kotlin, Scala, F# (F# shares
# rules_dotnet with C#), plus C++ and Java negative-only isolation (none of
# the seven tracked ecosystem repos leak into their action graphs; no positive
# ownership marker is asserted -- rules_cc appears in Rust/C++/Kotlin action
# graphs as base CC toolchain and rules_java appears in Java/Kotlin/Scala
# action graphs as base JDK toolchain, so neither proves ownership here).
# Adopt-polyglot stays out of scope by design (multi-foundation consumer,
# zero-work proof does not apply).
# Markers are
# ecosystem-specific repo strings as they appear in aquery output
# (rules_rust / aspect_rules_py / aspect_rules_js / rules_go /
# rules_dotnet / rules_kotlin / rules_scala), not shared base toolchains.
#
# Delivered per #85 together with the runtime action-command proof
# (examples_laziness_runtime.sh): dependency closure plus action graph
# plus action commands complete the seed-host proof. Network-denied
# execution plus empty-cache remote-cache proof stay owned by #308
# and platform evidence by #298. This harness is
# action-graph only, no execution.
#
# Run by CI via `bazel run //tools/ci:examples_laziness_aquery`,
# after //tools/ci:examples_laziness_query.
set -euo pipefail

# Shared workspace + runfiles helpers (issue #319).
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

dx_cd_workspace

dx_test_init

check_example() { # example, want-marker, forbidden-markers...
  local example="$1" want="$2"
  shift 2
  local actions
  if ! actions="$(bazel aquery "//examples/$example/..." --noshow_progress 2>/dev/null)"; then
    bad "$example: bazel aquery failed"
    return
  fi
  if [[ -z "$actions" ]]; then
    bad "$example: empty aquery output"
    return
  fi
  if [[ "$actions" == *"$want"* ]]; then
    ok
  else
    bad "$example: want marker [$want] in aquery actions"
  fi
  local marker
  for marker in "$@"; do
    if [[ "$actions" == *"$marker"* ]]; then
      bad "$example: forbidden marker [$marker] in aquery actions (unused foundation leaks)"
    else
      ok
    fi
  done
}

check_negative() { # example, forbidden-markers...
  local example="$1"
  shift
  local actions
  if ! actions="$(bazel aquery "//examples/$example/..." --noshow_progress 2>/dev/null)"; then
    bad "$example: bazel aquery failed"
    return
  fi
  if [[ -z "$actions" ]]; then
    bad "$example: empty aquery output"
    return
  fi
  local marker
  for marker in "$@"; do
    if [[ "$actions" == *"$marker"* ]]; then
      bad "$example: forbidden marker [$marker] in aquery actions (unused foundation leaks)"
    else
      ok
    fi
  done
}

# Rust: owns rules_rust; Python/JS/Go/DotNet/Kotlin/Scala contribute no actions.
check_example adopt-rust "rules_rust" "aspect_rules_py" "aspect_rules_js" "rules_go" "rules_dotnet" "rules_kotlin" "rules_scala"
# Python: owns aspect_rules_py; Rust/JS/Go/DotNet/Kotlin/Scala contribute no actions.
check_example adopt-python "aspect_rules_py" "rules_rust" "aspect_rules_js" "rules_go" "rules_dotnet" "rules_kotlin" "rules_scala"
# JS/TS: owns aspect_rules_js; Rust/Python/Go/DotNet/Kotlin/Scala contribute no actions.
check_example adopt-js-ts "aspect_rules_js" "rules_rust" "aspect_rules_py" "rules_go" "rules_dotnet" "rules_kotlin" "rules_scala"
# Go: owns rules_go; Rust/Python/JS/DotNet/Kotlin/Scala contribute no actions.
check_example adopt-go "rules_go" "rules_rust" "aspect_rules_py" "aspect_rules_js" "rules_dotnet" "rules_kotlin" "rules_scala"
# C#: owns rules_dotnet; Rust/Python/JS/Go/Kotlin/Scala contribute no actions.
check_example adopt-csharp "rules_dotnet" "rules_rust" "aspect_rules_py" "aspect_rules_js" "rules_go" "rules_kotlin" "rules_scala"
# Kotlin: owns rules_kotlin; Rust/Python/JS/Go/DotNet/Scala contribute no actions.
check_example adopt-kotlin "rules_kotlin" "rules_rust" "aspect_rules_py" "aspect_rules_js" "rules_go" "rules_dotnet" "rules_scala"
# Scala: owns rules_scala; Rust/Python/JS/Go/DotNet/Kotlin contribute no actions.
check_example adopt-scala "rules_scala" "rules_rust" "aspect_rules_py" "aspect_rules_js" "rules_go" "rules_dotnet" "rules_kotlin"
# F#: owns rules_dotnet (shared with C#); Rust/Python/JS/Go/Kotlin/Scala contribute no actions.
check_example adopt-fsharp "rules_dotnet" "rules_rust" "aspect_rules_py" "aspect_rules_js" "rules_go" "rules_kotlin" "rules_scala"
# C++: negative-only isolation; none of the seven tracked ecosystem repos
# contribute actions (no positive marker: rules_cc is base CC toolchain across
# Rust/C++/Kotlin action graphs, so it proves no ownership here).
check_negative adopt-cpp "rules_rust" "aspect_rules_py" "aspect_rules_js" "rules_go" "rules_dotnet" "rules_kotlin" "rules_scala"
# Java: negative-only isolation; none of the seven tracked ecosystem repos
# contribute actions (no positive marker: rules_java is base JDK toolchain
# across Java/Kotlin/Scala action graphs).
check_negative adopt-java "rules_rust" "aspect_rules_py" "aspect_rules_js" "rules_go" "rules_dotnet" "rules_kotlin" "rules_scala"

dx_test_summary "examples laziness aquery"
