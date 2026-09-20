# Generation

These documents are the authoritative contracts for first-party BUILD generation. The accepted
ADRs explain why the contracts exist; the test matrix defines the required evidence.

## Contracts

- [Common](common.md): Gazelle APIs, resolution, ownership, merge, lifecycle, resources, and entry
  points.
- [Rust](rust.md): crate and Cargo discovery, tests, examples, benchmarks, and build scripts.
- [Python](python.md): runtime sources, stubs, tests, and uv-backed dependency scope.
- [JavaScript and TypeScript](javascript-typescript.md): core extensions, pnpm scope, and tests.
- [Framework adapters](framework-adapters.md): Vue, Svelte, Astro, and MDX container boundaries.

## Related Authorities

- [`dx generate`](../cli/commands/generate.md) defines the command workflow and output behavior.
- [ADR 0015](../decisions/0015-first-party-gazelle-extensions.md),
  [ADR 0013](../decisions/0013-rust-javascript-typescript-foundations.md), and
  [ADR 0010](../decisions/0010-python-foundation.md) record the rationale.
- [Generation test matrix](../testing/generation.md) defines conformance evidence.

## Language Mapping Qualification

Accepted. Each foundation keeps its provisional upstream; no switch is approved here.

Core (Rust, Python, JavaScript, TypeScript) plus admitted additional foundations
(Go, Java, Kotlin, Scala, C#, F#, C/C++) are qualified by focused fixtures:
thin wrappers in `<lang>/rules/defs.bzl` preserving the upstream provider and
adding `QualitySourcesInfo`, Gazelle extensions in `gazelle/<lang>/` with
parser/naming/lang fixtures plus focused tests, hello builds in `<lang>/tests/fixtures/hello/`
as wrapper consumers, and lock authority (`rust/tests/fixtures/hello/Cargo.lock` plus
`cargo-bazel-lock.json` and `MODULE.bazel.lock`, `python/tests/fixtures/hello/uv.lock` plus
tools lock, `pnpm-lock.yaml` plus tools lock,
`third_party/jvm/maven_install.json`, `third_party/dotnet/paket.lock` plus
`paket.dependencies` qualified seed-only under issue #482,
`third_party/go/go.mod` plus `go.sum` qualified seed-only under issue #483
with hello staying stdlib-only).
C/C++ has none (every `http_archive` carries `sha256`/`integrity`).
C/C++ hash wiring qualified seed-only under issue #484 via
`bazel run //tools/ci:cc_hermetic_qualification` with `cc/tests/fixtures/hermetic/`.
Upstream pins live in `MODULE.bazel`.
Ruby and PowerShell have no generation mapping: deferred beyond v1 by
[ADR 0019](../decisions/0019-first-release-additional-foundations.md).
Swift has none: excluded from v1 by the same record.
Deferred/excluded generation record is decided by [ADR 0019](../decisions/0019-first-release-additional-foundations.md);
no `ruby/`, `powershell/`,
or `swift/` Gazelle extension lands here.

Required-core Rust provider plus Gazelle maps are pinned (issue #470):
`rust/rules/defs.bzl` wrappers preserve `CrateInfo`/`DepInfo`/`TestCrateInfo`/`CcInfo`
plus `QualitySourcesInfo`, proven by `rust/rules/wrapper_tests.bzl` conformance over the
six `rust/tests/fixtures/hello/` subjects; Gazelle kinds/loads in `gazelle/rust/lang.go`
(`rust_library`, `rust_binary`, `rust_test`, `rust_proc_macro`, `rust_shared_library`,
`rust_static_library`, `cargo_build_script`, `dx_rust_crate` with `rust/rules:defs.bzl`,
`cargo:defs.bzl`, `crates.bzl` loads) with cargo plus source-only plus merge plus
native-config goldens and focused tests; hello plus `Cargo.lock` plus
`cargo-bazel-lock.json` lock authority with `MODULE.bazel` pins.

Required-core Rust build-script hermetic defaults are implemented
(`use_cc_toolchain = True`, `use_default_shell_env = False`, `emit_warnings = True`
in `gazelle/rust/lang.go`, proven by `gazelle/rust/lang_test.go`, with the
third-party half pinned global `False` in `.bazelrc` and zero per-crate
opt-ins under issue #472); the kept CC opt-out linker path is pinned seed-only
under issue #471 (pure-Rust `rust/tests/fixtures/cc_optout/` with kept
`use_cc_toolchain = 0`, sysroot `rust-lld` fallback plus `no_cc` stubs,
`bazel run //tools/ci:cc_optout_qualification`); bindgen LLVM-22-vs-23 compat
is qualified under issue #473 with the LLVM-22 parser baseline vs LLVM-23
target pinned plus the standalone/build-script `bindgen.h` plus
`bindgen.expected` fixture pair
(`bazel run //tools/ci:bindgen_qualification`); CXX graph identity is pinned
seed-only under issue #474 (single crate_universe `crates` graph with
`cxx == cxxbridge-cmd == 1.0.200` and `@crates//:cxxbridge-cmd`, never a
`cxx.rs` second graph, proven by `rust/tests/fixtures/cxx_identity/` via
`bazel run //tools/ci:cxx_identity_qualification`); exact-target discovery is
qualified seed-only under issue #475 (resolver-owned exact labels to upstream
`TARGETS` with hello exact-isolation plus `rust/tests/fixtures/discovery/pins.bzl` via
`bazel run //tools/ci:exact_target_qualification`); remaining native gaps
(shell-env default)
stay owned under issue #472 per the
[native plan](../native-toolchains.md#qualification-questions-and-delivery).

Admitted additional foundations keep their provisional upstreams with hello test runners
pinned (`go_test` over `go test` with package-level `embed`, qualified seed-only under issue #478
(`go/tests/fixtures/gotest/pins.bzl` plus the hello fixture via
`bazel run //tools/ci:gotest_qualification`; implicit runner rejected),
`scala_test` over ScalaTest, qualified seed-only under issue #480
(`scala/tests/fixtures/scalatest/pins.bzl` plus the hello `AnyFlatSpec`
fixture via `bazel run //tools/ci:scalatest_qualification`; unpinned runner
rejected),
`java_test`/`kotlin_test` over the JUnit 4 seed plus the qualified JUnit 6.1.3 Jupiter
upgrade with 5.14.x fallback (`execute --select-class` console-launcher fixtures in
`java/tests/fixtures/junit/` plus `kotlin/tests/fixtures/junit/` via
`bazel run //tools/ci:junit_qualification` under issue #476), plain
`cc_test` assert executables plus the qualified GoogleTest v1.18.0 mapping
below, plain `csharp_test`/`fsharp_test` hello executables)
and lock authority (`third_party/jvm/maven_install.json`,
`third_party/dotnet/paket.lock` plus `paket.dependencies` qualified seed-only
under issue #482 via `bazel run //tools/ci:paket_qualification` with
`csharp/tests/fixtures/paket/pins.bzl`, Go `go.mod` plus `go.sum` qualified
seed-only under issue #483 via `bazel run //tools/ci:godeps_qualification` with
`go/tests/fixtures/godeps/pins.bzl`, C/C++ none qualified
seed-only under issue #484 via `bazel run //tools/ci:cc_hermetic_qualification`
with `cc/tests/fixtures/hermetic/`).
The qualified Maven maven_install.json plus fail-closed repin is pinned under issue #481
(single shared lock via `lock_file` plus `fail_if_repin_required` in `third_party/jvm/pins.bzl`
over the shared `@maven` hub, proven by the Java/Kotlin Jupiter plus seed fixtures via
`bazel run //tools/ci:maven_lock_qualification`; non-fail-closed rejected).
The xUnit v3 4.0.0 runner mapping is qualified under issue #477 (plain
`csharp_test`/`fsharp_test` with `[Fact]`/`[Theory]` sources plus checked-in
MTP entry-point shims over the pinned `@paket.main//xunit.v3` closure, proven by
`csharp/tests/fixtures/xunit/` and `fsharp/tests/fixtures/xunit/` via
`bazel run //tools/ci:xunit_qualification`; unpinned runner rejected).
The qualified GoogleTest v1.18.0 mapping is pinned under issue #479 (plain
`cc_test` with `TEST()`/`EXPECT_*` sources over the pinned
`@googletest//:gtest_main` with an explicit `-std=c++17` floor, proven by
`cc/tests/fixtures/googletest/` via
`bazel run //tools/ci:googletest_qualification`; living at head rejected).
Upgrades stay owned under issues #476-#484 (JUnit 6.1.3
plus 5.14.x fallback qualified seed-only under issue #476 via
`bazel run //tools/ci:junit_qualification`; xUnit v3 4.0.0 qualified seed-only
under issue #477 via `bazel run //tools/ci:xunit_qualification`; `go test` qualified
seed-only under issue #478 via `bazel run //tools/ci:gotest_qualification`; GoogleTest
v1.18.0 plus C++17 floor qualified seed-only under issue #479 via
`bazel run //tools/ci:googletest_qualification`; ScalaTest 3.2.20 qualified
seed-only under issue #480 via `bazel run //tools/ci:scalatest_qualification`;
Maven `maven_install.json` plus fail-closed repin qualified seed-only under issue #481 via
`bazel run //tools/ci:maven_lock_qualification`;
Paket files plus sha512 qualified seed-only under issue #482 via
`bazel run //tools/ci:paket_qualification`; Go `go.mod` plus `go.sum` qualified
seed-only under issue #483 via `bazel run //tools/ci:godeps_qualification`;
C/C++ sha256-integrity qualified seed-only under issue #484 via
`bazel run //tools/ci:cc_hermetic_qualification`);
no `Supported` claim
until platform plus consumer plus release evidence passes.

Pinned by `bazel run //tools/ci:foundation_maps`.
