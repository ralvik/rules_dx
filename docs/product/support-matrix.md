# Support Matrix

## Status Lifecycle

`Planned` → `Seed-host-delivered` → `Platform-qualified` → `Supported`.

`Planned` is accepted scope only and claims no delivery. `Seed-host-delivered` is implemented and
verified on the Linux x86_64 seed host only. No `Planned` cell is `Seed-host-delivered`:
as-built delivery is owned by the [verification matrix](../testing/verification-matrix.md),
not by this matrix. `Implementation status: implemented` / `Status: implemented` notes elsewhere
describe their owning contract's scope, not promotion under this lifecycle. `Platform-qualified` adds required-platform evidence per
[ADR 0014](../decisions/0014-tested-platform-release-stack.md). A `Platform-qualified`
cell never shares its status with a provisional backend without an explicit `provisional`
qualifier: the provisional backend stays an exception, never a qualified claim, and
`dx_tools` artifact delivery under #616 does not qualify a provisional
compiler/SDK backend. `Supported`
adds release evidence and is owned by this matrix; no cell is currently
`Supported`. The per-cell promotion checklist (tag hygiene, versioning, platform
plus consumer plus release evidence) is owned by the
[promotion checklist](promotion-checklist.md) and qualified seed-only under
closed #611 with fixture evidence in
`tools/ci/tests/fixtures/promotion_checklist/pins.bzl` via
`bazel run //tools/ci:promotion_checklist_qualification`.

## Unqualified Platforms

