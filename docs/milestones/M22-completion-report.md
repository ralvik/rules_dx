# M22 Completion Report: Additional Native Foundations And Toolchain Parity

Seed host: local Linux x86_64 glibc (same host class as the M00 seed report).
Local execution only, no remote. Non-Linux platforms were unavailable and are
recorded as gaps, not claimed. No capability-term transitions are claimed:
the integrations are adapter-tested first-party implementation under test, not a
product support claim. O30 stays open for its owner; this report supplies the
milestone-specific evidence (upstream stack, supported targets, provider
mappings, route proofs, platforms, limitations). No quality adapter claims any
M22 class yet; adapter qualification moves to M24 under O32, recorded below as
deferred scope rather than a silent drop.

## WP1: Upstream Foundations, Wrappers, Classification, Scala Route

Upstream stack (pinned): `rules_go 0.63.0` ruleset with Go SDK `1.26.6`
(`MODULE.bazel` `go_sdk.download`), and `rules_cc 0.2.22` build rules over the
[native qualification plan](../native-toolchains.md) (hermetic-llvm `v0.8.19`
Linux/macOS backend first, toolchains_msvc clang-cl/Microsoft-STL Windows
candidate still blocked). Decision: reuse the pinned upstream rulesets through
thin wrappers rather than replacement compiler rules; rationale: the narrowest
API that preserves upstream providers while adding the one new
`QualitySourcesInfo` fact quality aspects gate on.

`go/rules/defs.bzl` (`dx_go_library`, `dx_go_binary`, `dx_go_test`): one
private `<name>_dx_upstream` target plus one public forwarding target,
preserving `GoInfo`, `GoArchive`, `DefaultInfo`, `InstrumentedFilesInfo`
unchanged and adding `QualitySourcesInfo(direct_sources={"go": <direct .go>})`.
`importpath` always passes through with no invented default (Gazelle owns
inference later); cgo (`.c`/`.h`/assembly) scope fails closed until O30
qualifies it. `cc/rules/defs.bzl` (`dx_cc_library`, `dx_cc_binary`,
`dx_cc_test`) mirrors the shape over `cc_library`/`cc_binary`/`cc_test`,
preserving `CcInfo`, `DefaultInfo`, `InstrumentedFilesInfo` and adding
`QualitySourcesInfo` with header ownership per extension (`.h` to `c`,
`.hh`/`.hpp`/`.hxx` to `cpp`); CUDA, assembly, and `cc_shared_library` scope
fail closed. `go/hello` and `cc/hello` prove the boundaries (library/binary/
test over the wrappers, `bazel test //go/hello:hello_test //cc/hello:hello_test`
pass).

Classification: `go` to the `go` family, `c`/`cpp` to the `cc` family, and one
family per remaining standalone class (`cuda`, `cue`, `go_module`, `jsonnet`,
`pkl`, `protobuf`, `qml`, `shell`, `terraform`, `yaml`) in
`quality/adapters.bzl`; each keeps its existing ownership with no adapter claim
yet (classification only, no supported claim). O30 qualifies exact adapters later.

