# Support Matrix

## Application Foundations

`Planned` means the capability is accepted scope but not implemented or verified.
`Planned: feasibility` means required v1 evaluation under [first-release admission](scope.md#first-release-admission):
admit qualifying capabilities, or record an approved disposition. Additional foundations may be
deferred when completion needs substantial infrastructure; required quality tools are independent.
Upstream mappings are not yet qualified; provisional candidates are recorded separately.
`Not planned` records an existing capability gap that O46 must reassess, not permission to defer a
qualifying integration. `Deferred beyond v1` records an evidence-backed deferral of an additional
foundation under [first-release admission](scope.md#first-release-admission); the row's quality-tool
cells stay in force and are unaffected by the deferral. `External` identifies an
existing Bazel/Bzlmod capability rather than a `rules_dx` support claim. `N/A` means the
capability does not apply to that source family. The repository [README](../../README.md) owns the
current project status; no cell is currently `Supported`. Status cells are owned by
this matrix, and promotion to `Supported` requires release evidence per the
[milestone index](../milestones/README.md#capability-terms).

`Audit` means source audit over declared source owners. Ecosystem dependency-vulnerability audit
coverage is separate and remains unresolved in [O11](../open-decisions.md); this matrix makes no
ecosystem audit-tool or dependency-source claim.

The following core and named framework foundations are required for v1 on every
[required platform](../decisions/0014-tested-platform-release-stack.md#required-platforms).
They are not eligible for the additional-foundation deferral policy.

| Language | Build | Test | Dependencies | Generate | Environment | IDE | Coverage | Format | Lint | Typecheck | Audit |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Rust | Planned | Planned | Planned: Cargo lock | Planned | Planned: native tools | Planned: rust-analyzer/flycheck | Planned | Planned: rustfmt | Planned: Clippy | Planned: compiler diagnostics | Not planned |
| Python | Planned | Planned: pytest | Planned: uv lock | Planned | Planned: `.venv` | Planned: interpreter/imports | Planned | Planned: Ruff | Planned: Ruff, pydoclint; flake8/pylint opt-in | Planned: Ty | Planned: O11-selected audit tools |
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
providers, or region mappings; those choices remain open in
[O29 and O40-O43](../open-decisions.md).

## Minimal Required Core Freeze (O46, Pre-M00)

This section is the O46 minimal required core/framework inventory freeze required before
M00. It records what is in, which provisional upstream each item builds on, which open
decision owns each remaining mapping, and which milestone delivers it. Exact pins, hosts,
and floors remain owned by [O14/O37](../open-decisions.md); no version
below is a release pin. Ownership: the sole repository maintainer owns every row until
maintenance is explicitly delegated. Effort beyond milestone assignment is estimated from
qualification evidence as each O-item lands; no person-hour figures are frozen here.

Quality-tool defaults posture, doc-wide (core rows above and admitted languages below):
enabled-tool membership follows the [curated baseline](../tools/tool-baseline.md#curated-differences),
including its explicit opt-ins; naming an integration here does not enable it by default.
Enabled tools use their pinned native defaults unless an applicable checked-in native config
supplies policy, per [Native Configuration](../quality/native-configuration.md#authority),
not a hidden `rules_dx` preset. Owning O-items qualify exact versions and configurations with
fixture evidence; provisional notes below do not select additional defaults or native presets.

| Foundation | Provisional upstream basis | Open mappings | Delivery |
| --- | --- | --- | --- |
| Rust application | `rules_rs` + pinned patched `rules_rust` ([ADR 0013](../decisions/0013-rust-javascript-typescript-foundations.md)) | O16 (providers, M02), O22 (Gazelle, M09), O24 (integration, M12) | M02 → M12 |
| Python application | `aspect_rules_py` 2.x ([ADR 0010](../decisions/0010-python-foundation.md)) | O25 (mappings, M14) | M14 |
| JavaScript/TypeScript application | `aspect_rules_js`/`aspect_rules_ts`, `aspect_rules_jest` ([ADR 0013](../decisions/0013-rust-javascript-typescript-foundations.md)) | O27 (wrappers/Gazelle, M16) | M16 |
| JS/TS quality | Biome default; ESLint, Prettier available; `tsc` diagnostic-only | O28 (mappings, M17) | M17 |
| Python quality | Ruff, Ty, pydoclint (Bandit excluded from v1 by [ADR 0019](../decisions/0019-first-release-additional-foundations.md)) | O26 (M15), O5/O6 (Ty, M15/M28) | M15 |
| Vue / Svelte / Astro / MDX | Named upstream adapters per framework | O29/O40/O41/O42 (M18–M21) | M18–M21 |
| Generation transport | Declared versioned manifest artifact | O13 (M10), O48 scoped syntax (M10) | M10 |
| Target resolution | Most-correct-first query strategy | O44 (M08) | M08 |
| Quality core/result contract | Internal Protobuf + NDJSON output | O15 (M02), O17 (M03), O18 (M03), O19 (M03) | M02–M05 |
| Required platforms | [Required-platform table](../decisions/0014-tested-platform-release-stack.md#required-platforms) | O14 (M00), O37 (M28) | M00 → M28 |
| Coverage gate | Instrumentation-first; behavioral fallback only on proof | Resolved in [coverage](../testing/README.md#coverage) | M00 |
| Consumer CI | Reusable workflow + caller template | CI mappings (M27) | M27 |
| Repository workflows | Codegen/env/setup (M25); audit/update (M26) | O11/O12 (M26), O33–O36 (M25) | M25–M26 |

Required platforms, the quality-tool baseline (minus excluded Swift), and the milestone
DAG M00–M30 are the delivery plan for this inventory. Admitted additional
foundations are owned by their cohort milestones (M22/M23) with closure in M24;
remaining feasibility detail stays in the
[feasibility review](#additional-v1-foundations) for M22/M23 qualification.
Swift is an evidence-backed v1 exclusion, not a pending assessment. Effort
beyond role + milestone assignment is estimated from qualification evidence as
each O-item lands; no person-hour figures are frozen here.

## Additional V1 Foundations

Admit/defer/exclude outcomes are decided by
[ADR 0019](../decisions/0019-first-release-additional-foundations.md) and
summarized in [Candidate Disposition Status](#candidate-disposition-status).
The review below records upstream evidence only. Include complete low-cost
upstream-backed foundations; substantial missing integration may justify an explicit approved
deferral under the admission policy. O46 must split each foundation into applicable build, test, dependency, generation,
environment, IDE, coverage, and quality cells with named upstreams and evidence. No row implies
that a ruleset has already been selected or that every capability/platform is feasible.
M22 owns native/toolchain foundations; M23 owns managed-runtime foundations. This is a minimum
inventory, not an exhaustive list of eligible languages.

### Admitted To V1

Java, Kotlin, C#, F#, Go, C/C++, and Scala application foundations are admitted
to v1 scope by [ADR 0019](../decisions/0019-first-release-additional-foundations.md), on the evidence in the
[candidate review](#additional-language-foundation-candidate-review): each has
an active Bzlmod-published upstream ruleset with a concrete dependency-lock
and toolchain story. Their quality integration is v1 scope with the defaults in
[Provisional Default Quality Tools](#provisional-default-quality-tools);
exact versions, rule sets, and adapter mappings continue under O30/O31. Delivery stays with the existing cohorts: Go and C/C++ in M22
(O30); Java, Kotlin, C#, and F# in M23 (O31); Scala after the M22
native/toolchain-versus-managed route decision (O30), then M22 or M23.
Unresolved cells block qualification; moving an admitted foundation out later
requires a new evidence-backed decision.

### Deferred Beyond V1

Ruby and PowerShell application foundations are deferred beyond v1 by
[ADR 0019](../decisions/0019-first-release-additional-foundations.md), with
evidence in the [candidate review](#additional-language-foundation-candidate-review):
Ruby's fragmented ruleset maintenance ownership and gem/bundler packaging
effort, and PowerShell's single young execution-only upstream with unproven
generation, dependency, environment, and IDE stories, exceed the low-cost
hermetic integration bar. Deferred foundations' quality-tool cells (RuboCop,
StandardRB, PSScriptAnalyzer) stay in force under O31; a foundation
deferral removes no baseline tool. Reconsideration after v1 requires a new
scope decision.

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

### Initial Feasibility Review

The approved review order starts with Go and C/C++. C/C++ initially assesses Bazel-native
projects; CMake/Meson integration is a separate review, not an automatic-migration promise.
This prioritization admits or defers no foundation and does not change milestone dependencies.

The following are upstream documentation/source observations, not executed qualification evidence.
Status cells are owned by this matrix; provisional toolchain and backend
choices are owned by the [native qualification plan](../native-toolchains.md). O46 must resolve the identified contract conflicts before affected implementation:

- [rules_go](https://github.com/bazel-contrib/rules_go) and
  [Gazelle](https://github.com/bazel-contrib/bazel-gazelle) offer SDK acquisition, package build/test,
  module dependencies, and generation. The approved narrow Go exception preserves package-level
  tests and declared platform/build constraints under the
  [generation contract](../generation/common.md#ownership-and-naming); exact mappings remain open.
  Source-only module identity, strict dependency resolution, and cgo/race scope remain unresolved.
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
does not admit an additional foundation or approve an implementation milestone.

The approved [Windows setup requirement](../decisions/0014-tested-platform-release-stack.md#decision)
allows explicit Microsoft EULA acknowledgement, as required by the inspected toolchains_msvc and
windows_support routes. This removes the prior setup-policy conflict, not the remaining acquisition,
licensing, interoperability, or coverage qualification gaps. Automatic acceptance is not approved;
the exact upstream mechanism and applicable usage/distribution rights remain under O46.

### Native Toolchain Alternatives

The review prioritizes existing upstream capability and minimal project-owned integration over a
universal compiler distribution. Linux cross-builds are the first priority, not a mandate to build
every target from every host. The [approved stack](../decisions/0014-tested-platform-release-stack.md#decision)
requires glibc and static-musl Linux profiles; runtime floors and qualified routes remain under review.
Broader cross-builds are desirable when upstream configuration keeps maintenance bounded.
Windows hermeticity is not relaxed to reduce setup or integration effort.

The following are source/documentation observations; no builds were executed:

| Candidate | Upstream capability | Tradeoff for general developer workflows |
| --- | --- | --- |
| [hermetic-llvm v0.8.19](https://github.com/hermeticbuild/hermetic-llvm/blob/v0.8.19/README.md) | Integrated Linux glibc and static-only musl targets on x86_64/arm64, Apple SDK acquisition, LLVM runtime and tool targets, and Linux/macOS coverage fixtures. | Strongest integrated primary candidate with the preferred Rust rules. Dynamic musl is not supported. Default libc++ does not imply compatibility with prebuilt libstdc++ libraries; its Linux-glibc libstdc++ route is dynamic-only. Released Windows support does not satisfy the approved baseline. |
| [uber/hermetic_cc_toolchain v4.3.0](https://github.com/uber/hermetic_cc_toolchain/tree/v4.3.0) | Zig 0.15.2 supplies Linux glibc/musl cross-compilation and GNU/MinGW Windows targets. | Strong Linux alternative, but documented Apple SDK gaps, an external Zig runtime cache, and missing integrated Bazel coverage/Clang developer tools increase integration ownership. Shared-musl capability is not complete Rust/runtime-deployment evidence. |
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
O24 still requires fixture-proven upstream mappings, not automatic compatibility with every crate.

The approved direction is to qualify rules_cc and rules_rs with hermetic-llvm for Linux glibc,
static-musl, and macOS. Windows first qualifies the toolchains_msvc clang-cl/Microsoft-STL route;
adoption remains blocked on upstream acquisition, interoperability, and coverage evidence.
Zig remains a comparison candidate where it supplies a concrete benefit. Do not implement missing
dynamic-musl, coverage, SDK, or compiler-stack infrastructure merely to fill the cross-product.
Apple/Microsoft acquisition rights and the existing managed-tool packaging/laziness contracts remain
gates even when upstream technically supports a route. O24/O46 must resolve the evidence and any
required contract changes before implementation; no exact pins or support claims are established.

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
provisional ruleset choices below are candidates for O30/O31 qualification,
not selections. This review itself admits, defers, or excludes nothing;
outcomes are decided by
[ADR 0019](../decisions/0019-first-release-additional-foundations.md) and
summarized in [Candidate Disposition Status](#candidate-disposition-status).

- Java: [`rules_jvm_external` 7.x](https://github.com/bazel-contrib/rules_jvm_external)
  (Bzlmod, Coursier/Maven/Gradle resolvers, checked-in `maven_install.json`
  checksum lockfile) is the provisional Maven-dependency route over the bundled
   `rules_java` toolchain. Bazel documents a hermetic compile route via
   `remote_java_repository` and `--java_runtime_version` flags; per-platform JDK
   acquisition and coverage integration remain open under O31 (M23); lock
   authority is `maven_install.json`
   ([Provisional Default Dependency Locks](#provisional-default-dependency-locks)).
   Admitted to v1 by [ADR 0019](../decisions/0019-first-release-additional-foundations.md).
- Kotlin: [`rules_kotlin` 2.4.10](https://registry.bazel.build/modules/rules_kotlin)
  (Bzlmod) is the provisional build/test route on the same
  JVM toolchain and Maven-lock story as Java. Kotlin compiler acquisition,
  worker behavior, and IDE projection remain open under O31 (M23). Admitted to
  v1 by [ADR 0019](../decisions/0019-first-release-additional-foundations.md).
- Scala: [`rules_scala` 7.x](https://github.com/bazel-contrib/rules_scala)
  (bazel-contrib, Bazel 7/8 plus Bzlmod, Coursier-backed Scala
  toolchains) is the provisional route. The M22 native/toolchain-versus-managed
  route decision stands: Coursier-fetched toolchains argue managed, while
  source-built or toolchain-coupled options argue native. Mappings and the route
  decision remain open under O30, with M23 consuming the outcome. Admitted to v1
  by [ADR 0019](../decisions/0019-first-release-additional-foundations.md).
- C# and F#: [`rules_dotnet` 0.22.2](https://github.com/bazel-contrib/rules_dotnet)
  (Bzlmod, Bazel >= 8) covers both languages with `csharp_*`
   and `fsharp_*` targets plus SDK acquisition through its `dotnet` toolchain
   extension, so one upstream review covers both rows. Per-platform SDK
   acquisition and test-runner versions remain open under O31 (M23); lock
   authority is the Paket lock
   ([Provisional Default Dependency Locks](#provisional-default-dependency-locks)).
   C# and F# admitted to v1 by [ADR 0019](../decisions/0019-first-release-additional-foundations.md).
- Ruby: [`rules_ruby`](https://github.com/bazel-contrib/rules_ruby) (Bazel 8,
  Bzlmod, published to the Bazel Central Registry) is the provisional route,
  but the ruleset history is fragmented (retired `bazelruby/rules_ruby`,
  several forks) and canonical maintenance ownership is the thinnest of this
  cohort. Gem/bundler integration and the packaging-effort boundary stay the
  cost drivers under O31; M23 already bounds Ruby as an exceptional bundle
  within the O46-approved effort. Mappings and the effort boundary remain open
  under O31 (M23). Foundation deferred beyond v1 by [ADR 0019](../decisions/0019-first-release-additional-foundations.md);
  the RuboCop/StandardRB tool cohort stays under O31 (M23).
- PowerShell: [`rules_powershell` 0.2.0](https://github.com/periareon/rules_powershell)
  (in the Bazel Central Registry, built over `rules_shell`) is the only known
  upstream execution route and the youngest of this cohort. Generation,
  dependency, environment, and IDE stories for `.ps1` application foundations
  are unproven, and PSScriptAnalyzer integration is unassessed; this row
  carries the highest deferral risk if qualification shows substantial missing
  infrastructure. All mappings remain open under O31 (M23). Foundation deferred
  beyond v1 by [ADR 0019](../decisions/0019-first-release-additional-foundations.md); the PSScriptAnalyzer tool cohort
  stays under O31 (M23).

### Candidate Disposition Status

| Candidate | Provisional basis | Delivery owner | Disposition |
| --- | --- | --- | --- |
| Java | `rules_jvm_external` + `rules_java` | M23 (O31) | Admitted to v1 ([ADR 0019](../decisions/0019-first-release-additional-foundations.md)) |
| Kotlin | `rules_kotlin` | M23 (O31) | Admitted to v1 ([ADR 0019](../decisions/0019-first-release-additional-foundations.md)) |
| Scala | `rules_scala` | M22 route decision (O30), then M22 or M23 | Admitted to v1 ([ADR 0019](../decisions/0019-first-release-additional-foundations.md)) |
| C# | `rules_dotnet` | M23 (O31) | Admitted to v1 ([ADR 0019](../decisions/0019-first-release-additional-foundations.md)) |
| F# | `rules_dotnet` | M23 (O31) | Admitted to v1 ([ADR 0019](../decisions/0019-first-release-additional-foundations.md)) |
| Ruby | `rules_ruby` (bazel-contrib) | M23 tool cohort (O31); foundation post-v1 | Deferred beyond v1 ([ADR 0019](../decisions/0019-first-release-additional-foundations.md)) |
| PowerShell | `rules_powershell` | M23 tool cohort (O31); foundation post-v1 | Deferred beyond v1 ([ADR 0019](../decisions/0019-first-release-additional-foundations.md)) |
| Go, C/C++ | Source/documentation review | M22 (O30) | Admitted to v1 ([ADR 0019](../decisions/0019-first-release-additional-foundations.md)) |
| Swift, Bandit | Excluded from v1 ([ADR 0019](../decisions/0019-first-release-additional-foundations.md)) | N/A | Evidence-backed v1 exclusion |

No person-hour figures are frozen here; effort is estimated from qualification
evidence as each O-item lands, per the freeze section above. A deferral or
exclusion recorded during M22/M23 qualification must be evidence-backed, not
silent.

### Provisional Default Test Runners

Upstream documentation observations; provisional defaults, not
selections. Exact versions, runner mappings, and
fixture qualification stay under O30/O31.

- Java and Kotlin: JUnit (≈62–79% JVM adoption, Spring Boot default; JUnit 6
  adds native Kotlin `suspend` support). JUnit 5 versus 6 follows the O31 JDK
  baseline; the provisional pin on the 6.x line is 6.1.3,
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
  which the O30 toolchain floor must cover. Upstream recommends living at
  head; dx pins the release.
- Scala: ScalaTest 3.2.20 (covers Scala 2.10–2.13 and 3.x),
  per the rules' own preference (`scala_test` runs suites written using the
  `scalatest` library; the rule implementation is ScalaTest-wired).
  `scala_junit_test` and `scala_specs2_junit_test` are rules-supported
  choices, not defaults. Scalactic is the recommended companion, not a
  requirement.

Where the upstream rules support multiple runners, the runner is a
configuration choice over the O30/O31-qualified set, not a hard-coded single
framework; the defaults above apply when unconfigured. Secondary runners
(TestNG, NUnit/MSTest, Catch2, specs2/JUnit variants) are therefore
choice candidates pending qualification evidence, not exclusions and not
v1 commitments.

### Provisional Default Quality Tools

Upstream documentation observations; provisional defaults, not
selections. Exact versions, rule sets, and adapter mappings stay under
O30/O31.

- Go: gofumpt (strict superset of gofmt) for format; staticcheck plus `govet`
  for lint, since neither subsumes the other. The existing `SA`-only suggestion
  conflicts with the default-check suggestion below; both remain provisional under O30,
  not an approved native rule selection.
- C/C++: clang-format, clang-tidy, and cppcheck as already named;
  compiler-integrated and standalone analysis are complementary.
- Java: google-java-format; PMD, Checkstyle, SpotBugs, plus Error Prone, which
  works out of the box with Bazel. The Error Prone version follows the O31
  JDK baseline.
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

The [doc-wide defaults posture](#minimal-required-core-freeze-o46-pre-m00) applies to
enabled tools. Qualify these suggestions against [native configuration](../quality/native-configuration.md)
before freezing them; curated membership does not authorize hidden presets. The following existing native-configuration suggestions remain provisional
qualification inputs under O30/O31, not approved presets or additions to curated membership:

- staticcheck default checks versus the `SA`-only suggestion above remain an O30
  conflict; neither is selected here. `govet` and errcheck for unhandled errors
  remain the existing complementary-tool suggestions, not new default selections.
- SpotBugs at default effort; PMD default ruleset; Error Prone at its default severities.
  The existing Checkstyle Google checks suggestion requires O31 resolution against the
  native-config contract; it is not an approved hidden or automatically supplied config.
- detekt `buildUponDefaultConfig` (full default set, not `allRules`);
  ktlint standard rules.
- Roslyn: SDK default analysis mode; StyleCop stays optional since its style
  rules can contradict the built-in IDE rules.
- clang-tidy default checks; cppcheck default enablement.
- Scalafix recommended built-ins plus OrganizeImports (import organization)
  and RemoveUnused (dead code): the existing suggestion remains provisional under O30,
  with O31 owning any managed-runtime route. It is not an approved hidden preset;
  native-config and artifact qualification must resolve the conflict before implementation.
- FSharpLint default ruleset with formatting rules off (Fantomas owns
  formatting).

Beyond-default switches (detekt `allRules`, experimental Error Prone checks,
`--enable=all`-style opt-in maxima) are not enabled by `rules_dx`. O30/O31 must qualify
these suggestions against pinned native behavior and checked-in configuration ownership;
this review selects neither a staticcheck rule set nor a Checkstyle/Scalafix preset.

### Provisional Default Dependency Locks

Upstream documentation observations; provisional defaults, not
selections. Exact files and fail-closed wiring freeze under O30/O31.

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
selections. Adapter design and normalization freeze under O30/O31.

Diagnostic wire formats:

- SARIF-native: PMD, Checkstyle (`-f sarif`), SpotBugs (`-sarif`),
  ktlint (`--reporter=sarif`), detekt (sarif report), staticcheck
  (`-f sarif`), Roslyn (`/errorlog`, SARIF 2.1).
- JSON: staticcheck (`-f json`), PMD (json), ktlint (json).
- Checkstyle XML: Checkstyle (`-f xml`), ktlint, detekt.
- Tool-native XML: cppcheck (`--xml --xml-version=2`, on stderr).
- Text-parse: govet and errcheck (`file:line[:col]: message`), Error Prone
  (javac diagnostics; structured output still open upstream), FSharpLint
  console (the `FSharpLint.Core` library API is the structured
  alternative), Scalafix console (no machine-readable CLI output; open
  upstream issue).
- clang-tidy emits clang text diagnostics; fixes travel separately as
  `--export-fixes` YAML for `clang-apply-replacements`.

Fix modes:

- Whole-file rewrite with check/diff mode: all seven formatters,
  ktlint (`--format`), Scalafix (`--check` unified diff),
  clang-tidy (`--fix` or `--export-fixes`), Error Prone
  (`-XepPatchChecks` with a patch-file location).
- Check-only: PMD, Checkstyle, SpotBugs, staticcheck, govet, errcheck,
  cppcheck, detekt, FSharpLint console.
- Provisional fix flow: tools run check-only for diagnostics; fixes are
  produced by applying in a sandbox and diffing, rendered as unified
  patches (precedent: aspect `rules_lint`). O30/O31 confirm.

Test result formats: JUnit XML (surefire/Gradle), xUnit XML,
`go test -json`, GoogleTest `--gtest_output=json|xml`, ScalaTest `-u`
JUnit XML. O30/O31 normalize these into the result contract.

Open adapter risks for O30/O31:

- Scalafix: no machine-readable CLI output; semantic rules need semanticdb
  plus classpath wiring, and source rewriting sits uneasily with immutable
  action outputs.
- Error Prone: no structured diagnostics; patch files need per-target
  declared outputs because `IN_PLACE` patching breaks under sandboxing.
- Roslyn `/errorlog` SARIF is per compiler invocation (per
  configuration/TFM/RID pivot); aggregation is O31 work.
- FSharpLint: console text parsing versus binding the .NET library API.

### O46 Review Delegation

The remaining O46 review dimensions from
[first-release admission](scope.md#first-release-admission) are owned by their
existing decisions, not duplicated here: additional test runners by O30/O31
qualification, framework adapters by the closed v1 set (O43), quality-tool
plugins and sharing by O31, audit/update ecosystems by O11/O12, and codegen
pairs by O33. Newly identified candidates still require an explicit
disposition under the admission policy.

### Milestone Cohort Determination

Every candidate row above maps to an existing cohort owner: admitted Go and
C/C++ to M22 under O30; admitted Java, Kotlin, C#, F#, and Scala (after the
M22 route decision) to M22/M23 under O30/O31. Deferred foundations (Ruby,
PowerShell) are out of v1 scope; their retained tool cohorts stay with their
O-item owners. No milestone split or DAG change is required by this review. Splits stay conditional: if O30/O31 qualification
shows a cohort exceeds reviewable work-package size, O46 splits the affected
specification and updates the milestone index before implementation.

## Quality And File Families

Applicable capabilities also enter the v1 feasibility review. Their application columns remain
explicit so formatter or linter availability cannot be mistaken for complete foundation support.
O46 must verify applicability, including existing `N/A` cells, rather than infer it from a file suffix.

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
minimum first-release quality baseline, but none is implemented yet. Swift and SwiftFormat are
excluded from v1 by [ADR 0019](../decisions/0019-first-release-additional-foundations.md):
the `Not planned` Swift row above is an evidence-backed v1 exclusion, not a
feasibility assessment awaiting O46. A host-toolchain
fallback was never approved. Reconsidering Swift after v1 requires a new scope decision.
