#!/usr/bin/env bash
# Cargo metadata qualification harness.
#
# Defines plus proves the Cargo-metadata slice of the native baseline with
# fixture evidence, without claiming qualified floors, qualified cross
# routes, or Supported:
# - features: a nonempty Cargo target `required-features` list stays
#   unsupported until upstream exposes exact first-party target
#   admissibility; generation reports the target, features, and manifest
#   and points to a kept handwritten `testonly` target with explicit
#   `crate_features`; it does not enable features or emit the target
#   unconditionally; no private `rules_rs` data dependency and no
#   project-owned Cargo feature resolver.
# - build-script metadata: conventional `build.rs`, explicit
#   `package.build` path, and `package.build = false` are authoritative;
#   `[package] version` feeds the generated build-script `version` plus
#   single-version path checks; the script sees only `[build-dependencies]`;
#   hermetic defaults (`use_cc_toolchain = True`,
#   `use_default_shell_env = False`, `emit_warnings = True`); the script
#   runs later as a Bazel execution action, never during generation;
#   generation does not recreate Cargo's protocol; `BuildScriptInfo`
#   outputs stay the narrow upstream export.
# - target kinds: ordinary libraries, binaries, tests, proc macros, sole
#   `cdylib`, and sole `staticlib` generate only through the stable
#   fixture-proven public upstream mapping; `dylib` plus multiple crate
#   types plus unknown kinds fail with a kept handwritten-target route;
#   no kind coercion and no duplicate root ownership; examples plus
#   benches generate only as explicitly declared ordinary-binary targets
#   with affixed names.
# - ownership: the authoritative crate root owns its root and every
#   recursively loaded module; integration-test roots separately own
#   their trees; one source has only one owner; bins plus tests plus
#   examples plus benches link the same-package library automatically;
#   declared first-party path dependencies mirror without detection
#   evidence; library flavors stay public while bins plus tests plus
#   scripts stay private.
# - public shape: checked-in Cargo declarations parsed in the first-party
#   extension, path crates resolved through Gazelle's local rule index,
#   exact externals emitted through the public `@crates//:crates.bzl`
#   `crate_deps` and `aliases` macros with `package_name` as the parent
#   Bazel directory joined with the Cargo package name; the extension
#   never reads `Cargo.Bazel.lock`, `cargo-bazel.json`, or crate_universe's
#   private dependency maps.
# - ad-hoc metadata rejected. Backends stay provisional; floors qualified
# seed-only, coverage qualified seed-only.
#
# Versioned here, run by CI via `bazel run //tools/ci:cargo_metadata_qualification`,
# following //tools/ci:lcov_accounting_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

pins="rust/tests/fixtures/cargo_metadata/pins.bzl"
pins_build="rust/tests/fixtures/cargo_metadata/BUILD.bazel"
pins_manifest="rust/tests/fixtures/cargo_metadata/Cargo.toml"
pins_expected="rust/tests/fixtures/cargo_metadata/cargo_metadata.expected"
native="docs/native-toolchains.md"
matrix="docs/product/support-matrix.md"
contract="docs/generation/rust.md"
gen_readme="docs/generation/foundation-qualification.md"
build="tools/ci/BUILD.bazel"
ci=".github/workflows/ci.yml"
verify="docs/testing/verification-matrix.md"
cargo_go="gazelle/rust/cargo.go"
lang_go="gazelle/rust/lang.go"

# Fixture files stay present.
if [[ -f "$pins" && -f "$pins_build" && -f "$pins_manifest" && -f "$pins_expected" ]]; then
  ok
else
  bad "cargo metadata fixture missing (want $pins plus $pins_build plus Cargo.toml plus cargo_metadata.expected)"
fi

# Pins record the features shape with the kept testonly route and narrow export.
if grep -q -F -e 'nonempty required-features stays opt-in with kept testonly' "$pins" &&
  grep -q -F -e 'keep a handwritten testonly target selecting them' "$pins" &&
  grep -q -F -e 'does not enable features or emit the target unconditionally' "$pins" &&
  grep -q -F -e 'no private rules_rs data dependency and no project-owned Cargo feature resolver' "$pins" &&
  grep -q -F -e 'until upstream exposes exact first-party target admissibility' "$pins" &&
  grep -q -F -e 'explicit crate_features' "$pins"; then
  ok
else
  bad "pins.bzl lost its features shape plus kept testonly plus narrow admissibility export under issue #502"
fi