Scala/Scalafix route (O30, frozen 2026-09-13): the managed-runtime route owned
by M23 under O31. Coursier-fetched toolchains share Java's Maven-lock story
(`maven_install.json` plus `fail_if_repin_required`), Scalafix semantic rules
need semanticdb plus classpath wiring, and no source-built or
toolchain-coupled native advantage was evidenced; recorded in the
[open decisions](../open-decisions.md) and the
[support matrix](../product/support-matrix.md#additional-language-foundation-candidate-review).

## WP2: Generation, Env Plans, Focused Route Proofs

Physical ownership: each checked-in `.go` has one physical owner
(`dx_go_library`/`dx_go_binary`/`dx_go_test`, package-level test semantics
preserved per the generation-contract Go exception); each `.c`/`.h`/`.cc` file
has one physical owner (`dx_cc_library`/`dx_cc_binary`/`dx_cc_test`).
`gazelle/go` (single kind `dx_go_library`, `.go` only; package-main sources
stay handwritten) and `gazelle/cc` (single kind `dx_cc_library`, C/C++
sources+headers; main-defining sources stay handwritten) mirror the
`gazelle/vue` strict-resolution shape: source-only goldens
(`testdata/source_only`), self-deps dropped, stdlib skipped, errors abort
before emission, stale sweep, `dx_ignore_import` with inheritance and
stale-fail-closed.

Execution: `bazel test //go/hello:hello_test //cc/hello:hello_test` pass over
runfiles. Env: `go/env/plan.bzl` (`GoEnvPlanInfo` over preserved
`GoInfo`/`GoArchive` + `QualitySourcesInfo`, deterministic JSON +
`DxSubjectInfo`) with `//go/env:hello_lib_plan` pinned; `cc/env/plan.bzl`
mirrors it over `CcInfo`. Quality: no lint/format/typecheck adapter claims
`go`, `c`, `cpp`, or any standalone class yet.

Focused route proofs (O30 order: clang-format/clang-tidy, Buf, Scalafix routed
to M23, Qt last), recorded in the
[support matrix](../product/support-matrix.md#provisional-default-quality-tools)
and [tool acquisition](../tools/tool-acquisition.md#first-release-tool-routing):

- Clang (frozen 2026-09-13): clang-format and clang-tidy resolve from the
  qualified hermetic-llvm LLVM distribution's tool targets
  (authoritative-toolchain class, no separate acquisition); cppcheck stays a
  standalone checksummed-artifact candidate pending adapter qualification.
- Buf (frozen 2026-09-13): `buf` format+lint takes the checksummed
  native/self-contained artifact route (self-contained per-platform release
  binaries, no target compiler context, execution-platform lazy); exact
  assets, digests, and adapter qualification stay pending.
- Qt, last (frozen 2026-09-13): qmlformat and qmllint resolve from the Qt
  distribution (authoritative-toolchain class); exact Qt distribution
  identity, licensing, and platform artifact qualification stay pending.

## WP3: Standalone Ownership, Laziness, Deferred Adapters

The ten standalone families keep their existing ownership (foundation wrappers
where admitted, otherwise the file itself); the `protobuf` class additionally
records the generated-source projection boundary from the support matrix.
Laziness: both Gazelle adapters generate nothing for sourceless directories,
sweep only their own kinds, leave non-owned kinds untouched, and add no
test/binary inference (unit- and golden-proven alongside the M18-M21 adapters).

Deferred scope (evidence-backed, owned by M24 under O32, not silent):
no M22 quality adapter (gofumpt, staticcheck, govet, clang-format,
clang-tidy, cppcheck, `buf`, shfmt, ShellCheck, yamlfmt, and the remaining
standalone tools) is implemented here; exact versions, rule sets, config
mappings, and common-runner exceptions close in M24 parity closure. A
foundation deferral removes no baseline tool: every named quality cell stays
in force.

## Evidence

Exact commands on this host, committed tree:

- `bazel build //...`: success (540 targets).
- `bazel test //...`: 122/122 pass, including `//gazelle/go:go_test`,
  `//gazelle/go:generation_test`, `//gazelle/cc:cc_test`,
  `//gazelle/cc:generation_test`, `//go/env:env_plan_tests`,
  `//cc/env:env_plan_tests`, `//go/hello:hello_test`,
  `//cc/hello:hello_test`.
- `bazel coverage //gazelle/go:go_test //gazelle/cc:cc_test`: pass.
- `bazel run //dx:generate_check`: clean.
- Coverage inventory registers `gazelle/go` and `gazelle/cc` eligible +
  support paths; `//tools/coverage:coverage_test` passes in the full run.

## Changed Components

- `MODULE.bazel` (`rules_go 0.63.0`, `gazelle 0.52.2`, `rules_shell 0.6.1`,
  Go SDK `1.26.6` already pinned; `rules_cc 0.2.22` seed pin reused).
- `go/rules/` (`defs.bzl`, `BUILD.bazel`), `go/hello/` (fixture + test),
  `go/env/` (`plan.bzl`, `plan_tests.bzl`, `BUILD.bazel`).
- `cc/rules/` (`defs.bzl`, `BUILD.bazel`), `cc/hello/` (fixture + test),
  `cc/env/` (`plan.bzl`, `plan_tests.bzl`, `BUILD.bazel`).
- `gazelle/go/`, `gazelle/cc/` (adapters, parsers, naming, tests, goldens).
- `quality/adapters.bzl` (`go`/`cc`/standalone family classification).
- `tools/coverage/inventory.txt` (go/cc paths).
- `docs/product/support-matrix.md` (Scala managed route, Clang route,
  O30/O31 cohort determination), `docs/open-decisions.md` (O30/O31),
  `docs/tools/tool-acquisition.md` (Buf/Clang/Qt routing).
- This report.

## Open Items

- O30 stays open for its owner: Go/C++ provider/generation mapping review,
  Qt distribution identity/licensing/artifacts, cppcheck/Buf
  asset qualification, and any adapter that cannot use the common runner.
- M22 quality adapters deferred to M24 under O32 (see WP3); M23 consumes the
  Scala/Scalafix managed route under O31.
- Non-Linux hosts, remote execution, and clean external-consumer
  evidence are unproven (same gap class as M00/M12/M14/M16/M17).
- No supported claim; CUDA/assembly/`cc_shared_library`/cgo scope stays
  unresolved and fails closed.
