"""Cargo metadata pins (issue #502).

Contract: `docs/native-toolchains.md#qualification-questions-and-delivery`,
`docs/generation/rust.md#crates-and-cargo`,
`docs/testing/generation.md`.
Fixture: `rust/tests/fixtures/cargo_metadata/` via
`bazel run //tools/ci:cargo_metadata_qualification`.

Proves public Cargo metadata represents every generated target on the
seed host without private serialized dependency-graph access, native only:

- features: a nonempty Cargo target `required-features` list stays
  unsupported until upstream exposes exact first-party target
  admissibility; generation reports the target, features, and manifest
  and points to a kept handwritten `testonly` target with explicit
  `crate_features`; it does not enable features or emit the target
  unconditionally; no private `rules_rs` data dependency and no
  project-owned Cargo feature resolver.
- build-script metadata: conventional `build.rs`, explicit
  `package.build` path, and `package.build = false` are authoritative;
  `[package] version` feeds the generated build-script `version`
  attribute and single-version path-dependency checks; the script sees
  only `[build-dependencies]`; generated rules carry hermetic defaults
  (`use_cc_toolchain = True`, `use_default_shell_env = False`,
  `emit_warnings = True`); the script runs later as a Bazel execution
  action, never during generation; generation does not recreate Cargo's
  protocol.
- target kinds: ordinary libraries, binaries, tests, proc macros, sole
  `cdylib`, and sole `staticlib` generate only through the stable
  fixture-proven public upstream mapping; `dylib` plus multiple crate
  types for one source owner plus unknown kinds fail with manifest
  context and a user-owned kept-target route; generation does not coerce
  kinds or duplicate the crate root; examples plus benches generate only
  as explicitly declared ordinary-binary targets with affixed names.
- ownership: the authoritative crate root owns its root and every
  recursively loaded module; integration-test roots separately own
  their trees; one source may have only one owner; bins plus tests plus
  examples plus benches link the same-package library automatically;
  declared first-party path dependencies mirror without detection
  evidence; emitted library flavors stay `//visibility:public` while
  bins plus tests plus scripts stay private.
- public shape: checked-in Cargo declarations parsed in the first-party
  extension, path crates resolved through Gazelle's local rule index,
  exact imported external names emitted through the public
  `@crates//:crates.bzl` `crate_deps` and `aliases` macros; the macro
  `package_name` is the parent Bazel directory joined with the Cargo
  package name; the extension never reads `Cargo.Bazel.lock`,
  `cargo-bazel.json`, or crate_universe's private dependency maps.
- narrow upstream exports where missing: `required-features`
  admissibility plus `BuildScriptInfo` outputs stay narrow upstream
  metadata exports, never ad-hoc private graph reads.

Ad-hoc metadata rejected per the issue alternatives. Backends stay
provisional; floors qualified seed-only under issue #500, coverage qualified
seed-only under issue #501; no `Supported` claim.
"""

# Features: nonempty required-features stays opt-in with a kept testonly route.
FEATURES_SHAPE = "nonempty required-features stays opt-in with kept testonly"
FEATURES_KEPT_ROUTE = "keep a handwritten testonly target selecting them"
FEATURES_NO_UNCONDITIONAL = "does not enable features or emit the target unconditionally"
FEATURES_NO_PRIVATE_RESOLVER = "no private rules_rs data dependency and no project-owned Cargo feature resolver"
FEATURES_NARROW_EXPORT = "until upstream exposes exact first-party target admissibility"
FEATURES_CRATE_FEATURES = "explicit crate_features"

# Build-script metadata: authoritative build key plus version plus build-deps scope.
BUILD_SCRIPT_SHAPE = "conventional build.rs plus explicit package.build plus build=false authoritative"
BUILD_SCRIPT_VERSION = "package version feeds script version plus single-version checks"
BUILD_SCRIPT_SCOPES = "script sees only [build-dependencies]"
BUILD_SCRIPT_DEFAULTS = "use_cc_toolchain True plus use_default_shell_env False plus emit_warnings True"
BUILD_SCRIPT_EXEC = "script runs later as a Bazel execution action, never during generation"
BUILD_SCRIPT_NO_PROTOCOL = "generation does not recreate Cargo's protocol"
BUILD_SCRIPT_NARROW_EXPORT = "BuildScriptInfo outputs stay narrow upstream metadata exports"

# Target kinds: accepted set generates only through the stable public mapping.
TARGET_KINDS_SHAPE = "ordinary libraries plus binaries plus tests plus proc macros plus sole cdylib plus sole staticlib"
TARGET_KINDS_PUBLIC_MAPPING = "only through the stable fixture-proven public upstream mapping"
TARGET_KINDS_REJECTED = "dylib plus multiple crate types plus unknown kinds fail with kept handwritten-target route"
TARGET_KINDS_NO_COERCION = "generation does not coerce kinds or duplicate the crate root"
EXAMPLE_BENCH_SHAPE = "only explicitly declared ordinary-binary targets with affixed names"

# Ownership: per-crate root owns its tree with single-owner enforcement.
OWNERSHIP_SHAPE = "authoritative crate root owns its root and every recursively loaded module"
OWNERSHIP_TEST_ROOTS = "integration-test roots separately own their trees"
OWNERSHIP_ONE_OWNER = "one source may have only one owner"
OWNERSHIP_SIBLING_LIB = "bins plus tests plus examples plus benches link the same-package library automatically"
OWNERSHIP_MIRROR = "declared first-party path dependencies mirror without detection evidence"
OWNERSHIP_VISIBILITY = "emitted library flavors stay //visibility:public while bins plus tests plus scripts stay private"

# Public shape: checked-in declarations plus local index plus public macros.
PUBLIC_SHAPE = "checked-in Cargo declarations parsed in the first-party extension"
PUBLIC_INDEX = "path crates resolved through Gazelle's local rule index"
PUBLIC_MACROS = "exact imported external names emitted through the public @crates//:crates.bzl crate_deps and aliases macros"
PUBLIC_PACKAGE_NAME = "parent Bazel directory joined with the Cargo package name"
PUBLIC_NEVER_READS = "never reads Cargo.Bazel.lock plus cargo-bazel.json plus private dependency maps"
PUBLIC_NO_PRIVATE_GRAPH = "without private serialized dependency-graph access"

# Rejected substitutes per the issue alternatives: ad-hoc metadata.
REJECTED_ALTERNATIVES = [
    "ad-hoc metadata",
    "unconditional required-features target",
    "kind coercion",
    "duplicate root ownership",
    "private serialized dependency-graph access",
    "Cargo.Bazel.lock read",
    "cargo-bazel.json read",
    "private dependency maps read",
]

# Live proof labels: the fixture Cargo plus expected pair plus the
# already-wired hello plus cc_optout plus cargo testdata shapes.
CARGO_METADATA_FIXTURE_MANIFEST = "//rust/tests/fixtures/cargo_metadata:Cargo.toml"
CARGO_METADATA_FIXTURE_EXPECTED = "//rust/tests/fixtures/cargo_metadata:cargo_metadata.expected"
CARGO_METADATA_LIVE_HELLO = "//rust/tests/fixtures/hello:hello"
CARGO_METADATA_LIVE_CC_OPTOUT = "//rust/tests/fixtures/cc_optout:cc_optout"
CARGO_METADATA_TESTDATA_APP = "gazelle/rust/testdata/cargo/crates/app"
CARGO_METADATA_TESTDATA_SHAPES = "gazelle/rust/testdata/cargo/crates/shapes"
