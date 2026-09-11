# M12 Completion Report: Complete Rust Foundation

Seed host: local Linux x86_64 glibc (same host class as the M00 seed report).
Local execution only, no remote. Non-Linux platforms were unavailable and are
recorded as gaps, not claimed. No capability-term transitions are claimed:
the foundation is adapter-tested first-party implementation under test, not a
product support claim. Public repository/root/exact-target env planning,
collection, and orchestration remain M25.

## WP1: Wrappers, Providers, Version Behavior

All six v1 wrapper shapes forward the authoritative upstream providers
unchanged (`rust/rules/defs.bzl`): `dx_rust_library`, `dx_rust_binary`,
`dx_rust_test`, and `dx_rust_proc_macro` preserve `CrateInfo` + `DepInfo`;
`dx_rust_shared_library` and `dx_rust_static_library` preserve
`TestCrateInfo` + `DepInfo` + `CcInfo` (upstream exposes no `CrateInfo` for
Cc-linking shapes). Every wrapper adds `QualitySourcesInfo` normalized from
its direct `srcs` plus mandatory `InstrumentedFilesInfo` forwarding, so
coverage collection survives the wrapper boundary. Conformance is pinned by
`//rust/hello:wrapper_registry_tests` (advertised provider sets) and
`//rust/hello:wrapper_conformance_tests` (six subjects, one per shape).

Version behavior is a documented omission, not an ignored field: per-target
toolchain selection does not exist upstream (single pinned toolchain per
[ADR 0012](../decisions/0012-language-toolchain-versions.md)), so the
[Versions section](../generation/rust.md) records the omission and Cargo
version metadata stays authoritative for script-version and path-dependency
checks.

## WP2: Cargo Matrix, Examples, Benchmarks, Unsupported Semantics

Gazelle emits the complete v1 Cargo matrix (`gazelle/rust/lang.go`,
`gazelle/rust/cargo.go`): `dx_rust_library`, `dx_rust_binary`,
`dx_rust_test`, `dx_rust_proc_macro`, `dx_rust_shared_library`,
`dx_rust_static_library`, and the upstream `cargo_build_script` wrapper
macro. Declared examples and benchmarks map to generated binary/test
targets; lock-only and source-only fixtures preserve native crate ownership
and authoritative metadata (`gazelle/rust/testdata/cargo`,
`gazelle/rust/testdata/source_only`). Name-claim collisions across
generated and existing rules stay fail-closed with no partial emission;
unsupported shapes fail closed through the same claim path. Golden
`BUILD.out` fixtures plus `cargo_test.go`/`lang_test.go` pin the matrix.

## WP3: Compiler Diagnostics, IDE, Coverage, And Environment Projection

Compiler diagnostics are owned end to end: the `rust_toolchain_rustc`
invocation, finding parser, and runner plus registry/aspect/family/CLI
wiring, with `typecheck_clean.rs`/`typecheck_dirty.rs` fixtures proving
target ownership, normalized findings, and failure status
(`//quality/testdata:real_typecheck_presence` and siblings pass).

IDE support reuses the pinned patched upstream binaries with no
project-owned discovery: `//rust/ide:ide_acquisition_test` proves
`gen_rust_project` and `flycheck` answer `--help` with status zero.
Focused exact-target projection was run manually and is recorded here:
`bazel run @rules_rust//tools/rust_analyzer:gen_rust_project --
 //rust/hello:hello_lib` produced `rust-project.json` with exactly one
crate (`hello`); the file was removed afterwards, so no generated graph is
checked in and no environment or codegen selection was mutated. Dynamic
refresh shares those semantics per the [Rust environment](../environments/rust.md#ide-integration)
contract.

Coverage is proven through the wrapper, not around it:
`bazel coverage //rust/...` passes (10/10) and the wrapper
`//rust/hello:hello_test` `coverage.dat` attributes `rust/hello/src/lib.rs`
(49-line LCOV record, one `SF` entry). New Cargo fixture sources were added
to `tools/coverage/inventory.txt`; the new Starlark packages need no
inventory entries (only `*.rs`/`*.go` reconcile) and are covered behaviorally
by their tests.

The focused environment plan is the provider-derived `rust_env_plan` rule
(`rust/env/plan.bzl`): crate identity, type, edition, root, direct sources,
and direct dependency count read from the analyzed `CrateInfo`
(library/binary/test) or `TestCrateInfo` (Cc-linking shapes) of one wrapper
target, materialized as deterministic JSON plus `DxSubjectInfo`.
`//rust/env:env_plan_tests` pins four plans: `hello_lib` (rlib, 1 dep,
`lib.rs`), `hello` (bin, 2 deps, `main.rs`), `hello_test` (bin view of the
test crate, 1 dep, no direct sources because `srcs` are owned by
`hello_lib`), and `hello_cdylib` (cdylib via `TestCrateInfo`, 0 deps,
`cdylib.rs`). No checkout scan, no Cargo re-resolution, no
repository/root/exact-target orchestration (M25).

Build-script policy follows the [generation contract](../generation/rust.md#build-scripts):
native exposure defaults on with a kept opt-out; shell environment and
nonhermetic-path discovery default off. Proven: hermetic default for the
nonhermetic path (`d8a081d`) and user-owned `data`/`tools`/`build_script_env`/`files`/`toolchains`
surviving generation via non-mergeable `# keep` semantics
(`TestBuildScriptUserAttrsPreserved`, `7fe9c43`). The kept-CC-opt-out
execution reproduction stays an O24-owned qualification question per the
[native plan](../native-toolchains.md#qualification-questions-and-delivery);
it is recorded there, not silently deferred.

## WP4: Full Suites

`bazel test //...`: 82 tests pass, 0 fail (local Linux x86_64 glibc).
`bazel build //rust/... //quality/... //env/...` succeeds. Buildifier
`--mode=check --lint=warn` is clean over the new `rust/env` and `rust/ide`
Starlark. New packages declare M05 `corpus` targets for direct-Bazel
dogfood; the IDE acquisition script is a test runner, never a linted corpus
source.

## Open Items

- O24 remains open: bindgen/CXX single-graph fixtures and kept-CC-opt-out
  execution reproduction are owned downstream per the qualification plan.
- Non-Linux hosts and remote execution are unproven (same gap class as M00).
