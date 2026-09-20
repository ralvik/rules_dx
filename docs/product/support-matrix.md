# Support Matrix

## Status Lifecycle

`Planned` → `Seed-host-delivered` → `Platform-qualified` → `Supported`.

`Planned` is accepted scope only and claims no delivery. `Seed-host-delivered` is implemented and
verified on the Linux x86_64 seed host only. No `Planned` cell is `Seed-host-delivered`:
as-built delivery is owned by the [verification matrix](../testing/verification-matrix.md),
not by this matrix. `Implementation status: implemented` / `Status: implemented` notes elsewhere
describe their owning contract's scope, not promotion under this lifecycle. `Platform-qualified` adds required-platform evidence per
[ADR 0014](../decisions/0014-tested-platform-release-stack.md). `Supported`
adds release evidence and is owned by this matrix; no cell is currently
`Supported`.

## Unqualified Platforms

Only the Linux x86_64 seed host plus Linux arm64 glibc native (issue
#410) plus the two Linux static-musl profiles (issue #411) plus macOS
arm64 native (issue #412) plus macOS x86_64 best-effort native (issue
#413) plus Windows x86_64 MSVC-compatible native (issue #414) are
delivered today. Every other host, including the remaining
[required platforms](../decisions/0014-tested-platform-release-stack.md#required-platforms),
is unqualified: `dx` refuses cleanly with `unsupported_platform`, naming the
host and pointing here and at ADR 0014, before any Bazel work starts. It
never presents partial execution on an unqualified host as success. The
refusal reads the same qualified-host list that platform evidence extends,
so a host flips on exactly when its evidence lands. This remains open work under issues #410-#414 and #415.
Platform qualification was tracked under issue #298 (closed): the Linux
x86_64 seed host plus Linux arm64 native plus the two static-musl profiles
plus macOS arm64 native plus macOS x86_64 best-effort native plus Windows
x86_64 MSVC-compatible native are delivered and tested; every other
required host needs pins, hosts, floors, JDK/SDK/CRT identities, qualified
routes, per-cell coverage, and consumer plus release evidence under its
per-host successor. The CI host matrix across these hosts is pinned by
`bazel run //tools/ci:ci_matrix_qualification` (issue #415).

Per-required-host qualification state (V1 status from ADR 0014; evidence
dimensions per issue #298; exact pins, hosts, floors, and SDK/CRT identities
remain owned by issues #410-#414 and are not pinned here):

| Host | V1 status | Qualification evidence | Current state |
| --- | --- | --- | --- |
| Linux x86_64 glibc | Required, primary bootstrap | Pins/hosts/floors, JDK/SDK/CRT identities, qualified native plus cross routes, per-cell coverage, consumer plus release evidence | Seed-host-delivered (CI: ubuntu-latest; artifacts: linux_x86_64; coverage: seed cell; consumer self-call: all-enabled on linux_x86_64 plus linux_arm64 plus macos_arm64 plus macos_x86_64, issue #408) |
| Linux arm64 glibc | Required | Same dimensions as above, native workflow (not cross-only) | Platform-qualified (issue #410; CI: ubuntu-24.04-arm; dx_tools linux_arm64 artifacts stay an owned follow-up gap with the recorded no-artifact diagnostic; coverage: arm64 cell; consumer self-call: all-enabled, issue #408; release evidence open) |
| Linux x86_64/arm64 static musl | Required profiles | Same dimensions; static native closure; dynamic musl explicitly out of scope | Platform-qualified (issue #411; CI: ubuntu-latest plus ubuntu-24.04-arm cross-build with per-profile bazel-musl-* cache scopes; Rust musl std via extra_target_triples with exec-platform tools for build scripts/proc macros and target musl libs for apps, prebuilt glibc libs never musl-compatible by linker change alone; hermetic-llvm static-only musl targets stay provisional; coverage: musl x86_64 plus musl arm64 cells with no union; consumer self-call plus musl jobs; release evidence open) |
| macOS arm64 | Required | Same dimensions; pinned acquired SDK (SDK version is not the deployment floor) | Platform-qualified (issue #412; CI: macos-14 with bazel-macos-arm64- cache scope; pinned upstream toolchains with the hermetic-llvm Apple-SDK backend provisional plus immutable lazy fetch; host-installed SDK fallback never approved; Apple-SDK handling leaks no secrets and needs no interactive acceptance; dx_tools macos_arm64 artifacts stay an owned follow-up gap with the recorded no-artifact diagnostic; coverage: macos arm64 cell with no union; consumer self-call: all-enabled, issue #408; release evidence open) |
| macOS x86_64 | Best-effort | Same dimensions; pinned acquired SDK (SDK version is not the deployment floor); best-effort non-blocking | Platform-qualified (issue #413; CI: macos-15-intel with bazel-macos-x86_64- cache scope; `macos-13` retired December 2025, `macos-15-intel` until August 2027; pinned upstream toolchains with the hermetic-llvm Apple-SDK backend provisional plus immutable lazy fetch; host-installed SDK fallback never approved; Apple-SDK handling leaks no secrets and needs no interactive acceptance; dx_tools macos_x86_64 artifacts stay an owned follow-up gap with the recorded no-artifact diagnostic; coverage: macos x86_64 cell with no union; consumer self-call: all-enabled, issue #408; release evidence open; gaps never block required-host release) |
| Windows x86_64 MSVC-compatible | Required | Same dimensions; hermetic acquisition plus MSVC compatibility gates unchanged; explicit EULA acceptance required, never automatic | Platform-qualified (issue #414; CI: windows-latest with bazel-windows-x86_64- cache scope plus shell bash; pinned upstream toolchains with the toolchains_msvc clang-cl/Microsoft-STL backend provisional plus immutable lazy fetch; explicit EULA acceptance required never automatic, usage vs redistribution reviewed separately, merely adding the module requires no acceptance and fetches no restricted payloads; installed Build Tools fallback never approved; prebuilt-MSVC interop fixtures with explicit STL/CRT/linker/library combos incl mixed Rust/C/C++ qualify host-to-target plus target execution separately, compiler-target availability alone is not proof; manifest/path/ABI gaps closed with fixtures, remaining corpus gaps owned under issues #473, #474, #499; dx_tools windows_x86_64 artifacts stay an owned follow-up gap with the recorded no-artifact diagnostic; coverage: windows x86_64 cell with no union; consumer self-call: all-enabled, issue #408; release evidence open) |
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
(`Open (adapter-less)` for Go, Java, Kotlin, Scala, C#, F#, C++ and `Open (regions)` for
Vue, Svelte, Astro, MDX); `Dependencies` and framework `Build`/`Test` composition for Vue,
Svelte, Astro, MDX map to `Depcheck` (`Open`) and `Examples` (`Open (composition)`). Rust,
Python, JavaScript, and TypeScript `Build`/`Test`/`Dependencies`/`Generate`/`Coverage`/
`Format`/`Lint`/`Typecheck` delivery follows the `Delivered` verification cells for corpus
dogfood, Layer-2, generation, examples, E2E contract, and depcheck; their `Environment`/`IDE`
cells stay scope-only while `Env/codegen` stays `Open`. No `Planned` cell implies
remaining-required-platform,
external-consumer, or trusted-builder release evidence; see
`../../CHANGELOG.md`. `Planned: feasibility` means evaluation
under [first-release admission](scope.md#first-release-admission):
admit qualifying capabilities, or record an approved disposition. Additional foundations may be
deferred when completion needs substantial infrastructure; required quality tools are independent.
Remaining upstream mappings are tracked in open work under issues #416-#420 and #476-#489.
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
source-audit tooling is open work while ecosystem audit/update wiring is delivered.
Ecosystem dependency-vulnerability audit
coverage is separate; see the [audit contract](../cli/commands/audit-update-bazel.md). This matrix makes no
ecosystem audit-tool or dependency-source claim beyond that contract.

The following core and named framework foundations are required for v1 on every
[required platform](../decisions/0014-tested-platform-release-stack.md#required-platforms).
They are not eligible for the additional-foundation deferral policy.

| Language | Build | Test | Dependencies | Generate | Environment | IDE | Coverage | Format | Lint | Typecheck | Audit |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Rust | Planned | Planned | Planned: Cargo lock | Planned | Planned: native tools | Planned: rust-analyzer/flycheck | Planned | Planned: rustfmt | Planned: Clippy | Planned: compiler diagnostics | Not planned |
| Python | Planned | Planned: pytest | Planned: uv lock | Planned | Planned: `.venv` | Planned: interpreter/imports | Planned | Planned: Ruff | Planned: Ruff, pydoclint; flake8/pylint opt-in | Planned: Ty | Planned: audit tools (open work under issue #512) |
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
open work under issue #510 and issues #416-#420.

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
| Rust application | `rules_rs` + pinned patched `rules_rust` ([ADR 0013](../decisions/0013-rust-javascript-typescript-foundations.md)) | Providers/Gazelle/integration pinned; native gaps: kept CC opt-out linker, shell-env default, bindgen LLVM-22-vs-23, CXX graph identity, exact-target discovery | Pinned (`bazel run //tools/ci:foundation_maps`, issue #470); native gaps under issues #471, #472, #473, #474, #475 |
| Python application | `aspect_rules_py` 2.x ([ADR 0010](../decisions/0010-python-foundation.md)) | Mappings | Pinned (`bazel run //tools/ci:foundation_maps`) |
| JavaScript/TypeScript application | `aspect_rules_js`/`aspect_rules_ts`, `aspect_rules_jest` ([ADR 0013](../decisions/0013-rust-javascript-typescript-foundations.md)) | Wrappers/Gazelle | Pinned (`bazel run //tools/ci:foundation_maps`) |
| JS/TS quality | Biome default; ESLint, Prettier available; `tsc` diagnostic-only | Mappings | Pinned (`bazel run //tools/ci:foundation_maps`) |
| Python quality | Ruff, Ty, pydoclint (Bandit excluded from v1 by [ADR 0019](../decisions/0019-first-release-additional-foundations.md)) | Mappings, Ty | Pinned (`bazel run //tools/ci:foundation_maps`) |
| Vue / Svelte / Astro / MDX | Named upstream adapters per framework | Adapter mappings plus composition evidence | issue #510 |
| Generation transport | Declared versioned manifest artifact (`GenerationManifest` in `generation/result.proto`, projected by `ProjectedManifest` in `cli/cli/src/generate.rs`) with explicit path/label/pattern scope | Implemented | Shipped |
| Target resolution | All-direct-owners query strategy ([Target Resolution](../cli/target-resolution.md), pinned by resolver fixtures) | Implemented | Shipped |
| Quality core/result contract | Internal Protobuf + NDJSON output | Core mappings | open work under issue #511 |
| Required platforms | [Required-platform table](../decisions/0014-tested-platform-release-stack.md#required-platforms) | Pins, hosts, floors | open work under issues #410-#414 and #500 |
| Coverage gate | Instrumentation-first; behavioral fallback only on proof | Resolved in [coverage](../testing/README.md#coverage) | Enforced by CI |
| Consumer CI | Reusable workflow + caller template | Delivered; verification open | Shipped |
| Repository workflows | Codegen/env/setup implemented; `dx update` plus `dx audit` live execution delivered | Codegen pairs; audit/update | open work under issue #506 and issue #512 |

Required-core Rust providers/Gazelle/integration are pinned (issue #470); the five
native gaps below stay owned under issues #471, #472, #473, #474, #475; Vue/Svelte/Astro/MDX
adapter mappings plus composition evidence stay open under issue #510. Python mappings
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
authority, issue #470),
build-script hermetic defaults (`use_cc_toolchain = True`, `use_default_shell_env = False`,
`emit_warnings = True` in `gazelle/rust/lang.go`, proven by `gazelle/rust/lang_test.go`),
Ty provenance plus `ty` typecheck adapter mapping (`quality/artifacts/ty.linux_x86_64.bzl`,
`quality/adapters.bzl`, `quality/adapter/src/parsers/ty.rs`), JS/TS quality adapter mappings
(`biome`/`eslint`/`prettier`/`tsc` in `quality/adapters.bzl` plus their parsers), and
framework composition (`examples/mixed/hello/` plus `gazelle/mixed/`). Remaining native gaps
(kept CC opt-out linker failure path under issue #471, global shell-env False versus
annotation extension under issue #472, bindgen LLVM-22-vs-23 compatibility under issue
#473, CXX graph identity under issue #474, exact-target discovery under issue #475) stay
owned under issues #471, #472, #473, #474, #475 per the [native plan](../native-toolchains.md#qualification-questions-and-delivery).
No `Supported` claim until platform plus consumer plus release evidence passes.

Required platforms and the quality-tool baseline (minus excluded Swift) frame
delivery of this inventory. Admitted additional foundations are tracked in
open work under issues #416-#420 and #476-#489;
remaining feasibility detail stays in the
[feasibility review](#additional-v1-foundations) for qualification.
Swift is an evidence-backed v1 exclusion, not a pending assessment. Effort
is estimated from qualification evidence as
each tracked item lands; no person-hour figures are frozen here.

## Additional V1 Foundations

Admit/defer/exclude outcomes are decided by
[ADR 0019](../decisions/0019-first-release-additional-foundations.md) and
summarized in [Candidate Disposition Status](#candidate-disposition-status).
The review below records upstream evidence only. Include complete low-cost
upstream-backed foundations; substantial missing integration may justify an explicit approved
deferral under the admission policy. Each foundation is tracked with applicable build, test, dependency, generation,
environment, IDE, coverage, and quality cells with named upstreams and evidence. No row implies
that a ruleset has already been selected or that every capability/platform is feasible.
This is a minimum inventory, not an exhaustive list of eligible languages.

### Admitted To V1

Java, Kotlin, C#, F#, Go, C/C++, and Scala application foundations are admitted
to v1 scope by [ADR 0019](../decisions/0019-first-release-additional-foundations.md), on the evidence in the
[candidate review](#additional-language-foundation-candidate-review): each has
an active Bzlmod-published upstream ruleset with a concrete dependency-lock
and toolchain story. Their quality integration is v1 scope with the defaults in
[Provisional Default Quality Tools](#provisional-default-quality-tools);
exact versions, rule sets, and adapter mappings are tracked in
open work under issues #485-#489. The managed Scala
route decision was recorded 2026-09-13.
Unresolved cells block qualification; moving an admitted foundation out later
requires a new evidence-backed decision.

Admitted additional foundations (Go, C/C++, Java, Kotlin, Scala, C#, F#) stay open
under issues #476-#484 plus #485-#488: per-foundation exact upstream versions, rulesets,
adapter mappings, lock wiring, test runners, and quality tools remain qualification work. Qualified mappings
are pinned by `bazel run //tools/ci:foundation_maps` with owning qualification in
[Generation](../generation/README.md#language-mapping-qualification),
[Environments](../environments/README.md#language-mapping-qualification),
[Tools](../tools/README.md#language-mapping-qualification), and the
[native plan](../native-toolchains.md#qualification-questions-and-delivery):
wrappers preserving upstream providers plus `QualitySourcesInfo`, Gazelle extensions,
env plans, hello builds, and lock authority (`maven_install.json` plus fail-closed,
Paket plus `paket.main`, Go stdlib-only, C/C++ none), with test runners (`go test`,
ScalaTest, JUnit 4 seed, plain `cc`/`csharp`/`fsharp` executables) and
classification-only quality families. Dependency hygiene (lockfile-consistency plus
declared-dependency usage with category, exception, and obsolete checks) is qualified for
all admitted languages in `tools/depcheck/` (issue #22; remaining opens under issue #510)
with native authorities (go.sum,
`maven_install.json`, `paket.lock`, per-archive sha256) and focused fixtures. Remaining gaps
(GoogleTest v1.18.0 under issue #479, JUnit 6.1.3 plus 5.14.x fallback under issue #476,
xUnit v3 4.0.0 under issue #477, Go `from_file` when
non-stdlib deps land under issue #483, quality adapters qualified under issue #307 with
deferred implementation
owned by ADR 0019, C/C++ MSVC interop plus SDK licensing)
stay owned under issues #476-#484 plus #485-#488. No `Supported` claim until platform plus consumer plus release
evidence passes.

### Deferred Beyond V1

Ruby and PowerShell application foundations are deferred beyond v1 by
[ADR 0019](../decisions/0019-first-release-additional-foundations.md), with
evidence in the [candidate review](#additional-language-foundation-candidate-review):
Ruby's fragmented ruleset maintenance ownership and gem/bundler packaging
effort, and PowerShell's single young execution-only upstream with unproven
generation, dependency, environment, and IDE stories, exceed the low-cost
hermetic integration bar. Deferred foundations' quality-tool cells (RuboCop,
StandardRB, PSScriptAnalyzer) stay in force; a foundation
deferral removes no baseline tool. Reconsideration after v1 requires a new
scope decision. The deferred/excluded record is decided by
[ADR 0019](../decisions/0019-first-release-additional-foundations.md) (Ruby plus
PowerShell deferred, Swift plus Bandit excluded, host-toolchain fallback never approved;
no open tracker; reconsideration requires a new scope decision).
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
library-API binding, and per-tool adapter mappings qualified under issue #420 with deferred
implementation owned by ADR 0019; reconsideration requires
a new scope decision) stay decided by ADR 0019 with no open tracker. No `Supported` claim until platform plus
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
| Ruby | Deferred beyond v1 | Planned: feasibility | Planned: RuboCop, StandardRB |
| PowerShell | Deferred beyond v1 | Planned: feasibility | Planned: PSScriptAnalyzer |
| Swift | Not planned | Not planned | Not planned |

`Planned` in the table above means scope admitted to v1 by ADR 0019 with no delivery
claimed; delivery follows the [verification matrix](../testing/verification-matrix.md).
Admitted `Format`/`Lint` cells map to `Layer-2 matrix Open (adapter-less)` (no adapter
claims them yet; open under issue #418 for the Native cohort and issues #416/#417 for the
JVM and Scala + .NET cohorts), `Environment`/`IDE` dimensions map to `Env/codegen Open`,
and per-language source audit lumped in the third column is distinct from ecosystem
`Audit/update Delivered`.

### Initial Feasibility Review

The approved review order starts with Go and C/C++. C/C++ initially assesses Bazel-native
projects; CMake/Meson integration is a separate review, not an automatic-migration promise.
This prioritization admits or defers no foundation and changes no tracking dependencies.

The following are upstream documentation/source observations, not executed qualification evidence.
Status cells are owned by this matrix; provisional toolchain and backend
choices are owned by the [native qualification plan](../native-toolchains.md). Identified contract conflicts must be resolved (tracked in
open work under issue #505) before affected implementation:

- [rules_go](https://github.com/bazel-contrib/rules_go) and
  [Gazelle](https://github.com/bazel-contrib/bazel-gazelle) offer SDK acquisition, package build/test,
  module dependencies, and generation. The approved narrow Go exception preserves package-level
  tests and declared platform/build constraints under the
  [generation contract](../generation/common.md#ownership-and-naming); exact mappings remain open
  (open work under issue #510).
  Source-only module identity, strict dependency resolution, and cgo/race scope remain unresolved;
  remains open under issue #510.
- The documented [Go editor driver](https://github.com/bazel-contrib/rules_go/blob/v0.63.0/docs/editors.md)
  invokes Bazel. That automatic integration is approved under
  [environment refresh](../environments/environment.md#ownership-and-refresh), following the
  cross-language automatic-first policy rather than building a static snapshot alternative.
  Exact-target isolation, failure propagation, cgo IDE behavior, and every host workflow still need
  qualification; upstream explicitly does not guarantee cgo completion.
- [rules_cc](https://github.com/bazelbuild/rules_cc) supplies build rules, not a hermetic compiler
  distribution. [hermetic-llvm v0.8.19](https://github.com/hermeticbuild/hermetic-llvm/tree/v0.8.19)
  is the inspected release of the preferred Linux/macOS backend. Its released Windows route uses
  MinGW-w64/UCRT, which does not satisfy the approved Windows baseline by itself. MSVC-compatible
  interoperability and Windows coverage need qualification; Apple/Microsoft SDK acquisition and
  license terms remain unresolved. No host-installed SDK fallback is approved.
- [gazelle_cc v0.6.0](https://github.com/EngFlow/gazelle_cc/tree/v0.6.0) and
  [Hedron's compilation-command extractor](https://github.com/hedronvision/bazel-compile-commands-extractor)
  are candidate generation/IDE building blocks, not conforming integrations yet. Strict include
  resolution, C++ module scope, managed clangd/toolchain projections, and selected-target context
  remain work to assess under the existing contracts.

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
do not promise universal interchangeability. Before qualification, define and test representative
prebuilt MSVC C++ library interoperability fixtures with explicit STL, CRT, linker, and library
combinations, including mixed Rust/C/C++ dependencies. Qualify specific host-to-target build routes
and target execution separately; compiler target availability alone does not prove either.
Direct SDK downloads, permission to use, and permission to redistribute are separate checks;
the qualification plan records an upstream acceptance API, not a promised consumer setup command. The stack decision
does not admit an additional foundation or approve implementation.

The approved [Windows setup requirement](../decisions/0014-tested-platform-release-stack.md#decision)
allows explicit Microsoft EULA acknowledgement, as required by the inspected toolchains_msvc and
windows_support routes. This removes the prior setup-policy conflict, not the remaining acquisition,
licensing, interoperability, or coverage qualification gaps. Automatic acceptance is not approved;
the exact upstream mechanism and applicable usage/distribution rights are tracked in
open work under issue #496.

### Native Toolchain Alternatives

The review prioritizes existing upstream capability and minimal project-owned integration over a
universal compiler distribution. Linux cross-builds are the first priority, not a mandate to build
every target from every host. The [approved stack](../decisions/0014-tested-platform-release-stack.md#decision)
requires glibc and static-musl Linux profiles; runtime floors (open work under issue #500) and qualified routes (open work under issues #499 and #504) remain under review.
Broader cross-builds are desirable when upstream configuration keeps maintenance bounded.
Windows hermeticity is not relaxed to reduce setup or integration effort.

The following are source/documentation observations; no builds were executed:

| Candidate | Upstream capability | Tradeoff for general developer workflows |
| --- | --- | --- |
| [hermetic-llvm v0.8.19](https://github.com/hermeticbuild/hermetic-llvm/blob/v0.8.19/README.md) | Integrated Linux glibc and static-only musl targets on x86_64/arm64, Apple SDK acquisition, LLVM runtime and tool targets, and Linux/macOS coverage fixtures. | Strongest integrated primary candidate with the preferred Rust rules. Dynamic musl is not supported. Default libc++ does not imply compatibility with prebuilt libstdc++ libraries; its Linux-glibc libstdc++ route is dynamic-only. Released Windows support does not satisfy the approved baseline. Floor pinning stays owned under open work under issue #500; libstdc++ interop fixtures stay owned under open work under issue #498; profile completeness (PIE, ELF dependencies, glibc symbols, corpus) stays owned under open work under issue #499. |
| [uber/hermetic_cc_toolchain v4.3.0](https://github.com/uber/hermetic_cc_toolchain/tree/v4.3.0) | Zig 0.15.2 supplies Linux glibc/musl cross-compilation and GNU/MinGW Windows targets. | Strong Linux alternative, but documented Apple SDK gaps, an external Zig runtime cache, and missing integrated Bazel coverage/Clang developer tools increase integration ownership. Shared-musl capability is not complete Rust/runtime-deployment evidence. Zig stays a comparison candidate only with the same evidence required, not a scope shortcut. |
| [toolchains_llvm v1.9.0](https://github.com/bazel-contrib/toolchains_llvm/tree/v1.9.0) | Configurable LLVM distributions, Linux/macOS C/C++ toolchains, sysroot APIs, and coverage/tool paths. | Useful when consumers already manage SDK/sysroot artifacts. It does not supply the complete target runtime closure, and the inspected Windows distributions do not establish a Windows C/C++ backend. |
| [toolchains_msvc prototype](https://github.com/Dragnalith/toolchains_msvc/tree/8e2aa4624bbb5a53a94f135e90995f307875d1ad) | No-install Microsoft compiler/SDK acquisition and real C/C++ toolchains, including clang-cl with Microsoft STL. | Candidate for Windows-hosted builds, not a qualified release: immutable acquisition, Rust build-script paths, coverage, and licensing need review. Its current repository operations use Windows commands; do not infer cross-host support. |
| [windows_support v0.4.1](https://github.com/hermeticbuild/windows_support/tree/v0.4.1) | Pinned acquisition APIs for Microsoft SDK, STL/runtime headers, libraries, and redistributables. | Reusable acquisition building block, not a compiler or C/C++ toolchain. It does not by itself change hermetic-llvm's libc++ selection or supply coverage. |
| [Installed Microsoft Build Tools](https://bazel.build/configure/windows#build_cpp) | Established Bazel MSVC or clang-cl route with Microsoft SDK/STL. | Rejected for the current contract: external provisioning and host-state dependence do not meet the required hermetic acquisition. Coverage also remains unqualified. |

[rules_rs v0.0.109](https://github.com/hermeticbuild/rules_rs/tree/v0.0.109) documents the LLVM
integration and pins a patched rules_rust; it declares LLVM 0.8.18, so even upgrading to 0.8.19
requires stack qualification. Zig substitution needs matching libc constraints and execution/target
selection, not just dependency replacement. Direct rules_rust has upstream cross-compilation and
bindgen examples, but changing the preferred Rust foundation still requires approval under
[ADR 0013](../decisions/0013-rust-javascript-typescript-foundations.md).

The proposed qualification corpus should exercise source-built SQLite, OpenSSL with explicitly
declared build tools or Bazel library inputs, C/assembly crates such as ring, bindgen inside build
scripts versus explicit Bazel binding generation, CXX, and native dependencies of proc macros.
Features and runtime closures matter more than crate names. Prebuilt glibc libraries do not become
musl-compatible by changing the linker; build scripts and proc macros need execution-platform tools
while applications need target-platform libraries. The current
[build-script contract](../generation/rust.md#build-scripts) enables the selected hermetic C/C++
toolchain by default with an explicit kept opt-out. Additional native inputs remain declared;
fixture-proven upstream mappings are still required (tracked in
open work under issue #499; PIE plus ELF-dependency plus glibc-symbol evidence stays under issue #499, floor pinning stays under issue #500, bindgen compatibility stays under issue #473, CXX identity stays under issue #474; mixed Rust/C/C++ prebuilt-MSVC combos stay owned with issue #414, not double-claimed here), not automatic compatibility with every crate.

The approved direction is to qualify rules_cc and rules_rs with hermetic-llvm for Linux glibc,
static-musl, and macOS. Windows first qualifies the toolchains_msvc clang-cl/Microsoft-STL route;
adoption remains blocked on upstream acquisition, interoperability, and coverage evidence.
Zig remains a comparison candidate where it supplies a concrete benefit. Do not implement missing
dynamic-musl, coverage, SDK, or compiler-stack infrastructure merely to fill the cross-product.
Apple/Microsoft acquisition rights and the existing managed-tool packaging/laziness contracts remain
gates even when upstream technically supports a route. The evidence and any
required contract changes must be resolved (tracked in
open work under issue #505) before implementation; no exact pins or support claims are established.

The [native qualification plan](../native-toolchains.md) owns the detailed provisional choices,
execution-to-target cohort, source-derived defects, alternatives, and remaining questions. Findings
include a Rust build-script CC opt-out linker failure path, third-party shell-environment defaults,
Windows manifest/path/ABI gaps, and incomplete coverage/generation/IDE integration. These invalidate
any assumption that the complete native foundation is configuration-only; they do not reverse the
approved build-script default or constitute an additional-foundation deferral.

### Additional Language-Foundation Candidate Review

The following extends the documentation/source review pattern to the remaining
additional-foundation language candidates. These are upstream
documentation/source observations, not executed qualification evidence.
Status cells are owned by the matrices above;
provisional ruleset choices below are candidates for qualification tracked in
open work under issues #416-#420,
not selections. This review itself admits, defers, or excludes nothing;
outcomes are decided by
[ADR 0019](../decisions/0019-first-release-additional-foundations.md) and
summarized in [Candidate Disposition Status](#candidate-disposition-status).

- Java: [`rules_jvm_external` 7.x](https://github.com/bazel-contrib/rules_jvm_external)
  (Bzlmod, Coursier/Maven/Gradle resolvers, checked-in `maven_install.json`
  checksum lockfile) is the provisional Maven-dependency route over the bundled
   `rules_java` toolchain. Bazel documents a hermetic compile route via
   `remote_java_repository` and `--java_runtime_version` flags; per-platform JDK
   acquisition and coverage integration remain open (open work under issues #500 and #507); lock
   authority is `maven_install.json`
   ([Provisional Default Dependency Locks](#provisional-default-dependency-locks)).
   Admitted to v1 by [ADR 0019](../decisions/0019-first-release-additional-foundations.md).
- Kotlin: [`rules_kotlin` 2.4.10](https://registry.bazel.build/modules/rules_kotlin)
  (Bzlmod) is the provisional build/test route on the same
  JVM toolchain and Maven-lock story as Java. Kotlin compiler acquisition,
  worker behavior, and IDE projection remain open (open work under issues #500 and #506). Admitted to
  v1 by [ADR 0019](../decisions/0019-first-release-additional-foundations.md).
- Scala: [`rules_scala` 7.x](https://github.com/bazel-contrib/rules_scala)
  (bazel-contrib, Bazel 7/8 plus Bzlmod, Coursier-backed Scala
  toolchains) is the provisional route. Route decision (recorded
  2026-09-13): managed route tracked under
  issue #417. Coursier-fetched
  toolchains share Java's Maven-lock story (`maven_install.json` plus
  `fail_if_repin_required`), Scalafix semantic rules need semanticdb plus
  classpath wiring per the adapter-input notes, and no source-built or
  toolchain-coupled native advantage was evidenced. Mappings are tracked under
  issue #417. Admitted to v1
  by [ADR 0019](../decisions/0019-first-release-additional-foundations.md).
- C# and F#: [`rules_dotnet` 0.22.1](https://github.com/bazel-contrib/rules_dotnet)
  (Bzlmod, Bazel >= 8) covers both languages with `csharp_*`
   and `fsharp_*` targets plus SDK acquisition through its `dotnet` toolchain
   extension, so one upstream review covers both rows. Per-platform SDK
   acquisition and test-runner versions remain open under issue #417; lock
   authority is the Paket lock
   ([Provisional Default Dependency Locks](#provisional-default-dependency-locks)).
   C# and F# admitted to v1 by [ADR 0019](../decisions/0019-first-release-additional-foundations.md).
- Ruby: [`rules_ruby`](https://github.com/bazel-contrib/rules_ruby) (Bazel 8,
  Bzlmod, published to the Bazel Central Registry) is the provisional route,
  but the ruleset history is fragmented (retired `bazelruby/rules_ruby`,
  several forks) and canonical maintenance ownership is the thinnest of this
cohort. Gem/bundler integration and the packaging-effort boundary stay the
cost drivers, open. Foundation deferred beyond v1 by [ADR 0019](../decisions/0019-first-release-additional-foundations.md);
   the RuboCop/StandardRB tool cohort is open under issue #420.
- PowerShell: [`rules_powershell` 0.2.0](https://github.com/periareon/rules_powershell)
  (in the Bazel Central Registry, built over `rules_shell`) is the only known
  upstream execution route and the youngest of this cohort. Generation,
  dependency, environment, and IDE stories for `.ps1` application foundations
  are unproven, and PSScriptAnalyzer integration is unassessed; this row
  carries the highest deferral risk if qualification shows substantial missing
  infrastructure. All mappings are open. Foundation deferred
   beyond v1 by [ADR 0019](../decisions/0019-first-release-additional-foundations.md); the PSScriptAnalyzer tool cohort
   is open under issue #420.

### Candidate Disposition Status

| Candidate | Provisional basis | Tracking | Disposition |
| --- | --- | --- | --- |
| Java | `rules_jvm_external` + `rules_java` | open work under issue #416 | Admitted to v1 ([ADR 0019](../decisions/0019-first-release-additional-foundations.md)) |
| Kotlin | `rules_kotlin` | open work under issue #416 | Admitted to v1 ([ADR 0019](../decisions/0019-first-release-additional-foundations.md)) |
| Scala | `rules_scala` | issue #417 (managed route decided 2026-09-13) | Admitted to v1 ([ADR 0019](../decisions/0019-first-release-additional-foundations.md)) |
| C# | `rules_dotnet` | issue #417 | Admitted to v1 ([ADR 0019](../decisions/0019-first-release-additional-foundations.md)) |
| F# | `rules_dotnet` | issue #417 | Admitted to v1 ([ADR 0019](../decisions/0019-first-release-additional-foundations.md)) |
| Ruby | `rules_ruby` (bazel-contrib) | issue #420 tool cohort; foundation post-v1 | Deferred beyond v1 ([ADR 0019](../decisions/0019-first-release-additional-foundations.md)) |
| PowerShell | `rules_powershell` | issue #420 tool cohort; foundation post-v1 | Deferred beyond v1 ([ADR 0019](../decisions/0019-first-release-additional-foundations.md)) |
| Go, C/C++ | Source/documentation review | issue #418 (native cohort, routes decided 2026-09-13) | Admitted to v1 ([ADR 0019](../decisions/0019-first-release-additional-foundations.md)) |
| Swift, Bandit | Excluded from v1 ([ADR 0019](../decisions/0019-first-release-additional-foundations.md)) | N/A | Evidence-backed v1 exclusion |

No person-hour figures are frozen here; effort is estimated from qualification
evidence as each tracked item lands, per the core section above. A deferral or
exclusion recorded during qualification must be evidence-backed, not
silent.

### Provisional Default Test Runners

Upstream documentation observations; provisional defaults, not
selections. Exact versions, runner mappings, and
fixture qualification are open.

- Java and Kotlin: JUnit (≈62–79% JVM adoption, Spring Boot default; JUnit 6
  adds native Kotlin `suspend` support). JUnit 5 versus 6 follows the qualified JDK
  baseline (open); the provisional pin on the 6.x line is 6.1.3,
  with 5.14.x maintained as the fallback line.
- C# and F#: xUnit v3 4.0.0 (greenfield default: isolation
  and parallelism by default; used by the ASP.NET Core team; Microsoft
  documents xUnit for F#). The `xunit.v3` package pulls `xunit.analyzers`
  (2.0.0) as a dependency.
- Go: `go test` through `rules_go` (`go_test`); no alternative exists. The
  runner follows the Go toolchain pin, so no separate version is named here.
- C/C++: GoogleTest v1.18.0 (industry default with native mocking
  and death tests; first-class Bazel support via the Central Registry module
  and `cc_test` integration). The 1.18.x branch requires C++17 or newer,
  which the qualified toolchain floor must cover (tracked in
  open work under issues #479 and #500). Upstream recommends living at
  head; dx pins the release.
- Scala: ScalaTest 3.2.20 (covers Scala 2.10–2.13 and 3.x),
  per the rules' own preference (`scala_test` runs suites written using the
  `scalatest` library; the rule implementation is ScalaTest-wired).
  `scala_junit_test` and `scala_specs2_junit_test` are rules-supported
  choices, not defaults. Scalactic is the recommended companion, not a
  requirement.

Where the upstream rules support multiple runners, the runner is a
configuration choice over the qualified set, not a hard-coded single
framework; the defaults above apply when unconfigured. Secondary runners
(TestNG, NUnit/MSTest, Catch2, specs2/JUnit variants) are therefore
choice candidates pending qualification evidence, not exclusions and not
v1 commitments.

### Provisional Default Quality Tools

Upstream documentation observations; provisional defaults, not
selections. Exact versions, rule sets, and adapter mappings are tracked in
open work under issues #485-#489.

- Go: gofumpt (strict superset of gofmt) for format; staticcheck plus `govet`
  for lint, since neither subsumes the other. The existing `SA`-only suggestion
  conflicts with the default-check suggestion below; both remain provisional,
  not an approved native rule selection. Route decision (recorded 2026-09-13):
  gofumpt, staticcheck, and errcheck stay standalone checksummed-artifact
  candidates while `govet` ships with the authoritative Go toolchain (no
  separate acquisition); the `SA`-only versus default-checks conflict stays
  unresolved under issue #418 — neither is selected here. No adapter claims
  `go` yet (open under issue #418).
- C/C++: clang-format, clang-tidy, and cppcheck as already named;
  compiler-integrated and standalone analysis are complementary. Route
  decision (recorded 2026-09-13): clang-format and clang-tidy resolve from
  the qualified hermetic-llvm LLVM distribution's tool targets
  (authoritative-toolchain class, no separate acquisition); cppcheck stays a
  standalone checksummed-artifact candidate pending adapter qualification.
  No adapter claims c/cpp yet (open under issue #418).
- Java: google-java-format; PMD, Checkstyle, SpotBugs, plus Error Prone, which
  works out of the box with Bazel. The Error Prone version follows the
  qualified JDK baseline (open).
- Kotlin: ktfmt for format with ktlint for style (fast, no type resolution);
  detekt for smells, complexity, and potential bugs (200+ rules, type
  resolution optional). Format and analysis stay divided as upstream documents
  them.
- Scala: scalafmt and Scalafix as already named.
- C#: CSharpier (faster and more opinionated than `dotnet format`; no MSBuild
  dependency); SDK-built-in Roslyn CA analyzers, on by default for .NET 5+,
  for lint. StyleCop stays a style choice candidate, not the default.
- F#: Fantomas for format; FSharpLint for lint (its own formatting rules are
  deprecated in favor of Fantomas, so the split is clean).

Typechecking for every admitted language is compiler-owned (javac, kotlinc,
Roslyn, `go`, clang/gcc, scalac); no separate typechecker is selected, unlike
the standalone Ty and `tsc` tools in the required core.

Focused proofs (recorded 2026-09-13) select the delivery routes in
[first-release tool routing](../tools/tool-acquisition.md#first-release-tool-routing):
JVM tools (google-java-format, Checkstyle, PMD, SpotBugs, ktfmt, ktlint)
take the complete-upstream-artifact plus shared-JDK route (issue #416, live successor
to closed #307 for the `java`/`kotlin` cohort); Scalafmt/Scalafix
take the managed JVM route (issue #417, live successor to closed #307 for the
`scala` cohort); CSharpier/Fantomas take the exact-package plus
shared-.NET-runtime route (issue #417, live successor to closed #307 for the
`csharp`/`fsharp` cohort); clang-format, clang-tidy, and cppcheck take the
split native route (issue #418, live successor to closed #307 for the
`c`/`cpp` classes); gofumpt, staticcheck, `govet`, and errcheck take the split
Go route (issue #418, live successor to closed #307 for the `go` class);
`buf` takes the checksummed native/self-contained artifact route (issue #419,
live successor to closed #307 for the `protobuf` class; needs no target
compiler context, execution-platform lazy); qmlformat and qmllint take the
authoritative-toolchain route from the Qt distribution (issue #419, live
successor to closed #307 for the `qml` class; Qt last);
PSScriptAnalyzer takes the exact-module plus
portable-PowerShell-runtime route (issue #420, live successor to closed #307 for the
`powershell` class); RuboCop/StandardRB take the
release-assembled Ruby closure route (issue #420, live successor to closed #307 for the
`ruby` class; the exceptional bundle within the approved packaging-effort boundary).
Exact JVM artifacts, versions, rule sets,
and adapter mappings are tracked under issue #416; exact Scala/.NET artifacts,
versions, rule sets, and adapter mappings are tracked under issue #417; exact
native artifacts, versions, rule sets, and adapter mappings are tracked under
issue #418; exact structured (`buf`/Qt) artifacts, versions, rule sets, and
adapter mappings are tracked under issue #419; exact interpreted/file-family
(`ruby`/`powershell` plus cue/jsonnet/pkl/css/html_template/gherkin/sql/xml/go_module/
terraform/yaml/text) artifacts, versions, rule sets, and adapter mappings are tracked
under issue #420; remaining cohorts stay in
open work under issue #510; no adapter claims any of these
classes yet.

The [doc-wide defaults posture](#minimal-required-core) applies to
enabled tools. Qualify these suggestions against [native configuration](../quality/native-configuration.md)
before freezing them; curated membership does not authorize hidden presets. The following existing native-configuration suggestions remain provisional
qualification inputs open, not approved presets or additions to curated membership:

- staticcheck default checks versus the `SA`-only suggestion above remain an
  unresolved conflict; neither is selected here. `govet` and errcheck for unhandled errors
  remain the existing complementary-tool suggestions, not new default selections
  (all provisional under issue #418).
- SpotBugs at default effort; PMD default ruleset; Error Prone at its default severities.
  The existing Checkstyle Google checks suggestion requires resolution against the
  native-config contract (open under issue #416); it is not an approved hidden or automatically supplied config.
- detekt `buildUponDefaultConfig` (full default set, not `allRules`);
  ktlint standard rules (both provisional under issue #416).
- Roslyn: SDK default analysis mode; StyleCop stays optional since its style
  rules can contradict the built-in IDE rules (both provisional under issue #417).
- clang-tidy default checks; cppcheck default enablement (both provisional under issue #418).
- Scalafix recommended built-ins plus OrganizeImports (import organization)
  and RemoveUnused (dead code): the existing suggestion remains provisional,
  open under issue #417. It is not an approved hidden preset;
  native-config and artifact qualification must resolve the conflict before implementation.
- FSharpLint default ruleset with formatting rules off (Fantomas owns
  formatting) (provisional under issue #417).
- `buf` `STANDARD` lint rules; qmlformat/qmllint `.qmlformat.ini`/`.qmllint.ini`
  discovery versus explicit flags (both provisional under issue #419).
- RuboCop `.rubocop.yml` versus the StandardRB unconfigurable ruleset; PSScriptAnalyzer
  settings-file versus explicit-rule selection; djlint, Stylelint, prettier-plugin, and
  yamllint rule selection; yamlfmt and keep-sorted config discovery (all provisional
  under issue #420).

Beyond-default switches (detekt `allRules`, experimental Error Prone checks,
`--enable=all`-style opt-in maxima) are not enabled by `rules_dx`. These
suggestions must be qualified against pinned native behavior and checked-in
configuration ownership (tracked in
open work under issues #485-#487);
this review selects neither a staticcheck rule set nor a Checkstyle/Scalafix preset.

### Provisional Default Dependency Locks

Upstream documentation observations; provisional defaults, not
selections. Exact files and fail-closed wiring are tracked in
open work under issues #481-#484.

- Java, Kotlin, Scala: `maven_install.json` via `rules_jvm_external`
  (`lock_file` plus `fail_if_repin_required`); Scala shares Java's Maven
  story. Coursier is the default resolver; the Maven and Gradle resolvers
  require a lock file by construction.
- C#, F#: Paket (`paket.dependencies` plus `paket.lock`) via `paket2bazel`
  into `nuget_repo` targets carrying per-package sha512. The Paket files are
  user-authored; generation consumes them and never writes them. NuGet's
  native `packages.lock.json` is unsuitable: one file per project instead of
  a central lock, and its hashes are incompatible with Bazel's downloader
  (rules_dotnet issue 444).
- Go: `go.mod`/`go.sum` via `go_deps.from_file` (`go.work` only for
  multi-module layouts); Gazelle documents `from_file` as preferred over
  hand-written module tags.
- C/C++: no ecosystem lockfile; every `http_archive` carries `sha256` or
  `integrity`, and system packages are excluded as non-hermetic.
- Every lockfile is committed; stale locks fail the build and CI
  (fail-closed): `fail_if_repin_required`, locked-mode restore semantics,
  go.sum verification, hash-checked archives. `MODULE.bazel.lock` is
  committed alongside.

### Provisional Adapter Input Notes

Upstream documentation observations; provisional input, not
selections. Adapter design and normalization for the JVM cohort is tracked under
issue #416; adapter design and normalization for the Scala + .NET cohort is tracked
under issue #417; adapter design and normalization for the Native cohort is tracked
under issue #418; adapter design and normalization for the Structured cohort is tracked
under issue #419; adapter design and normalization for the Interpreted/file-family
cohort is tracked under issue #420 (remaining cohorts stay in open work under issue #510).

Diagnostic wire formats:

- SARIF-native: PMD, Checkstyle (`-f sarif`), SpotBugs (`-sarif`),
  ktlint (`--reporter=sarif`), detekt (sarif report), staticcheck
  (`-f sarif`), Roslyn (`/errorlog`, SARIF 2.1). JVM shapes are unproven mappings
  owned by issue #416. Roslyn shapes are unproven mappings owned by issue #417.
  staticcheck shapes are unproven mappings owned by issue #418.
- JSON: staticcheck (`-f json`), PMD (json), ktlint (json).
  staticcheck JSON shapes are unproven mappings owned by issue #418.
  `buf lint --error-format=json` JSONL (`path,start_line,start_column,end_line,
  end_column,type,message`; no SARIF in 1.71.0) is an unproven mapping owned by
  issue #419; qmllint `--json` (messages plus file/line/severity) is an unproven
  mapping owned by issue #419. RuboCop `--format json` plus Stylelint
  `--formatter json` are unproven mappings owned by issue #420.
- Checkstyle XML: Checkstyle (`-f xml`), ktlint, detekt.
- Tool-native XML: cppcheck (`--xml --xml-version=2`, on stderr).
  Unproven mapping owned by issue #418.
- Text-parse: govet and errcheck (`file:line[:col]: message`, unproven mappings
  owned by issue #418), `buf` text (`file:line:column:message`, unproven mapping
  owned by issue #419), yamllint and djlint text diagnostics plus `terraform fmt
  -check -diff` plus keep-sorted output (unproven mappings owned by issue #420),
  RuboCop text plus PSScriptAnalyzer `Invoke-ScriptAnalyzer` console
  (`System.Management.Automation` library-API binding is the structured
  alternative, itemized under issue #420), Error Prone
  (javac diagnostics; structured output still open upstream, itemized under issue #416),
  FSharpLint
  console (the `FSharpLint.Core` library API is the structured
  alternative, itemized under issue #417), Scalafix console (no machine-readable CLI output; open
  upstream issue, itemized under issue #417).
- clang-tidy emits clang text diagnostics; fixes travel separately as
  `--export-fixes` YAML for `clang-apply-replacements`. Target-coupled wiring
  versus check-only stays open under issue #418.
- qmlformat emits the formatted file to stdout (whole-file rewrite); check/diff
  versus `--write`/`-i` wiring stays open under issue #419.

Fix modes:

- Whole-file rewrite with check/diff mode: all seven formatters,
  ktlint (`--format`), Scalafix (`--check` unified diff),
  clang-tidy (`--fix` or `--export-fixes`), Error Prone
  (`-XepPatchChecks` with a patch-file location). Native formatters
  (clang-format, gofumpt) plus clang-tidy modes are itemized under issue #418.
   Structured formatters (`buf format --diff --exit-code` plus `--write`,
   qmlformat stdout plus `-i`) are itemized under issue #419. Interpreted/file-family
   formatters (RuboCop plus `standardrb --fix`, cue `fmt`, jsonnetfmt, pkl, modfmt,
   `terraform fmt`, yamlfmt) are itemized under issue #420.
- Check-only: PMD, Checkstyle, SpotBugs, staticcheck, govet, errcheck,
  cppcheck, detekt, FSharpLint console. Native check-only tools (staticcheck,
  `govet`, errcheck, cppcheck) are itemized under issue #418. Structured
  check-only tools (`buf lint`, qmllint) are itemized under issue #419.
  Interpreted/file-family check-only tools (PSScriptAnalyzer, djlint, Stylelint,
  Prettier plugin closures, yamllint, keep-sorted) are itemized under issue #420.
- Provisional fix flow: tools run check-only for diagnostics; fixes are
  produced by applying in a sandbox and diffing, rendered as unified
  patches (precedent: aspect `rules_lint`).

Test result formats: JUnit XML (surefire/Gradle), xUnit XML,
`go test -json`, GoogleTest `--gtest_output=json|xml`, ScalaTest `-u`
JUnit XML. Normalization into the result contract is tracked in
open work under issue #511.

Open adapter risks tracked in
open work under issues #416-#420 and #490-#493:

- Scalafix: no machine-readable CLI output; semantic rules need semanticdb
  plus classpath wiring, and source rewriting sits uneasily with immutable
  action outputs (issue #417).
- Error Prone: no structured diagnostics; patch files need per-target
  declared outputs because `IN_PLACE` patching breaks under sandboxing (issue #416).
- Roslyn `/errorlog` SARIF is per compiler invocation (per
  configuration/TFM/RID pivot); aggregation work remains (issue #417).
- FSharpLint: console text parsing versus binding the .NET library API (issue #417).
- clang-tidy: compile-commands context couples the adapter to target wiring;
  target-coupled wiring versus check-only stays open (issue #418).
- staticcheck: default checks versus `SA`-only stays an unresolved conflict;
  neither is selected here (issue #418).
- cppcheck, `govet`, errcheck: tool-native XML on stderr plus text-parse shapes
  with the sandbox-apply-and-diff fix flow stay unproven mappings (issue #418).
- `buf`: native JSONL versus text-parse choice plus `STANDARD` rule selection and
  module-root-sensitive `PACKAGE_DIRECTORY_MATCH` plus `--path` scoping stay open
  (issue #419).
- qmlformat/qmllint: Qt distribution identity plus licensing plus platform artifacts,
  `.qmlformat.ini`/`.qmllint.ini` discovery versus explicit flags, and `--json`
  shape normalization stay open (issue #419).
- RuboCop/StandardRB: release-assembled closure contents plus lock inputs plus
  manifest/SBOM/licenses/provenance, with the bundle-vs-adapter qualification split
  (closure qualification versus parser plus runner-matrix plus native-config adapter
  qualification), stay open (issue #420).
- PSScriptAnalyzer: exact module version plus portable `pwsh` runtime identity plus
  console-parse versus library-API binding stay open (issue #420).
- File-family remainder (cue, jsonnetfmt, pkl, djlint, Stylelint, prettier-plugin
  closures, modfmt, terraform fmt, yamlfmt, yamllint, keep-sorted): exact pins/digests
  plus parser shapes plus Buildifier/Taplo/Vale probe promotion stay open (issue #420;
  `protobuf`/`qml` stay owned by issue #419).

### Remaining Work Tracking

The remaining review dimensions from
[first-release admission](scope.md#first-release-admission) are owned by their
existing trackers, not duplicated here: additional test runners (issues #476-#480) and
foundation mappings (issues #481-#484),
framework adapters by open work under issue #510, quality-tool
registry and policy by open work under issues #485-#489 and #512, audit
ecosystems by open work under issue #512 and update
ecosystems by open work under issue #510, and codegen pairs
by open work under issue #506 and
open work under issue #510. Newly identified
candidates still require an explicit disposition in the issue tracker.

### Cohort Tracking

Every candidate row above maps to an existing tracker: admitted Java and Kotlin
(JVM cohort) to issue #416; admitted Scala plus C# and F# (Scala + .NET cohort,
managed JVM plus exact-package shared-.NET-runtime routes decided
2026-09-13) to issue #417; admitted Go and
C/C++ (Native cohort, split native plus split Go routes decided
2026-09-13) to issue #418; structured `protobuf`/`qml` (Structured cohort,
checksummed `buf` artifact plus authoritative Qt distribution routes decided
2026-09-13, Qt last) to issue #419. Interpreted `ruby`/`powershell` plus the
file-family remainder (cue, jsonnet, pkl, css, html_template, gherkin, sql, xml,
go_module, terraform, yaml, text; Interpreted/file-family cohort with the
release-assembled Ruby closure plus exact-module portable-`pwsh` routes;
`protobuf`/`qml` cross-linked to issue #419, never double-claimed) map to issue
#420 (live successor to closed #307 for this cohort). Deferred foundations (Ruby,
PowerShell) are out of v1 scope; their retained tool cohorts (RuboCop/StandardRB via
the release-assembled Ruby closure, PSScriptAnalyzer via the exact-module plus
portable `pwsh` runtime) plus the file-family remainder stay tracked under issue
#420. If qualification
shows a cohort exceeds reviewable work-package size, it is split into
follow-up issues under the owning tracker before implementation.

## Quality And File Families

Applicable capabilities also enter the v1 feasibility review. Their application columns remain
explicit so formatter or linter availability cannot be mistaken for complete foundation support.
Applicability verification, including existing `N/A` cells, is owned by the registry
rather than inferred from a file suffix. File-family quality integrations
qualified seed-only under issue #489 (`bazel run //tools/ci:file_family_qualification`;
provider-class applicability with never-suffix inference, Starlark/Buildifier plus TOML/Taplo
adapter-backed evidence, parity-deferred routes with owner plus frozen acquisition per family;
adapter execution, exact pins/digests/rule-sets/adapter mappings per family, and platform plus
consumer plus release evidence stay owned gaps; no Supported claim).

| Source family | Build | Test | Dependencies | Generate | Environment | IDE | Coverage | Quality |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CUE | Planned: feasibility | Planned: feasibility | N/A | Planned: feasibility | Planned: feasibility | Planned: feasibility | Planned: feasibility | Planned: cue fmt |
| CUDA | Planned: feasibility | Planned: feasibility | Planned: feasibility | Planned: feasibility | Planned: feasibility | Planned: feasibility | Planned: feasibility | Planned: clang-format |
| CSS, Less, SCSS | N/A | N/A | N/A | N/A | N/A | N/A | N/A | Planned: Prettier and Stylelint |
| Gherkin | N/A | N/A | N/A | N/A | N/A | N/A | N/A | Planned: prettier-plugin-gherkin |
| GraphQL | N/A | N/A | N/A | Planned | N/A | N/A | N/A | Planned: Prettier |
| HTML | N/A | N/A | N/A | N/A | N/A | N/A | N/A | Planned: Prettier |
| HTML templates | N/A | N/A | N/A | N/A | N/A | N/A | N/A | Planned: djlint |
| JSON, JSON5, JSONC | N/A | N/A | N/A | N/A | N/A | N/A | N/A | Planned: Prettier |
| Jsonnet | Planned: feasibility | Planned: feasibility | Planned: feasibility | Planned: feasibility | Planned: feasibility | Planned: feasibility | Planned: feasibility | Planned: jsonnetfmt |
| Go modules | N/A | N/A | N/A | N/A | N/A | N/A | N/A | Planned: modfmt |
| Markdown | N/A | N/A | N/A | N/A | N/A | N/A | N/A | Planned: Prettier and Vale |
| Pkl | Planned: feasibility | Planned: feasibility | Planned: feasibility | Planned: feasibility | Planned: feasibility | Planned: feasibility | Planned: feasibility | Planned: pkl |
| Protocol Buffer | N/A | N/A | N/A | Planned | N/A | Planned: generated-source projection | N/A | Planned: buf format and lint |
| QML | Planned: feasibility | Planned: feasibility | Planned: feasibility | Planned: feasibility | Planned: feasibility | Planned: feasibility | Planned: feasibility | Planned: qmlformat and qmllint |
| Shell | Planned: feasibility | Planned: feasibility | N/A | N/A | N/A | N/A | Planned: feasibility | Planned: shfmt and ShellCheck |
| SQL | N/A | N/A | N/A | N/A | N/A | N/A | N/A | Planned: prettier-plugin-sql |
| Starlark | External: Bazel | External: Bazel | External: Bzlmod | Planned: feasibility | External: Bazel | Planned: Buildifier/editor integration | N/A | Planned: Buildifier |
| Terraform | Planned: feasibility | Planned: feasibility | Planned: feasibility | Planned: feasibility | Planned: feasibility | Planned: feasibility | Planned: feasibility | Planned: terraform fmt |
| TOML | N/A | N/A | N/A | N/A | N/A | N/A | N/A | Planned: Taplo format and lint |
| YAML | N/A | N/A | N/A | N/A | N/A | N/A | N/A | Planned: yamlfmt and yamllint |
| XML | N/A | N/A | N/A | N/A | N/A | N/A | N/A | Planned: prettier-plugin-xml |
| Language-independent text | N/A | N/A | N/A | N/A | N/A | N/A | N/A | Planned: keep-sorted |

Every cell above is current design status. The named quality integrations form the intended
minimum first-release quality baseline; the Linux x86_64 seed-host plus
Linux arm64 native (issue #410) plus the two static-musl profiles (issue
#411) plus macOS arm64 native (issue #412) plus macOS x86_64 best-effort
native (issue #413) plus Windows x86_64 MSVC-compatible native (issue
#414) implementations exist so far,
and no cell is `Supported` yet (release evidence tracked in the issue tracker). Swift and SwiftFormat are
excluded from v1 by [ADR 0019](../decisions/0019-first-release-additional-foundations.md):
the `Not planned` Swift row above is an evidence-backed v1 exclusion, not a
feasibility assessment. A host-toolchain
fallback was never approved. Reconsidering Swift after v1 requires a new scope decision.
Swift exclusion is decided by ADR 0019 with no open tracker.
