#!/usr/bin/env bash
# Examples laziness query proof (issue #85, slice 3): unused-foundation
# zero-work via `bazel query deps` over single-foundation adopt-* examples.
#
# The static harness (examples_laziness.sh) proves no installer commands
# in tool code plus per-foundation wrapper isolation. This harness proves
# the dependency-closure half: each single-foundation example's
# transitive closure contains its own ecosystem repo and none of the
# other foundations' ecosystem repos, so unused foundations contribute
# no targets/repos to that consumer's build.
#
# Covered here (minimum per #85 plus Go + C# + Kotlin + Scala + F# + C++/Java
# follow-ups): Rust, Python, JS/TS, Go, C#, Kotlin, Scala, F# (F# shares
# rules_dotnet with C#), plus C++ and Java negative-only isolation (none of
# the seven tracked ecosystem repos leak into their closures; no positive
# ownership marker is asserted for them -- rules_cc also appears in the
# Kotlin query closure and rules_java appears in every closure, so both are
# base-toolchain-like and intentionally not asserted here, exactly like
# bazel_tools/skylib/platforms). Adopt-polyglot stays out of scope by design
# (multi-foundation consumer, zero-work proof does not apply).
# Markers are
# ecosystem-specific Bazel repos, not shared base toolchains
# (bazel_tools/skylib/platforms/rules_java appear across closures and
# are intentionally not asserted here; rules_cc likewise spans C++/Kotlin
# query closures and is not asserted as an ownership marker).
#
# Still delivered per #85 together with the runtime action-command proof
# (examples_laziness_runtime.sh): query-closure plus action-graph plus
# action-command attribution complete the seed-host proof. Remote-cache /
# empty-cache download attribution stays owned by #308 and platform
# evidence by #298. This harness is query-closure only.
#
# Run by CI via `bazel run //tools/ci:examples_laziness_query`,
# after //tools/ci:examples_laziness.
set -euo pipefail

# Shared workspace + runfiles helpers (issue #319).
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

dx_cd_workspace

dx_test_init

check_example() { # example, want-marker, forbidden-markers...
  local example="$1" want="$2"
  shift 2
  local deps
  if ! deps="$(bazel query "deps(//examples/$example/...)" --noshow_progress 2>/dev/null)"; then
    bad "$example: bazel query failed"
    return
  fi
  if [[ "$deps" == *"$want"* ]]; then
    ok
  else
    bad "$example: want marker [$want] in deps closure"
  fi
  local marker
  for marker in "$@"; do
    if [[ "$deps" == *"$marker"* ]]; then
      bad "$example: forbidden marker [$marker] in deps closure (unused foundation leaks)"
    else
      ok
    fi
  done
}

check_negative() { # example, forbidden-markers...
  local example="$1"
  shift
  local deps
  if ! deps="$(bazel query "deps(//examples/$example/...)" --noshow_progress 2>/dev/null)"; then
    bad "$example: bazel query failed"
    return
  fi
  if [[ -z "$deps" ]]; then
    bad "$example: empty deps closure"
    return
  fi
  local marker
  for marker in "$@"; do
    if [[ "$deps" == *"$marker"* ]]; then
      bad "$example: forbidden marker [$marker] in deps closure (unused foundation leaks)"
    else
      ok
    fi
  done
}

# Rust: owns rules_rust; Python/JS/Go/DotNet/Kotlin/Scala contribute nothing.
check_example adopt-rust "rules_rust" "aspect_rules_py" "aspect_rules_js" "rules_go" "rules_dotnet" "rules_kotlin" "rules_scala"
# Python: owns aspect_rules_py; Rust/JS/Go/DotNet/Kotlin/Scala contribute nothing.
check_example adopt-python "aspect_rules_py" "rules_rust" "aspect_rules_js" "rules_go" "rules_dotnet" "rules_kotlin" "rules_scala"
# JS/TS: owns aspect_rules_js; Rust/Python/Go/DotNet/Kotlin/Scala contribute nothing.
check_example adopt-js-ts "aspect_rules_js" "rules_rust" "aspect_rules_py" "rules_go" "rules_dotnet" "rules_kotlin" "rules_scala"
# Go: owns rules_go; Rust/Python/JS/DotNet/Kotlin/Scala contribute nothing.
check_example adopt-go "rules_go" "rules_rust" "aspect_rules_py" "aspect_rules_js" "rules_dotnet" "rules_kotlin" "rules_scala"
# C#: owns rules_dotnet; Rust/Python/JS/Go/Kotlin/Scala contribute nothing.
check_example adopt-csharp "rules_dotnet" "rules_rust" "aspect_rules_py" "aspect_rules_js" "rules_go" "rules_kotlin" "rules_scala"
# Kotlin: owns rules_kotlin; Rust/Python/JS/Go/DotNet/Scala contribute nothing.
check_example adopt-kotlin "rules_kotlin" "rules_rust" "aspect_rules_py" "aspect_rules_js" "rules_go" "rules_dotnet" "rules_scala"
# Scala: owns rules_scala; Rust/Python/JS/Go/DotNet/Kotlin contribute nothing.
check_example adopt-scala "rules_scala" "rules_rust" "aspect_rules_py" "aspect_rules_js" "rules_go" "rules_dotnet" "rules_kotlin"
# F#: owns rules_dotnet (shared with C#); Rust/Python/JS/Go/Kotlin/Scala contribute nothing.
check_example adopt-fsharp "rules_dotnet" "rules_rust" "aspect_rules_py" "aspect_rules_js" "rules_go" "rules_kotlin" "rules_scala"
# C++: negative-only isolation; none of the seven tracked ecosystem repos
# contribute targets (no positive marker: rules_cc also appears in the Kotlin
# closure and Rust-adjacent toolchains treat CC as base, so it proves no
# ownership here; rules_java is base everywhere).
check_negative adopt-cpp "rules_rust" "aspect_rules_py" "aspect_rules_js" "rules_go" "rules_dotnet" "rules_kotlin" "rules_scala"
# Java: negative-only isolation; none of the seven tracked ecosystem repos
# contribute targets (no positive marker: the Java example pulls no @maven
# closure and rules_java is base across all closures).
check_negative adopt-java "rules_rust" "aspect_rules_py" "aspect_rules_js" "rules_go" "rules_dotnet" "rules_kotlin" "rules_scala"

dx_test_summary "examples laziness query"