# Pins record build-script metadata with authoritative keys plus version plus scopes.
if grep -q -F -e 'conventional build.rs plus explicit package.build plus build=false authoritative' "$pins" &&
  grep -q -F -e 'package version feeds script version plus single-version checks' "$pins" &&
  grep -q -F -e 'script sees only [build-dependencies]' "$pins" &&
  grep -q -F -e 'use_cc_toolchain True plus use_default_shell_env False plus emit_warnings True' "$pins" &&
  grep -q -F -e 'script runs later as a Bazel execution action, never during generation' "$pins" &&
  grep -q -F -e "generation does not recreate Cargo's protocol" "$pins" &&
  grep -q -F -e 'BuildScriptInfo outputs stay narrow upstream metadata exports' "$pins"; then
  ok
else
  bad "pins.bzl lost its build-script metadata shape plus version plus scopes plus hermetic defaults under issue #502"
fi

# Pins record the target-kinds shape with the public mapping and kept-route rejection.
if grep -q -F -e 'ordinary libraries plus binaries plus tests plus proc macros plus sole cdylib plus sole staticlib' "$pins" &&
  grep -q -F -e 'only through the stable fixture-proven public upstream mapping' "$pins" &&
  grep -q -F -e 'dylib plus multiple crate types plus unknown kinds fail with kept handwritten-target route' "$pins" &&
  grep -q -F -e 'generation does not coerce kinds or duplicate the crate root' "$pins" &&
  grep -q -F -e 'only explicitly declared ordinary-binary targets with affixed names' "$pins"; then
  ok
else
  bad "pins.bzl lost its target-kinds shape plus public mapping plus kept-route rejection under issue #502"
fi

# Pins record ownership with single-owner enforcement plus sibling and mirror edges.
if grep -q -F -e 'authoritative crate root owns its root and every recursively loaded module' "$pins" &&
  grep -q -F -e 'integration-test roots separately own their trees' "$pins" &&
  grep -q -F -e 'one source may have only one owner' "$pins" &&
  grep -q -F -e 'bins plus tests plus examples plus benches link the same-package library automatically' "$pins" &&
  grep -q -F -e 'declared first-party path dependencies mirror without detection evidence' "$pins" &&
  grep -q -F -e 'emitted library flavors stay //visibility:public while bins plus tests plus scripts stay private' "$pins"; then
  ok
else
  bad "pins.bzl lost its ownership shape plus single-owner plus sibling plus mirror under issue #502"
fi

# Pins record the public shape without private serialized graph access.
if grep -q -F -e 'checked-in Cargo declarations parsed in the first-party extension' "$pins" &&
  grep -q -F -e "path crates resolved through Gazelle's local rule index" "$pins" &&
  grep -q -F -e 'exact imported external names emitted through the public @crates//:crates.bzl crate_deps and aliases macros' "$pins" &&
  grep -q -F -e 'parent Bazel directory joined with the Cargo package name' "$pins" &&
  grep -q -F -e 'never reads Cargo.Bazel.lock plus cargo-bazel.json plus private dependency maps' "$pins" &&
  grep -q -F -e 'without private serialized dependency-graph access' "$pins"; then
  ok
else
  bad "pins.bzl lost its public shape plus local index plus public macros plus never-reads under issue #502"
fi

# Pins record the rejected substitutes (ad-hoc metadata).
if grep -q -F -e '"ad-hoc metadata"' "$pins" &&
  grep -q -F -e '"private serialized dependency-graph access"' "$pins" &&
  grep -q -F -e '"Cargo.Bazel.lock read"' "$pins" &&
  grep -q -F -e '"cargo-bazel.json read"' "$pins" &&
  grep -q -F -e '"private dependency maps read"' "$pins"; then
  ok
else
  bad "pins.bzl lost its ad-hoc metadata rejection under issue #502"
fi

# Native plan owns the qualified metadata record plus fixture proof.
if grep -q -F -e 'qualified seed-only under issue #502' "$native" &&
  grep -q -F -e 'cargo_metadata_qualification' "$native" &&
  grep -q -F -e 'rust/tests/fixtures/cargo_metadata/pins.bzl' "$native" &&
  grep -q -F -e 'Can public Cargo metadata represent every generated target?' "$native" &&
  grep -q -F -e 'ad-hoc metadata rejected' "$native"; then
  ok
else
  bad "docs/native-toolchains.md lost its qualified cargo metadata record with fixtures under issue #502"
fi

# Support matrix owns the qualified metadata record with no Supported claim.
if grep -q -F -e 'qualified seed-only under issue #502' "$matrix" &&
  grep -q -F -e 'cargo_metadata_qualification' "$matrix" &&
  grep -q -F -e 'rust/tests/fixtures/cargo_metadata/pins.bzl' "$matrix"; then
  ok
else
  bad "docs/product/support-matrix.md lost its qualified cargo metadata record under issue #502"