Only the Linux x86_64 seed host plus Linux arm64 glibc native (issue
#410) plus the two Linux static-musl profiles (#411) plus macOS
arm64 native (#412) plus macOS x86_64 best-effort native (issue
#413) plus Windows x86_64 MSVC-compatible native (#414) are
delivered today. Every other host, including the remaining
[required platforms](../decisions/0014-tested-platform-release-stack.md#required-platforms),
is unqualified: `dx` refuses cleanly with `unsupported_platform`, naming the
host and pointing here and at ADR 0014, before any Bazel work starts. It
never presents partial execution on an unqualified host as success. The
refusal reads the same qualified-host list that platform evidence extends,
so a host flips on exactly when its evidence lands. Platform qualification is delivered seed-only under closed #410-#414 with CI matrix closed #415; release evidence landed for every required host under closed #803 plus #804 plus #805 plus #807 with best-effort macOS x86_64 exempt under closed #806 moot (process #808).
Platform qualification was tracked under #298 (closed): the Linux
x86_64 seed host plus Linux arm64 native plus the two static-musl profiles
plus macOS arm64 native plus macOS x86_64 best-effort native plus Windows
x86_64 MSVC-compatible native are delivered and tested; every other
required host needs pins, hosts, floors, JDK/SDK/CRT identities, qualified
routes, per-cell coverage, and consumer plus release evidence under its
per-host successor. The CI host matrix across these hosts is pinned by
`bazel run //tools/ci:ci_matrix_qualification` (closed #415).

Per-required-host qualification state (V1 status from ADR 0014; evidence
dimensions per closed #298; exact pins, hosts, floors, and SDK/CRT identities
remain owned by issues #410-#414 and are not pinned here):

| Host | V1 status | Qualification evidence | Current state |
| --- | --- | --- | --- |
| Linux x86_64 glibc | Required, primary bootstrap | Pins/hosts/floors, JDK/SDK/CRT identities, qualified native plus cross routes, per-cell coverage, consumer plus release evidence | Seed-host-delivered (CI: ubuntu-latest; artifacts: linux_x86_64; coverage: seed cell; consumer self-call: test-disabled on linux_x86_64 plus linux_arm64 plus macos_arm64 plus macos_x86_64 plus windows_x86_64, #408 plus Phase 1 #607 coverage superset) |
| Linux arm64 glibc | Required | Same dimensions as above, native workflow (not cross-only) | Platform-qualified (#410; CI: ubuntu-24.04-arm; dx_tools linux_arm64 artifacts delivered (#616); coverage: arm64 cell; consumer self-call: test-disabled, #408 plus Phase 1 #607 coverage superset; release evidence: linux_arm64 sbom-provenance delivered (#803; every required host landed under closed #803 plus #804 plus #805 plus #807 with best-effort macOS x86_64 exempt under closed #806 moot, process #808)) |
| Linux x86_64/arm64 static musl | Required profiles | Same dimensions; static native closure; dynamic musl explicitly out of scope | Platform-qualified (#411; provisional-backend exception: hermetic-llvm static-only musl targets stay provisional, so the musl C/C++ backend itself is not qualified; CI: ubuntu-latest plus ubuntu-24.04-arm cross-build with per-profile bazel-musl-* cache scopes; Rust musl std via extra_target_triples with exec-platform tools for build scripts/proc macros and target musl libs for apps, prebuilt glibc libs never musl-compatible by linker change alone; coverage: musl x86_64 plus musl arm64 cells with no union; consumer self-call: test-disabled plus musl jobs (#408 plus Phase 1 #607 coverage superset); release evidence: linux_x86_64_musl plus linux_arm64_musl sbom-provenance delivered (#804; every required host landed under closed #803 plus #804 plus #805 plus #807 with best-effort macOS x86_64 exempt under closed #806 moot, process #808)) |
| macOS arm64 | Required | Same dimensions; pinned acquired SDK (SDK version is not the deployment floor) | Platform-qualified (#412; provisional-backend exception: hermetic-llvm Apple-SDK backend provisional (backend itself not qualified); CI: macos-14 with bazel-macos-arm64- cache scope; pinned upstream toolchains with immutable lazy fetch; host-installed SDK fallback never approved; Apple-SDK handling leaks no secrets and needs no interactive acceptance; dx_tools macos_arm64 artifacts delivered (#616); coverage: macos arm64 cell with no union; consumer self-call: test-disabled, #408 plus Phase 1 #607 coverage superset; release evidence: macos_arm64 sbom-provenance delivered (#805; every required host landed under closed #803 plus #804 plus #805 plus #807 with best-effort macOS x86_64 exempt under closed #806 moot, process #808)) |
| macOS x86_64 | Best-effort | Same dimensions; pinned acquired SDK (SDK version is not the deployment floor); best-effort non-blocking | Platform-qualified (#413; provisional-backend exception: hermetic-llvm Apple-SDK backend provisional (backend itself not qualified); CI: macos-15-intel with bazel-macos-x86_64- cache scope; `macos-13` retired December 2025, `macos-15-intel` until August 2027; pinned upstream toolchains with immutable lazy fetch; host-installed SDK fallback never approved; Apple-SDK handling leaks no secrets and needs no interactive acceptance; dx_tools macos_x86_64 artifacts delivered (#616); coverage: macos x86_64 cell with no union; consumer self-call: test-disabled, #408 plus Phase 1 #607 coverage superset; release evidence exempt as best-effort non-blocking (closed #806 moot; every required host landed under closed #803 plus #804 plus #805 plus #807, process #808); gaps never block required-host release) |
| Windows x86_64 MSVC-compatible | Required | Same dimensions; hermetic acquisition plus MSVC compatibility gates unchanged; explicit EULA acceptance required, never automatic | Platform-qualified (#414; provisional-backend exception: toolchains_msvc clang-cl/Microsoft-STL backend provisional (backend itself not qualified); CI: windows-latest with bazel-windows-x86_64- cache scope plus shell bash; pinned upstream toolchains with immutable lazy fetch; explicit EULA acceptance required never automatic, usage vs redistribution reviewed separately, merely adding the module requires no acceptance and fetches no restricted payloads; installed Build Tools fallback never approved; prebuilt-MSVC interop fixtures with explicit STL/CRT/linker/library combos incl mixed Rust/C/C++ qualify host-to-target plus target execution separately, compiler-target availability alone is not proof; manifest/path/ABI gaps closed with fixtures, linux corpus qualified seed-only under #499; dx_tools windows_x86_64 artifacts delivered (#616); coverage: windows x86_64 cell with no union; consumer self-call: test-disabled, #408 plus Phase 1 #607 coverage superset; release evidence: windows_x86_64 sbom-provenance delivered (#807; every required host landed under closed #803 plus #804 plus #805 plus #807 with best-effort macOS x86_64 exempt under closed #806 moot, process #808)) |
| Windows arm64 | Out of v1 scope | Not a claim of impossibility | Unqualified: clean `unsupported_platform` refusal |

No cell below is `Supported`: promotion requires platform plus consumer plus
release evidence per the [status lifecycle](#status-lifecycle),
enforced by `bazel run //tools/ci:supported_evidence_gate`.

## Application Foundations

`Planned` below means accepted scope only with no delivery claimed; as-built delivery
per capability is owned by the [verification matrix](../testing/verification-matrix.md).
A `Planned` cell whose capability maps to an `Open` verification cell is scope with open
implementation: `Environment` maps to `Env/codegen` (`Open` for every language);
`Format`/`Lint`/`Typecheck` for adapter-less or region-only rows map to `Layer-2 matrix`
(`Open (adapter-less)` for Go, Scala, C#, F#, C++ and `Open (regions)` for
Vue, Svelte, Astro, MDX); `Dependencies` and framework `Build`/`Test` composition for Vue,
Svelte, Astro, MDX map to `Depcheck` (`Open`) and `Examples` (`Open (composition)`). Rust,
Python, JavaScript, and TypeScript `Build`/`Test`/`Dependencies`/`Generate`/`Coverage`/
`Format`/`Lint`/`Typecheck` delivery follows the `Delivered` verification cells for corpus
dogfood, Layer-2, generation, examples, E2E contract, and depcheck; their `Environment`/`IDE`
cells stay scope-only while `Env/codegen` stays `Open`. Every cell in the table below stays
`Planned` (or `Planned: ...`); none is `Seed-host-delivered` or higher, even where the
verification matrix shows `Delivered` for the capability: verification `Delivered` is
seed-host layer evidence, not support-matrix promotion. No `Planned` cell implies
remaining-required-platform,
external-consumer, or trusted-builder release evidence; see
`../../CHANGELOG.md`. `Planned: feasibility` means evaluation
under [first-release admission](scope.md#first-release-admission):
admit qualifying capabilities, or record an approved disposition. Additional foundations may be
deferred when completion needs substantial infrastructure; required quality tools are independent.
Remaining upstream mappings are tracked in open work under #796-#800 (successors to closed #416-#420) with foundation mappings qualified seed-only under closed #476-#489.
`Not planned` records an existing capability gap reassessed during delivery, not permission to defer a
qualifying integration. `Deferred beyond v1` records an evidence-backed deferral of an additional
foundation under [first-release admission](scope.md#first-release-admission); the row's quality-tool
cells stay in force and are unaffected by the deferral. `External` identifies an
existing Bazel/Bzlmod capability rather than a `rules_dx` support claim. `N/A` means the
capability does not apply to that source family. The repository [README](../../README.md) owns the
current project status; no cell is currently `Supported`. Status cells are owned by
this matrix, and promotion to `Supported` requires release evidence.

`Audit` means per-language source audit over declared source owners, distinct from the
verification-matrix `Audit/update` column which records repo-wide `dx audit`/`dx update`
live execution (ecosystem resolvers, advisory matching, secrets wiring). A source-`Audit`
cell of `Not planned` or `Planned: audit tools (open work)` is consistent with a `Delivered`
ecosystem `Audit/update` cell: Rust, JavaScript, TypeScript, Vue, Svelte, Astro, and MDX
source audit is `Not planned` while ecosystem audit/update is delivered repo-wide, and Python
source-audit tooling is qualified seed-only under issue #801 (successor to closed #613; Ruff S selected, Bandit excluded) while ecosystem audit/update wiring is delivered.
Python source-audit tooling is tracked under issue #801 with fixture evidence in
`python/tests/fixtures/python_audit/pins.bzl` via `bazel run //tools/ci:python_audit_qualification`;
closed #512 stays taxonomy-only and owns no Python audit tool.
Ecosystem dependency-vulnerability audit
coverage is separate; see the [audit contract](../cli/commands/audit-update-bazel.md). This matrix makes no
ecosystem audit-tool or dependency-source claim beyond that contract.

The following core and named framework foundations are required for v1 on every
[required platform](../decisions/0014-tested-platform-release-stack.md#required-platforms).
They are not eligible for the additional-foundation deferral policy.

| Language | Build | Test | Dependencies | Generate | Environment | IDE | Coverage | Format | Lint | Typecheck | Audit |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Rust | Planned | Planned | Planned: Cargo lock | Planned | Planned: native tools | Planned: rust-analyzer/flycheck | Planned | Planned: rustfmt | Planned: Clippy | Planned: compiler diagnostics | Not planned |
| Python | Planned | Planned: pytest | Planned: uv lock | Planned | Planned: `.venv` | Planned: interpreter/imports | Planned | Planned: Ruff | Planned: Ruff, pydoclint; flake8/pylint opt-in | Planned: Ty | Planned: audit tools (qualified seed-only under issue #801 with Ruff S selected via `python/tests/fixtures/python_audit/pins.bzl` with `python_audit.expected` via `bazel run //tools/ci:python_audit_qualification`; Ruff S via pinned Ruff 0.16.7 standalone artifact with S opt-in, curated audit stays empty with explicit disablement, Bandit excluded; platform plus consumer plus release #808 owned gap) |
| JavaScript | Planned | Planned: Jest | Planned: pnpm lock | Planned | Planned: `node_modules` | Planned: Node/modules | Planned | Planned: Biome default, Prettier available | Planned: Biome default, ESLint available | N/A | Not planned |
| TypeScript | Planned | Planned: Jest | Planned: pnpm lock | Planned | Planned: `node_modules` | Planned: TypeScript/Node | Planned | Planned: Biome default, Prettier available | Planned: Biome default, ESLint available | Planned: `tsc` | Not planned |
| Vue | Planned | Planned | Planned | Planned | Planned | Planned | Planned | Planned | Planned | Planned | Not planned |
| Svelte | Planned | Planned | Planned | Planned | Planned | Planned | Planned | Planned | Planned | Planned | Not planned |
| Astro | Planned | Planned | Planned | Planned | Planned | Planned | Planned | Planned | Planned | Planned | Not planned |
| MDX | Planned | Planned | Planned | Planned | Planned | Planned | Planned | Planned | Planned | Planned | Not planned |

Complete-foundation claims are made only after build, test, dependency, generation, environment,
IDE, coverage, platform, and external-consumer evidence passes. Quality is independently testable
and lazy before foundation completion, but `Supported` promotion occurs only during release
qualification. Generic framework cells intentionally do not select exact tools,
providers, or region mappings; those choices are tracked in
open work under #796-#800 (successors to closed #510 and closed #416-#420).

## Minimal Required Core

This section is the minimal required core/framework inventory. It records
what is in, which provisional upstream each item builds on, which tracker
owns each remaining mapping, and the current state. Exact pins, hosts,
and floors are open; no version
below is a release pin. Ownership: the sole repository maintainer owns every row until
maintenance is explicitly delegated. Effort is estimated from
qualification evidence as each tracked item lands; no person-hour figures are frozen here.

Quality-tool defaults posture, doc-wide (core rows above and admitted languages below):
enabled-tool membership follows the [curated baseline](../tools/tool-baseline.md#curated-differences),
including its explicit opt-ins; naming an integration here does not enable it by default.
Enabled tools use their pinned native defaults unless an applicable checked-in native config
supplies policy, per [Native Configuration](../quality/native-configuration.md#authority),
not a hidden `rules_dx` preset. Owning trackers qualify exact versions and configurations with
fixture evidence; provisional notes below do not select additional defaults or native presets.

| Foundation | Provisional upstream basis | Open mappings | Tracking |
| --- | --- | --- | --- |
| Rust application | `rules_rs` + pinned patched `rules_rust` ([ADR 0013](../decisions/0013-rust-javascript-typescript-foundations.md)) | Providers/Gazelle/integration pinned; kept CC opt-out linker pinned (#471); bindgen LLVM-22-vs-23 pinned (#473); CXX graph identity decided single-graph (#474); exact-target discovery qualified seed-only (#475); native gaps: shell-env default | Pinned (`bazel run //tools/ci:foundation_maps`, #470; `bazel run //tools/ci:cc_optout_qualification`, #471; `bazel run //tools/ci:bindgen_qualification`, #473; `bazel run //tools/ci:cxx_identity_qualification`, #474; `bazel run //tools/ci:exact_target_qualification`, #475); native gaps under #472 |
| Python application | `aspect_rules_py` 2.x ([ADR 0010](../decisions/0010-python-foundation.md)) | Mappings | Pinned (`bazel run //tools/ci:foundation_maps`) |
| JavaScript/TypeScript application | `aspect_rules_js`/`aspect_rules_ts`, `aspect_rules_jest` ([ADR 0013](../decisions/0013-rust-javascript-typescript-foundations.md)) | Wrappers/Gazelle | Pinned (`bazel run //tools/ci:foundation_maps`) |
| JS/TS quality | Biome default; ESLint, Prettier available; `tsc` diagnostic-only | Mappings | Pinned (`bazel run //tools/ci:foundation_maps`) |
| Python quality | Ruff, Ty, pydoclint (Bandit excluded from v1 by [ADR 0019](../decisions/0019-first-release-additional-foundations.md)) | Mappings, Ty | Pinned (`bazel run //tools/ci:foundation_maps`) |
| Vue / Svelte / Astro / MDX | Named upstream adapters per framework | Adapter mappings plus composition evidence qualified seed-only under closed #510 | Qualified (`bazel run //tools/ci:layer2_opens_qualification`, `quality/tests/fixtures/layer2_opens/pins.bzl` with `layer2_opens.expected`, adapter-less as pass rejected) |
| Generation transport | Declared versioned manifest artifact (`GenerationManifest` in `generation/result.proto`, projected by `ProjectedManifest` in `cli/cli/src/generate.rs`) with explicit path/label/pattern scope | Implemented | Shipped |
| Target resolution | All-direct-owners query strategy ([Target Resolution](../cli/target-resolution.md), pinned by resolver fixtures) | Implemented | Shipped |
| Quality core/result contract | Internal Protobuf + NDJSON output | Core mappings qualified seed-only under #511 | Qualified (`bazel run //tools/ci:result_contract_qualification`, `quality/tests/fixtures/result_contract/pins.bzl` with `result_contract.expected`, bare contract rejected) |
| Quality family taxonomy | Frozen class-to-family taxonomy plus curated execution | Taxonomy execution qualified seed-only under closed #512 with promotion gaps linked under #802 | Qualified (`bazel run //tools/ci:quality_taxonomy_qualification`, `quality/tests/fixtures/quality_taxonomy/pins.bzl` with `quality_taxonomy.expected`, taxonomy doc only rejected; deferred delivery under #796 plus #797 plus #798 plus #799 plus #800 with remaining owned under ADR 0019, digest policy plus platform plus consumer plus release linkage under #802, promotion gaps linked under #802) |
| Required platforms | [Required-platform table](../decisions/0014-tested-platform-release-stack.md#required-platforms) | Pins, hosts, floors | Pins, hosts, floors qualified seed-only under closed #410-#414 and closed #500; release evidence landed for every required host under closed #803 plus #804 plus #805 plus #807 with best-effort macOS x86_64 exempt under closed #806 moot (process #808) |
| Coverage gate | Instrumentation-first; behavioral fallback only on proof | Resolved in [coverage](../testing/README.md#coverage) | Enforced by CI |
| Consumer CI | Reusable workflow + caller template | Delivered; verification open | Shipped |
| Repository workflows | Codegen/env/setup implemented; `dx update` plus `dx audit` live execution delivered | Codegen pairs plus platform plus consumer plus release | Codegen platform plus consumer plus release evidence qualified under #787 (`env/tests/fixtures/env_codegen/pins.bzl` via `bazel run //tools/ci:env_codegen_qualification`; per-required-host symlink-only plus refusal, adopt-consumer, release checklist linkage; #751 plus #752 plus #753 stay open and out of scope for #787 plus #788); pairs evolution onboarding qualified under #788 with checklist plus per-pair fixtures plus qualification coverage (successors to closed #506); audit/update delivered |

File-family quality defaults are qualified seed-only under #489 with modfmt plus gherkin/xml
resolved seed-only under #582 (`bazel run //tools/ci:file_family_defaults_qualification` with
`quality/tests/fixtures/file_family_quality/pins.bzl` over upstream built-in defaults with no hidden preset;
cue v0.17.1 plus jsonnetfmt v0.22.0 plus pkl 0.32.1 plus terraform v1.16.1 plus djlint v1.45.0 plus Stylelint
17.14.1 plus Prettier 3.9.6 with prettier-plugin-sql 0.15.1 plus prettier-plugin-gherkin 4.0.0 plus
@prettier/plugin-xml 3.4.2 plus modfmt v0.4.0 from github.com/joshdk/modfmt plus yamlfmt v0.21.0 plus yamllint
1.38.0 plus keep-sorted v0.10.0; whole-file rewrite versus check-only per tool with no auto-supplied preset,
suffix inference rejected with registry-owned applicability, beyond-default switches rejected;
file-family adapters delivered under #800 (successor to closed #420); `protobuf`/`qml` adapters delivered under #799 (successor to closed #419),
never double-claimed; platform plus consumer plus release evidence stays owned gap; no Supported claim).

Structured quality defaults are qualified seed-only under issue #488 (`bazel run //tools/ci:structured_defaults_qualification` with
`quality/tests/fixtures/structured_quality/pins.bzl` over upstream built-in defaults with no hidden preset;
buf `STANDARD` plus qml ini as upstream built-in defaults; `protobuf`/`qml` adapters delivered under #799,
never double-claimed; platform plus consumer plus release evidence stays owned gap; no Supported claim).

Required-core Rust providers/Gazelle/integration are pinned (#470); kept CC
opt-out linker is pinned seed-only (#471); bindgen LLVM-22-vs-23 compat is
qualified seed-only (#473); CXX graph identity is pinned
seed-only (#474); exact-target discovery is qualified seed-only
(#475); the one remaining native
gap below stays owned under #472; Vue/Svelte/Astro/MDX
adapter mappings plus composition evidence stay open under #796-#800 (successors to closed #510) with fixture evidence
qualified seed-only under closed #510
(`quality/tests/fixtures/layer2_opens/pins.bzl` with `layer2_opens.expected`
via `bazel run //tools/ci:layer2_opens_qualification`, adapter-less as pass
rejected, no Supported claim). Python mappings
plus Ty and JS/TS wrappers/Gazelle plus quality mappings are pinned (see below).
Qualified mappings are pinned by
`bazel run //tools/ci:foundation_maps` with owning qualification in
[Generation](../generation/README.md#language-mapping-qualification),
[Environments](../environments/README.md#language-mapping-qualification),
[Tools](../tools/README.md#language-mapping-qualification), and
[Framework adapters](../generation/framework-adapters.md#framework-mapping-qualification):
Rust provider plus Gazelle maps with fixtures (`rust/rules/defs.bzl` preserving
`CrateInfo`/`DepInfo`/`TestCrateInfo`/`CcInfo` plus `QualitySourcesInfo`, proven by
`rust/rules/wrapper_tests.bzl` conformance over `rust/tests/fixtures/hello/`; Gazelle
kinds/loads in `gazelle/rust/lang.go` with cargo plus source-only plus merge plus
native-config goldens; provider-derived env plan in `rust/env/plan.bzl` pinned by
`rust/env/plan_tests.bzl`; hello plus `Cargo.lock` plus `cargo-bazel-lock.json` lock
authority, #470),
build-script hermetic defaults (`use_cc_toolchain = True`, `use_default_shell_env = False`,
`emit_warnings = True` in `gazelle/rust/lang.go`, proven by `gazelle/rust/lang_test.go`),
Ty provenance plus `ty` typecheck adapter mapping (`quality/artifacts/ty.linux_x86_64.bzl` plus per-host siblings under #616,
`quality/adapters.bzl`, `quality/adapter/src/parsers/ty.rs`), JS/TS quality adapter mappings
(`biome`/`eslint`/`prettier`/`tsc` in `quality/adapters.bzl` plus their parsers), and
framework composition (`examples/mixed/hello/` plus `gazelle/mixed/`). Kept CC
opt-out linker failure path is pinned seed-only under #471 (pure-Rust
`rust/tests/fixtures/cc_optout/` with kept opt-out, sysroot `rust-lld`
fallback, `bazel run //tools/ci:cc_optout_qualification`). Bindgen LLVM-22-vs-23
compat is qualified under #473 with the LLVM-22 parser baseline vs LLVM-23
target pinned plus the standalone/build-script fixture pair
(`bazel run //tools/ci:bindgen_qualification`). CXX graph identity
is pinned seed-only under #474 (single crate_universe `crates` graph
with `cxx == cxxbridge-cmd == 1.0.200` and `@crates//:cxxbridge-cmd`, never a
`cxx.rs` second graph, proven by `rust/tests/fixtures/cxx_identity/` via
`bazel run //tools/ci:cxx_identity_qualification`). Exact-target discovery is
qualified seed-only under #475 (resolver-owned exact labels to upstream
`TARGETS`, `Path`/`Buildfile` widening plus project-owned graph plus
`RustAnalyzerInfo` rejected, hello exact-isolation pair plus
`rust/tests/fixtures/discovery/pins.bzl` via
`bazel run //tools/ci:exact_target_qualification`). Cargo metadata is
qualified seed-only under #502 (`rust/tests/fixtures/cargo_metadata/pins.bzl`
with the `Cargo.toml` plus `cargo_metadata.expected` pair via
`bazel run //tools/ci:cargo_metadata_qualification`, ad-hoc metadata rejected).
The remaining native gap
(global shell-env False versus
annotation extension decided hermetic under #472 with global `False`
in `.bazelrc` and narrow per-crate opt-in at zero opt-ins) stays
owned under #472 per the [native plan](../native-toolchains.md#qualification-questions-and-delivery).
No `Supported` claim until platform plus consumer plus release evidence passes.

Required platforms and the quality-tool baseline (minus excluded Swift) frame
delivery of this inventory. Admitted additional foundations are tracked in
open work under #796-#800 (successors to closed #416-#420) with foundation mappings
qualified seed-only under closed #476-#489;
remaining feasibility detail stays in the
[feasibility review](#initial-feasibility-review) for qualification.
Swift is an evidence-backed v1 exclusion, not a pending assessment. Effort
is estimated from qualification evidence as
each tracked item lands; no person-hour figures are frozen here.

## Additional V1 Foundations

Admit/defer/exclude outcomes are decided by
[ADR 0019](../decisions/0019-first-release-additional-foundations.md), with Ruby
reconsideration decided by
[ADR 0030](../decisions/0030-ruby-foundation-reconsideration.md), with PowerShell
reconsideration decided by
[ADR 0031](../decisions/0031-powershell-foundation-reconsideration.md), and
summarized in [Admitted To V1](#admitted-to-v1) and
[Deferred Beyond V1](#deferred-beyond-v1).
The review below records upstream evidence only. Include complete low-cost
upstream-backed foundations; substantial missing integration may justify an explicit approved
deferral under the admission policy. Each foundation is tracked with applicable build, test, dependency, generation,
environment, IDE, coverage, and quality cells with named upstreams and evidence. No row implies
that a ruleset has already been selected or that every capability/platform is feasible.
This is a minimum inventory, not an exhaustive list of eligible languages.

### Admitted To V1

Java, Kotlin, C#, F#, Go, C/C++, and Scala application foundations are admitted
to v1 scope by [ADR 0019](../decisions/0019-first-release-additional-foundations.md), on the evidence in the
[candidate review](#initial-feasibility-review): each has
an active Bzlmod-published upstream ruleset with a concrete dependency-lock
and toolchain story. Their quality integration is v1 scope with the defaults in
the [disposition table](#deferred-beyond-v1);
exact versions, rule sets, and adapter mappings are tracked in
qualified seed-only under closed #485-#489 with adapter delivery open under #796-#800 (successors to closed #416-#420). The managed Scala
route decision was recorded 2026-09-13.
Unresolved cells block qualification; moving an admitted foundation out later
requires a new evidence-backed decision.

Admitted additional foundations (Go, C/C++, Java, Kotlin, Scala, C#, F#) stay open
under #796-#800 (successors to closed #416-#420) with foundation mappings qualified
seed-only under closed #476-#484 plus closed #485-#488: per-foundation exact upstream versions, rulesets,
adapter mappings, lock wiring, test runners, and quality tools remain qualification work. Qualified mappings
are pinned by `bazel run //tools/ci:foundation_maps` with owning qualification in
[Generation](../generation/README.md#language-mapping-qualification),
[Environments](../environments/README.md#language-mapping-qualification),
[Tools](../tools/README.md#language-mapping-qualification), and the
[native plan](../native-toolchains.md#qualification-questions-and-delivery):
wrappers preserving upstream providers plus `QualitySourcesInfo`, Gazelle extensions,
env plans, hello builds, and lock authority (Maven `maven_install.json` plus fail-closed
qualified seed-only under #481, Paket plus `paket.main` qualified seed-only
under #482, Go `go.mod`/`go.sum` qualified seed-only under #483 with hello staying stdlib-only, C/C++ none), with test runners (JUnit 6.1.3 plus 5.14.x fallback qualified seed-only under #476, xUnit v3 4.0.0
mapping via xunit fixtures qualified seed-only under #477, `go test`
qualified seed-only under #478, GoogleTest v1.18.0 plus C++17 floor
qualified seed-only under #479, ScalaTest qualified seed-only under issue
#480, plain `cc` executables, plain `csharp`/`fsharp` hello executables) and
classification-only quality families. Dependency hygiene (lockfile-consistency plus
declared-dependency usage with category, exception, and obsolete checks) is qualified for
all admitted languages in `tools/depcheck/` (#22; remaining opens under #796-#800, successors to closed #510)
with native authorities (go.sum,
`maven_install.json`, `paket.lock`, per-archive sha256) and focused fixtures. Remaining gaps
(quality adapters qualified under closed #307 with
deferred implementation
owned by ADR 0019, C/C++ MSVC interop plus SDK licensing)
stay owned under issues #476-#484 plus #485-#488 (JUnit 6.1.3 plus 5.14.x fallback
qualified seed-only under #476 via `bazel run //tools/ci:junit_qualification`;
xUnit v3 4.0.0 qualified seed-only under #477 via
`bazel run //tools/ci:xunit_qualification`;
`go test` qualified seed-only under #478 via
`bazel run //tools/ci:gotest_qualification`;
GoogleTest v1.18.0 plus C++17 floor qualified seed-only under #479 via
`bazel run //tools/ci:googletest_qualification`;
ScalaTest 3.2.20 qualified seed-only under #480 via
`bazel run //tools/ci:scalatest_qualification`;
Maven `maven_install.json` plus fail-closed repin qualified seed-only under #481 via
`bazel run //tools/ci:maven_lock_qualification`;
Paket files plus sha512 qualified seed-only under #482 via
`bazel run //tools/ci:paket_qualification`;
Go `go.mod`/`go.sum` qualified seed-only under #483 via
`bazel run //tools/ci:godeps_qualification`).
No `Supported` claim until platform plus consumer plus release
evidence passes.

### Deferred Beyond V1

Ruby and PowerShell application foundations are deferred beyond v1 by
[ADR 0019](../decisions/0019-first-release-additional-foundations.md), with
evidence in the [candidate review](#initial-feasibility-review):
Ruby's fragmented ruleset maintenance ownership and gem/bundler packaging
effort, and PowerShell's single young execution-only upstream with unproven
generation, dependency, environment, and IDE stories, exceed the low-cost
hermetic integration bar. Ruby reconsideration under issue #777 keeps the
deferral per [ADR 0030](../decisions/0030-ruby-foundation-reconsideration.md):
consolidated `rules_ruby` 0.28.0 plus portable-Ruby Linux/macOS-only plus
Windows RubyInstaller fallback plus open git-gem and checksum gaps plus missing
Gazelle/env/IDE/lock stories still exceed the bar. PowerShell reconsideration
under issue #778 keeps the deferral per
[ADR 0031](../decisions/0031-powershell-foundation-reconsideration.md):
single young execution-only `rules_powershell` 0.2.0 plus missing
Gazelle/dependency/env/IDE/lock stories still exceed the bar. Deferred foundations'
quality-tool cells (RuboCop, StandardRB, PSScriptAnalyzer) stay in force; a foundation
deferral removes no baseline tool. Reconsideration after v1 requires a new
scope decision. The deferred/excluded record is decided by
[ADR 0019](../decisions/0019-first-release-additional-foundations.md) plus
[ADR 0030](../decisions/0030-ruby-foundation-reconsideration.md) for Ruby plus
[ADR 0031](../decisions/0031-powershell-foundation-reconsideration.md) for PowerShell (Ruby plus
PowerShell deferred, Swift plus Bandit excluded, host-toolchain fallback never approved;
Ruby reconsidered under #777 still deferred, PowerShell reconsidered under #778 still deferred).
Qualified record is pinned by `bazel run //tools/ci:foundation_maps` with owning
qualification in [Generation](../generation/README.md#language-mapping-qualification),
[Environments](../environments/README.md#language-mapping-qualification),
[Tools](../tools/README.md#language-mapping-qualification),
[tool baseline](../tools/tool-baseline.md), [tool acquisition](../tools/tool-acquisition.md),
and the parity gate (`quality/parity_tests.bzl`): no `ruby/`, `powershell/`, or `swift/`
foundation dirs, wrappers, Gazelle extensions, env plans, hello builds, or `MODULE.bazel` deps;
`ruby`/`powershell` classes classified with no adapter claim (`quality/adapters.bzl` plus
`PARITY_DEFERRED` with ADR 0019); retained cohorts (RuboCop/StandardRB via
release-assembled Ruby closure, PSScriptAnalyzer via exact-module plus portable PowerShell
runtime); Swift/SwiftFormat plus Bandit exclusions with host-toolchain fallback never approved.
Remaining gaps (bundle contents, lock inputs, module/runtime identities, console-parse versus
library-API binding, and per-tool adapter mappings qualified under closed #420 with deferred
implementation owned by ADR 0019; Ruby reconsideration decided under #777 by ADR 0030 still
deferred, PowerShell reconsideration decided under #778 by ADR 0031 still
deferred,
adapter execution under #800) stay decided by ADR 0019 plus ADR 0030 for Ruby plus ADR 0031 for PowerShell. No `Supported` claim until platform plus
consumer plus release evidence passes.

| Language | Application foundation | Format | Lint, typecheck, or audit |
| --- | --- | --- | --- |
| Go | Planned | Planned: gofumpt | Planned: staticcheck, govet |
| C and C++ | Planned | Planned: clang-format | Planned: clang-tidy, cppcheck |
| Java | Planned | Planned: google-java-format | Planned: PMD, Checkstyle, SpotBugs, Error Prone |
| Kotlin | Planned | Planned: ktfmt | Planned: ktlint, detekt |
| Scala | Planned | Planned: scalafmt | Planned: scalafix |
| C# | Planned | Planned: CSharpier | Planned: Roslyn CA analyzers (SDK) |
| F# | Planned | Planned: Fantomas | Planned: FSharpLint |
| Ruby | Deferred beyond v1 (reconsidered under #777 still deferred per ADR 0030) | Planned: feasibility | Planned: RuboCop, StandardRB |
| PowerShell | Deferred beyond v1 (reconsidered under #778 still deferred per ADR 0031) | Planned: feasibility | Planned: PSScriptAnalyzer |
| Swift | Not planned | Not planned | Not planned |

`Planned` in the table above means scope admitted to v1 by ADR 0019 with no delivery
claimed; delivery follows the [verification matrix](../testing/verification-matrix.md).
Every cell in the table above stays `Planned` (or `Deferred beyond v1` / `Not planned`
where marked); none is `Seed-host-delivered` or higher, even where the verification
matrix shows `Delivered` for the capability: verification `Delivered` is seed-host
layer evidence, not support-matrix promotion. Admitted `Format`/`Lint` cells map to `Layer-2 matrix Open (adapter-less)` (no adapter
claims them yet; JVM delivered under #796, Native Layer-2 delivered under #798, and Scala + .NET delivered under #797), `Environment`/`IDE` dimensions map to `Env/codegen Open`,
and per-language source audit lumped in the third column is distinct from ecosystem
`Audit/update Delivered`.

### Initial Feasibility Review

The approved review order starts with Go and C/C++. C/C++ initially assesses Bazel-native
projects; CMake/Meson integration is a separate review, not an automatic-migration promise.
This prioritization admits or defers no foundation and changes no tracking dependencies.

The following are upstream documentation/source observations, not executed qualification evidence.
Status cells are owned by this matrix; provisional toolchain and backend
choices are owned by the [native qualification plan](../native-toolchains.md). Bounded remediation is qualified seed-only under #505 (`cc/tests/fixtures/remediation_bounds/pins.bzl` via `bazel run //tools/ci:remediation_bounds_qualification`, Do not implement missing infra merely to fill cross-product with unbounded fork rejected); identified contract conflicts stay tracked there before affected implementation:

- [rules_go](https://github.com/bazel-contrib/rules_go) and
  [Gazelle](https://github.com/bazel-contrib/bazel-gazelle) offer SDK acquisition, package build/test,
  module dependencies, and generation. The approved narrow Go exception preserves package-level
  tests and declared platform/build constraints under the
  [generation contract](../generation/common.md#ownership-and-naming); exact mappings remain open
  (open work under #798, successor to closed #510).
  Source-only module identity, strict dependency resolution, and cgo/race scope are resolved
  seed-only under #789 (`go/tests/fixtures/cgo/` plus `env/tests/fixtures/env_plugins_cgo/`
  via `bazel run //tools/ci:env_plugins_cgo_qualification`); exact mappings remain open
  under #798.
- The documented [Go editor driver](https://github.com/bazel-contrib/rules_go/blob/v0.63.0/docs/editors.md)
  invokes Bazel. That automatic integration is approved under
  [environment refresh](../environments/environment.md#ownership-and-refresh), following the
  cross-language automatic-first policy rather than building a static snapshot alternative.
  Exact-target isolation, failure propagation, cgo IDE behavior, and every host workflow still need
   qualification; upstream explicitly does not guarantee cgo completion (pure-Go boundary plus
   explicit cgo exception pinned by `env/tests/fixtures/env_plugins_cgo/` via
   `bazel run //tools/ci:env_plugins_cgo_qualification`, closed #587; cgo scope resolved
   seed-only under #789 with `go/tests/fixtures/cgo/`).
- [rules_cc](https://github.com/bazelbuild/rules_cc) supplies build rules, not a hermetic compiler
  distribution. [hermetic-llvm v0.8.19](https://github.com/hermeticbuild/hermetic-llvm/tree/v0.8.19)
  is the inspected release of the preferred Linux/macOS backend. Its released Windows route uses
  MinGW-w64/UCRT, which does not satisfy the approved Windows baseline by itself. MSVC-compatible
  interoperability and Windows coverage need qualification; Apple/Microsoft SDK acquisition rights
  are qualified seed-only under #496 (`cc/tests/fixtures/acquisition_rights/pins.bzl` via
  `bazel run //tools/ci:acquisition_rights_qualification`, usage vs redistribution reviewed separately).
  No host-installed SDK fallback is approved.
- [gazelle_cc v0.6.0](https://github.com/EngFlow/gazelle_cc/tree/v0.6.0) and
  [Hedron's compilation-command extractor](https://github.com/hedronvision/bazel-compile-commands-extractor)
  are candidate generation/IDE building blocks, not conforming integrations yet. Strict include
  resolution plus C++ module and PCH disposition qualified seed-only under #503
  (`cc/tests/fixtures/strict_generation/pins.bzl` via
  `bazel run //tools/ci:strict_generation_qualification`, loose generation rejected);
  C++ exact-target snapshot with managed clangd/toolchain projections and selected-target context
  qualified seed-only under #754 (`cc/tests/fixtures/cpp_snapshot/pins.bzl` via
  `bazel run //tools/ci:cpp_snapshot_qualification`, infer compile commands from `CcInfo` plus
  unrestricted query-driver plus workspace-writing refresh rejected).

The [approved Windows baseline](../decisions/0014-tested-platform-release-stack.md#decision)
requires MSVC compatibility and complete hermetic acquisition. The
[native qualification plan](../native-toolchains.md) selects toolchains_msvc with clang-cl and
Microsoft STL as the first Windows candidate; final adoption remains blocked. The following
comparison is not support evidence:

| Route | Compatibility and remaining evidence |
| --- | --- |
| Released MinGW-w64/UCRT with libc++ | Source-built dependency route, not sufficient for the approved MSVC-compatible baseline. Windows Bazel coverage remains unproven here. |
| clang-cl with MSVC ABI | Distinguish libc++ from Microsoft STL. The inspected [hermetic-llvm main revision](https://github.com/hermeticbuild/hermetic-llvm/tree/65a1e017a2d629f6e0e6d3d8d2189f3561ba684b) uses libc++ and rejects MSVC-target coverage; it is not released v0.8.19 support or proof of Microsoft-STL binary interoperability. |
| Actual MSVC compiler and Microsoft STL | [Bazel's documented route](https://bazel.build/configure/windows#build_cpp) uses installed tools and does not satisfy hermetic acquisition. A no-install upstream backend still needs exact identities, applicable licensing, Rust interoperability, and Bazel-owned coverage qualification. |

[Clang's MSVC compatibility](https://clang.llvm.org/docs/MSVCCompatibility.html) and
[Microsoft's binary compatibility rules](https://learn.microsoft.com/en-us/cpp/porting/binary-compat-2015-2017?view=msvc-170)
do not promise universal interchangeability. Representative
prebuilt MSVC C++ library interoperability fixtures with explicit STL, CRT, linker, and library
combinations, including mixed Rust/C/C++ dependencies, are qualified seed-only under #498
(`cc/tests/fixtures/prebuilt_interop/pins.bzl` via
`bazel run //tools/ci:prebuilt_interop_qualification`, host-to-target plus target execution
separately, compiler-target availability alone plus LLVM ancestry alone plus single-combo proof rejected).
Direct SDK downloads, permission to use, and permission to redistribute are separate checks;
the qualification plan records an upstream acceptance API, not a promised consumer setup command. The stack decision
does not admit an additional foundation or approve implementation.

The approved [Windows setup requirement](../decisions/0014-tested-platform-release-stack.md#decision)
allows explicit Microsoft EULA acknowledgement, as required by the inspected toolchains_msvc and
windows_support routes. This removes the prior setup-policy conflict, not the remaining transport,
corpus, or coverage qualification gaps. Automatic acceptance is not approved;
acquisition rights are qualified seed-only under #496
(`cc/tests/fixtures/acquisition_rights/pins.bzl` via
`bazel run //tools/ci:acquisition_rights_qualification`, usage vs redistribution reviewed separately,
official download not permission, assume rights rejected); prebuilt interop is qualified seed-only
under #498 (`cc/tests/fixtures/prebuilt_interop/pins.bzl` via
`bazel run //tools/ci:prebuilt_interop_qualification`, single-combo proof rejected).

### Native Toolchain Alternatives

The review prioritizes existing upstream capability and minimal project-owned integration over a
universal compiler distribution. Linux cross-builds are the first priority, not a mandate to build
every target from every host. The [approved stack](../decisions/0014-tested-platform-release-stack.md#decision)
requires glibc and static-musl Linux profiles; runtime floors are qualified seed-only under #500 (`cc/tests/fixtures/deployment_floors/pins.bzl` via `bazel run //tools/ci:deployment_floors_qualification`, unpinned floors rejected) and qualified routes are qualified seed-only under #504 (`cc/tests/fixtures/cross_routes/pins.bzl` via `bazel run //tools/ci:cross_routes_qualification`, not a mandate to build every target from every host; corpus qualified seed-only under #499) and bounded remediation is qualified seed-only under #505 (`cc/tests/fixtures/remediation_bounds/pins.bzl` via `bazel run //tools/ci:remediation_bounds_qualification`, Do not implement missing infra merely to fill cross-product with unbounded fork rejected) with backends provisional and no `Supported` claim.
Broader cross-builds are desirable when upstream configuration keeps maintenance bounded.
Windows hermeticity is not relaxed to reduce setup or integration effort.

The following are source/documentation observations; no builds were executed:

| Candidate | Upstream capability | Tradeoff for general developer workflows |
| --- | --- | --- |
| [hermetic-llvm v0.8.19](https://github.com/hermeticbuild/hermetic-llvm/blob/v0.8.19/README.md) | Integrated Linux glibc and static-only musl targets on x86_64/arm64, Apple SDK acquisition, LLVM runtime and tool targets, and Linux/macOS coverage fixtures. | Strongest integrated primary candidate with the preferred Rust rules. Dynamic musl is not supported. Default libc++ does not imply compatibility with prebuilt libstdc++ libraries; its Linux-glibc libstdc++ route is dynamic-only. Released Windows support does not satisfy the approved baseline. Floors qualified seed-only under #500 (`cc/tests/fixtures/deployment_floors/pins.bzl` via `bazel run //tools/ci:deployment_floors_qualification`, unpinned floors rejected); libstdc++ interop fixtures qualified seed-only under #498 (`cc/tests/fixtures/prebuilt_interop/pins.bzl` via `bazel run //tools/ci:prebuilt_interop_qualification`, single-combo proof rejected); the SQLite plus OpenSSL plus ring plus bindgen plus CXX corpus qualified seed-only under #499 (`cc/tests/fixtures/linux_corpus/pins.bzl` via `bazel run //tools/ci:linux_corpus_qualification`, single-crate proof rejected, native only); PIE plus ELF-dependency plus glibc-symbol bounded seed-only under #505 (`cc/tests/fixtures/remediation_bounds/pins.bzl` via `bazel run //tools/ci:remediation_bounds_qualification`, existing upstream constraints with no new infra), cross-build completeness qualified seed-only under #504 (`cc/tests/fixtures/cross_routes/pins.bzl` via `bazel run //tools/ci:cross_routes_qualification`). |
| [uber/hermetic_cc_toolchain v4.3.0](https://github.com/uber/hermetic_cc_toolchain/tree/v4.3.0) | Zig 0.15.2 supplies Linux glibc/musl cross-compilation and GNU/MinGW Windows targets. | Strong Linux alternative, but documented Apple SDK gaps, an external Zig runtime cache, and missing integrated Bazel coverage/Clang developer tools increase integration ownership. Shared-musl capability is not complete Rust/runtime-deployment evidence. Zig stays a comparison candidate only with the same evidence required, not a scope shortcut. |
| [toolchains_llvm v1.9.0](https://github.com/bazel-contrib/toolchains_llvm/tree/v1.9.0) | Configurable LLVM distributions, Linux/macOS C/C++ toolchains, sysroot APIs, and coverage/tool paths. | Useful when consumers already manage SDK/sysroot artifacts. It does not supply the complete target runtime closure, and the inspected Windows distributions do not establish a Windows C/C++ backend. |
| [toolchains_msvc prototype](https://github.com/Dragnalith/toolchains_msvc/tree/8e2aa4624bbb5a53a94f135e90995f307875d1ad) | No-install Microsoft compiler/SDK acquisition and real C/C++ toolchains, including clang-cl with Microsoft STL. | Candidate for Windows-hosted builds, not a qualified release: immutable lazy acquisition qualified seed-only under #495 (`cc/tests/fixtures/windows_acquisition/pins.bzl` via `bazel run //tools/ci:windows_acquisition_qualification`, mutable fetch rejected); acquisition rights qualified seed-only under #496 (`cc/tests/fixtures/acquisition_rights/pins.bzl` via `bazel run //tools/ci:acquisition_rights_qualification`, usage vs redistribution reviewed separately); transport plus ABI qualified seed-only under #497 (`cc/tests/fixtures/windows_transport/pins.bzl` via `bazel run //tools/ci:windows_transport_qualification`, compiler-target availability alone is not proof); prebuilt interop qualified seed-only under #498 (`cc/tests/fixtures/prebuilt_interop/pins.bzl` via `bazel run //tools/ci:prebuilt_interop_qualification`, single-combo proof rejected); linux corpus qualified seed-only under #499 (`cc/tests/fixtures/linux_corpus/pins.bzl` via `bazel run //tools/ci:linux_corpus_qualification`, single-crate proof rejected); floors qualified seed-only under #500 (`cc/tests/fixtures/deployment_floors/pins.bzl` via `bazel run //tools/ci:deployment_floors_qualification`, unpinned floors rejected); LCOV accounting qualified seed-only under #501 (`cc/tests/fixtures/lcov_accounting/pins.bzl` via `bazel run //tools/ci:lcov_accounting_qualification`, unaccounted lines plus ignored collection failures rejected). Its current repository operations use Windows commands; do not infer cross-host support. |

## Related issues

Tracking lives in the [roadmap](../roadmap.md). Platform hosts: #410, #411, #412, #413, #414. Release evidence: closed #803 plus #804 plus #805 plus #807 with best-effort exempt under closed #806 moot, process #808. Foundation mappings: #470-#489, #796-#800. See the roadmap for the full list.
