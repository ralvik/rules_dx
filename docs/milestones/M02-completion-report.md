# M02 Completion Report: Minimum Rust Wrappers And Providers

M02 delivers experimental `dx_rust_library/binary/test` wrappers over the
pinned `rules_rust 0.74.0` ruleset, the provisional `QualitySourcesInfo`
boundary, and Starlark conformance tests proving provider preservation,
single source ownership, and pinned-toolchain lint/tool identities. All work
runs on the M00 seed host (`sdu-144133.pc.sdu.dk`, Ubuntu 26.04.1, Bazelisk
v1.29.0 -> Bazel 9.2.0).

## Verdicts

- `bazel build //...`: success, 27 actions.
- `bazel test //...`: 12/12 pass (3 Rust/M00-M02, 2 wrapper/M02, 7 M01).
- `bazel coverage //...` + coverage gate: PASS 1118/1118 executable lines,
  exit 0. Starlark subjects/tests contribute no `DA` records; the gate
  denominator is unchanged from M01.
- `bazel run //rust/hello:hello`: `Hello, world!` (wrapper binary identical
  to upstream).
- Lint non-vacuity: appending `fn   misformatted( ) {}` to `lib.rs` fails
  `hello_fmt_test` with `Diff in .../rust/hello/src/lib.rs:51`; revert
  restores PASS. `hello_clippy_test` passes over the wrappers.
- No ambient Cargo/Rust discovery: `aquery` over the three wrapper targets
  matches 0 `build_script`/`cargo` actions; the first-party
  `deps(//rust/hello/...)` closure holds only wrappers, private upstreams,
  subjects, tests, and the two `.rs` sources. The only
  `cargo_build_script` targets in `deps(//...)` belong to the toolchain's
  own third-party host tools (`rrc__thiserror`, `rrc__typeid`), never to
  first-party targets.

## Work Packages

- WP1 (pinned upstream providers/configs): conformance subjects observe
  `rust_library/binary/test` shapes from `rules_rust 0.74.0`. Every wrapped
  upstream yields `CrateInfo` + `DepInfo` (never `TestCrateInfo` except via
  `staticlib`/`cdylib` per upstream `rustc.bzl`); `owner` points at the
  private `<name>_dx_upstream` target; `is_test` is true only for the
  `crate =` test. Toolchain type `@rules_rust//rust:toolchain_type`
  resolves `rustc/cargo/rustfmt/clippy-driver` under the pinned external
  repository (`tools_pinned=True`); rustfmt/Clippy identities come from
  `rustfmt_test`/`rust_clippy_test` over the wrappers, never host tools.
- WP2 (minimal wrappers, preservation): each macro creates one private
  `<name>_dx_upstream` target plus one public forwarder advertising
  `provides = [CrateInfo, DepInfo, DefaultInfo, InstrumentedFilesInfo,
  QualitySourcesInfo]`. All six `preserved_*` comparisons (name, edition,
  type, root, srcs, dep count) are true on all three shapes; both sides
  carry `InstrumentedFilesInfo`; `OutputGroupInfo`/`RunEnvironmentInfo`
  forward at runtime unadvertised, mirroring upstream `COMMON_PROVIDERS`.
- WP3 (narrow normalization): the only new fact is
  `QualitySourcesInfo(direct_sources = {"rust": <direct srcs>})`
  (`wrapper_has_quality_sources=True`,
  `upstream_has_quality_sources=False`). The `crate =` test reports
  `direct_sources=(none)`: the crate owner stays the single source owner.
  Lint markers are exactly `hello.rustfmt.ok,hello_lib.rustfmt.ok` and
  `hello.clippy.ok,hello_lib.clippy.ok`: aspects visit the wrappers and do
  not traverse the private `_dx_upstream` edge.

## Upstream Symbols, Mappings, Gaps, Limits

- Used from `@rules_rust//rust:defs.bzl`: `rust_library`, `rust_binary`,
  `rust_test`, `rust_common` (`crate_info`, `dep_info`, `test_crate_info`),
  `rustfmt_test`, `rust_clippy_test`. Toolchain types:
  `@rules_rust//rust:toolchain_type`, `@rules_rust//rust/rustfmt:toolchain_type`.
  No other upstream surface is used.
- Wrapper gap: the forwarder keeps the upstream `CrateInfo` object, so the
  binary crate name is the upstream target name (`hello_dx_upstream`);
  generation work (M09) must read names from wrapper labels, not `CrateInfo`.
- Coverage fidelity gaps closed: the test forwarder must advertise/forward
  `InstrumentedFilesInfo` and declare the `_lcov_merger` magic attribute
  (same declaration as upstream `rust_test`); without either, the wrapper
  test yields an empty `coverage.dat` while the staging report is full.
- Toolchain-selection limit: host/target/execution split is unproven beyond
  the local seed host; multi-platform qualification stays with M27/M28 per
  ADR 0014.

## Environment Findings (Pinned Bazel 9.2.0, Bzlmod, Local Linux)

- Bazel matches an aspect's `required_providers` against the rule's
  advertised `provides`, not returned providers: a forwarder returning
  `CrateInfo` without advertising it is silently skipped by the lint
  aspects (vacuous PASS). `provides` is strictly validated, so the
  forwarder fails loudly on shape drift instead.
- A rule providing an executable must create that file itself: the
  binary/test forwarders symlink the upstream executable via their own
  action (`bazel run`/`test` behavior identical to upstream).
- `rule()` takes no `testonly` keyword; `testonly` is a common attribute
  set at instantiation (used by the probe subjects, which observe
  test-only lint targets).
- `str(Label)` renders canonical `@@//...`; the probe strips one leading
  `@@` exactly like the M01 framework (requalify on pin bumps per O14).

## Changed Components

- New: `rust/rules/defs.bzl` (wrappers), `rust/rules/probe.bzl`
  (`dx_wrapper_subject`), `rust/rules/wrapper_tests.bzl`
  (`dx_wrapper_registry_tests`, `dx_wrapper_conformance_tests` with pinned
  `EXPECTED_OBSERVATIONS`), `rust/rules/BUILD.bazel`,
  `quality/sources.bzl` (`QualitySourcesInfo`, registry, `RUST`),
  `quality/BUILD.bazel`.
- Migrated: `rust/hello/BUILD.bazel` consumes `dx_rust_*`, adds lint tests
  plus three conformance subjects and both test macros.
- Unchanged contracts: `docs/quality/quality-sources.md` shape stays
  accepted; the constructor/registry remain the M02 provisional pick
  pending the O15 freeze; O16 exact mappings stay pending (M02 proves the
  fixture direction only).

## Gaps (Accepted)

- Local Linux only; other ADR 0014 platforms remain gaps per M00.
- No remote cache/execution evidence; no formatter/linter (M04/M05).
- Complete Cargo semantics, Gazelle generation, IDE environments, and
  public support stay out of scope per the M02 spec (M09/M12 own them).