fi

# Generation contract owns the qualified metadata record with fixtures.
if grep -q -F -e 'qualified seed-only under issue #502' "$contract" &&
  grep -q -F -e 'cargo_metadata_qualification' "$contract" &&
  grep -q -F -e 'rust/tests/fixtures/cargo_metadata/pins.bzl' "$contract"; then
  ok
else
  bad "docs/generation/rust.md lost its qualified cargo metadata record under issue #502"
fi

# Generation README pins the qualified metadata alongside the other gaps.
if grep -q -F -e 'issue #502' "$gen_readme" &&
  grep -q -F -e 'cargo metadata' "$gen_readme" &&
  grep -q -F -e 'cargo_metadata_qualification' "$gen_readme"; then
  ok
else
  bad "docs/generation/foundation-qualification.md lost its qualified cargo metadata record under issue #502"
fi

# BUILD owns the harness target plus CI wires it in dogfood-freshness.
if grep -q -F -e 'name = "cargo_metadata_qualification"' "$build" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:cargo_metadata_qualification' "$ci"; then
  ok
else
  bad "tools/ci/BUILD.bazel or ci.yml lost the cargo_metadata_qualification wiring (want target plus dogfood-freshness)"
fi

# Verification matrix owns the qualified seed-only record under.
if grep -q -F -e 'cargo_metadata_qualification' "$verify" &&
  grep -q -F -e 'qualified seed-only under #502' "$verify" &&
  grep -q -F -e 'bazel run //tools/ci:cargo_metadata_qualification' "$verify" &&
  grep -q -F -e '`cargo_metadata_qualification` 16/16' "$verify"; then
  ok
else
  bad "verification-matrix lost its #502 cargo metadata qualified record"
fi

# Fixture Cargo plus expected cover every generated target shape.
if grep -q -F -e 'name = "cargo-metadata"' "$pins_manifest" &&
  grep -q -F -e 'build = "build.rs"' "$pins_manifest" &&
  grep -q -F -e '[[test]]' "$pins_manifest" &&
  grep -q -F -e '[[example]]' "$pins_manifest" &&
  grep -q -F -e '[[bench]]' "$pins_manifest" &&
  grep -q -F -e '[build-dependencies]' "$pins_manifest" &&
  grep -q -F -e 'cargo_metadata_build_script (cargo_build_script)' "$pins_expected" &&
  grep -q -F -e 'demo_example (rust_binary)' "$pins_expected" &&
  grep -q -F -e 'criterion_bench (rust_binary)' "$pins_expected" &&
  grep -q -F -e 'one source has only one owner' "$pins_expected"; then
  ok
else
  bad "Cargo.toml plus cargo_metadata.expected lost target coverage (want lib/bin/test/example/bench/build-script plus single-owner, issue #502)"
fi

# Implementation keeps the public contract: required-features stays opt-in,
# kinds fold without coercion, and private serializations stay unread.
if grep -q -F -e 'required-features stay opt-in' "$cargo_go" &&
  grep -q -F -e 'one source owner maps to one Bazel target' "$cargo_go" &&
  grep -q -F -e 'one source may have only one owner' "$cargo_go" &&
  grep -q -F -e 'cargo_build_script' "$lang_go" &&
  ! grep -q -F -e 'Cargo.Bazel.lock' "$cargo_go" &&
  ! grep -q -F -e 'cargo-bazel.json' "$cargo_go" &&
  ! grep -q -F -e 'Cargo.Bazel.lock' "$lang_go" &&
  ! grep -q -F -e 'cargo-bazel.json' "$lang_go"; then
  ok
else
  bad "gazelle/rust lost its public cargo metadata contract (want opt-in required-features plus single-owner plus script kind with no private reads)"
fi

# Live proof: the cargo-aware Gazelle suites plus the already-wired hello plus
# cc_optout shapes stay green (generation unit plus testdata plus builds/tests).
if bazel test //gazelle/rust:rust_test --noshow_progress >/dev/null 2>&1 &&
  bazel test //gazelle/rust:generation_test --noshow_progress >/dev/null 2>&1 &&
  bazel build //rust/tests/fixtures/hello:hello //rust/tests/fixtures/cc_optout:cc_optout //rust/tests/fixtures/cargo_metadata/... --noshow_progress >/dev/null 2>&1 &&
  bazel test //rust/tests/fixtures/hello:hello_test //rust/tests/fixtures/cc_optout:cc_optout_test --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "cargo metadata live shapes failed (want gazelle rust_test plus generation_test plus hello plus cc_optout plus cargo_metadata green, issue #502)"
fi

dx_test_summary "cargo metadata qualification harness"
