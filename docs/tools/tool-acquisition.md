# Tool Acquisition

## Consumer Contract

`rules_dx` supplies every managed quality tool as a pinned, hermetic, ready-to-run Bazel
dependency. A consumer adds one `rules_dx` module dependency and uses the automatically available
foundations and curated tools;
it does not separately select quality-tool acquisition strategies, versions, runtimes,
package locks, plugin sets, URLs, checksums, or executable labels. A repository may still
select its application compiler/runtime versions under the authoritative language-
toolchain contract; coupled quality tools follow that selection automatically.

Consumer workflows never search ambient PATH, invoke an ecosystem installer, solve a tool
dependency graph, compile a managed tool, or download from an action. Repository setup may
use Bazel's downloader and established language-rule repository logic to fetch exact
artifacts from a `rules_dx`-owned lock. It must not invoke `pip install`, `npm install`,
`cargo install`, `dotnet tool install`, Bundler, or an equivalent installer.

The public experience is uniform even though upstream tools have different natural
distributions. An adapter always receives a declared executable, its complete runtime
inputs, deterministic environment, and version identity. Acquisition details remain
private implementation details.

The [Windows native-toolchain baseline](../decisions/0014-tested-platform-release-stack.md#decision)
also requires hermetic acquisition. Adapters requiring Windows compiler or SDK context must obtain
its qualified identity and declared inputs through the selected upstream toolchain, not an installed
host SDK. Acquisition and cache/remote behavior require the
[platform evidence](../testing/tools.md#pins-and-platform-tests), not a blanket hermeticity claim.

The contract is a requirement for the complete first-release tool baseline and mandatory
curated expansion under [First-Release Admission](../product/scope.md#first-release-admission)
(candidate review open), on every
[required platform](../decisions/0014-tested-platform-release-stack.md#required-platforms).
Swift and SwiftFormat are excluded from v1 by
[ADR 0019](../decisions/0019-first-release-additional-foundations.md); see
[Swift Feasibility](tool-baseline.md#swift-feasibility).

This contract does not state that any integration is currently implemented, dogfooded,
adapter-tested, or supported. A delivery class is accepted only after its required proof
fixtures pass; a plausible design or an entry in the tool matrix is not implementation evidence.
No adapter claims protobuf, qml, java, kotlin, scala, csharp, fsharp, powershell, ruby, or
c/cpp yet; required-core plus Buildifier/Taplo/Vale probes are qualified under issue #307
with parity manifest plus regeneration plus packaging evidence (deferred implementation owned
by ADR 0019; Buildifier/Taplo/Vale probes stay provisional).

## Bootstrap Maintenance And Promotion

There are no bootstrap-maintained seed tools and no seed-to-adapter
parity gate. Quality, audit, CI, and other integrations land directly through their
product adapters when the owning issue implements them; gaps are accepted until
then. Nothing in this section authorizes a temporary direct-tool check, fallback
wrapper, or second acquisition path.

Product adapters must independently satisfy the managed acquisition matrix in
[Tool And Platform Test Matrix](../testing/tools.md) and the [consumer contract](#consumer-contract).
A product adapter becomes dogfooded only when this repository and CI use the intended
product path. A maintenance-only check may remain separate when it has no product-adapter role.

### Initial Dogfood Routing

These routes define dogfood ownership; they do not claim current availability
or support. There is no seed path and no parity step: each adapter lands directly.

| Repository check | Product route | Disposition |
| --- | --- | --- |
| rustfmt, Clippy, and rustc typecheck | Authoritative selected Rust toolchain | Land directly through the selected Rust toolchain identity. |
| Buildifier, Taplo, and Vale | Checksummed standalone upstream artifacts | Land directly through each exact platform artifact and digest. |
| Markdown link and structure validation | None; repository maintenance only | Retain as a narrow repository check, separate from Vale and outside the product adapter baseline. |

Prettier and the private Node tool graph are not bootstrap or initial-dogfood dependencies.
They remain in the frozen first-release baseline but are deferred to
JavaScript And TypeScript Quality.

### Initial Artifact Research

Read-only upstream research supports the following candidates (see
open work under issues #416-#420), not qualified
pins or platform support. Recheck latest stable and verify actual bytes when adding each adapter.
The JVM cohort rows below are owned by issue #416 (live successor to closed
#307 for the `java`/`kotlin` classes); versions are observations, not pins.
The Scala + .NET cohort rows below are owned by issue #417 (live successor
to closed #307 for the `scala`/`csharp`/`fsharp` classes); versions are
observations, not pins.
The Native cohort rows below are owned by issue #418 (live successor
to closed #307 for the `c`/`cpp`/`go` classes); versions are
observations, not pins.
The Structured cohort rows below are owned by issue #419 (live successor
to closed #307 for the `protobuf`/`qml` classes); versions are
observations, not pins.
The Interpreted/file-family cohort rows below are owned by issue #420 (live successor
to closed #307 for the `ruby`/`powershell` plus `cue`/`jsonnet`/`pkl`/`css`/
`html_template`/`gherkin`/`sql`/`xml`/`go_module`/`terraform`/`yaml`/`text`
classes; `protobuf`/`qml` stay owned by issue #419 and are cross-linked, not
double-claimed); versions are observations, not pins.

| Tool | Upstream evidence | Candidate acquisition and remaining risk |
| --- | --- | --- |
| Buildifier | [v8.5.1 assets](https://github.com/bazel-contrib/buildtools/releases/tag/v8.5.1) and [release targets](https://github.com/bazel-contrib/buildtools/blob/v8.5.1/buildifier/BUILD.bazel) | Raw platform executable from `bazel-contrib/buildtools`; pure-Go build intent still requires artifact linkage, runtime, and platform evidence. |
| Taplo | [0.10.0 assets](https://github.com/tamasfe/taplo/releases/tag/0.10.0) and [release workflow](https://github.com/tamasfe/taplo/blob/0.10.0/.github/workflows/releases.yaml) | Gzip executable on Linux/macOS, ZIP on Windows. Bazel supports [gzip extraction](https://bazel.build/rules/lib/builtins/repository_ctx#download_and_extract); qualify member naming/mode with the seed Bazel. Reviewed release metadata supplied no asset digest/checksum file, so maintainer acquisition must establish and record byte identity. |
| Vale | [v3.20.0 assets](https://github.com/vale-cli/vale/releases/tag/v3.20.0) and [release configuration](https://github.com/vale-cli/vale/blob/v3.20.0/.goreleaser.yml) | Platform archive plus published checksums from `vale-cli/vale`. Linux CGO/GNU builds are not evidence of static-musl compatibility; inspect the actual runtime closure before admission. |
| google-java-format | [v1.35.0 Maven Central](https://repo1.maven.org/maven2/com/google/googlejavaformat/google-java-format/) (verified Mar 2026; v1.36.x observed on [GitHub releases](https://github.com/google/google-java-format/releases)) | Complete upstream all-deps JAR (`google-java-format-*-all-deps.jar`) over the one managed JDK cohort; no Maven-module reconstruction, no installer/solver/compiler on the consumer path. Reviewed release metadata supplies no Bazel-ready digest, so maintainer acquisition must establish and record byte identity and recheck latest stable when adding the adapter. |
| Checkstyle | [14.1.0 release](https://github.com/checkstyle/checkstyle/releases) (current per [checkstyle.org](https://checkstyle.org/)) | Complete upstream all-deps JAR (`checkstyle-*-all.jar`) over the one managed JDK cohort; SARIF via `-f sarif`. Maintainer acquisition must establish and record byte identity; recheck latest stable when adding the adapter. |
| PMD | [7.27.0 release notes](https://docs.pmd-code.org/latest/pmd_release_notes.html) (Aug 2026; [7.26.0 binary](https://github.com/pmd/pmd/releases)) | Complete upstream binary distribution (`pmd-dist-*-bin.zip`) over the one managed JDK cohort; SARIF via `-f sarif`. Maintainer acquisition must establish and record byte identity; recheck latest stable when adding the adapter. |
| SpotBugs | [4.10.4 releases](https://github.com/spotbugs/spotbugs/releases) (Aug 2026; [4.10.3 checksums](https://github.com/spotbugs/spotbugs/releases/tag/4.10.3)) | Complete upstream binary distribution (`spotbugs-*.zip`/`*.tgz`) over the one managed JDK cohort; SARIF via `-sarif`. Default effort stays a provisional native-config input until qualified. Maintainer acquisition must establish and record byte identity. |
| ktfmt | [v0.63 release](https://github.com/facebook/ktfmt/releases/tag/v0.63) (May 2026; v0.64 observed on Maven Central) | Complete upstream with-dependencies JAR (`ktfmt-*-with-dependencies.jar`) over the one managed JDK cohort. Maintainer acquisition must establish and record byte identity; recheck latest stable when adding the adapter. |
| ktlint | [1.8.0 release](https://github.com/ktlint/ktlint/releases/tag/1.8.0) (Nov 2025 stable; 2.0.0 alphas are not stable) | Complete upstream executable JAR (`ktlint` release asset, runnable as `java -jar ktlint`) over the one managed JDK cohort; SARIF via `--reporter=sarif`. Standard rules stay a provisional native-config input until qualified. Maintainer acquisition must establish and record byte identity. |
| detekt | [v1.23.8 stable](https://github.com/detekt/detekt/releases) (Feb 2025; 2.0.0 alphas target JDK 25 and are not stable) | Complete upstream CLI (`detekt-cli-*-all.jar`) over the one managed JDK cohort; SARIF report plus Checkstyle XML. `buildUponDefaultConfig` (not `allRules`) stays a provisional native-config input until qualified; type resolution is optional. Maintainer acquisition must establish and record byte identity. |
| Error Prone | [v2.50.0 release](https://github.com/google/error-prone/releases/tag/v2.50.0) (Jun 2026) | Javac plugin following the qualified JDK baseline (version coupled to the JDK, open under #416); no structured diagnostics upstream, so javac-diagnostic parsing is open work under issue #416 itemized here, not silently dropped. Patch files need per-target declared outputs because `IN_PLACE` patching breaks under sandboxing. |
| Scalafmt | [releases](https://github.com/scalameta/scalafmt/releases) (v3.11.4 observed Jul 2026) | Compatible JVM artifact over the one managed JDK cohort plus the Scala Maven-lock story (`maven_install.json` plus `fail_if_repin_required`); no source-built or toolchain-coupled route. Maintainer acquisition must establish and record byte identity and recheck latest stable when adding the adapter. |
| Scalafix | [v0.14.7 release](https://github.com/scalacenter/scalafix/releases/tag/v0.14.7) (Jun 2026) | Semantic-rule artifacts over the one managed JDK cohort plus the Scala Maven-lock story; semantic rules additionally need semanticdb plus classpath wiring per the adapter-input notes. No machine-readable CLI output upstream, so console-parse versus wire decision is open work under issue #417 itemized here, not silently dropped. Maintainer acquisition must establish and record byte identity. |
| CSharpier | [1.3.0 release](https://github.com/belav/csharpier/releases/tag/1.3.0) (Jun 2026; [NuGet](https://www.nuget.org/packages/CSharpier)) | Exact official tool package executed as declared DLLs over the one managed .NET runtime cohort (1.3.0 targets .NET 8.0); no consumer runs `dotnet tool install` or any equivalent installer. Runtime compatibility bounds stay open until qualified. Maintainer acquisition must establish and record byte identity. |
| Fantomas | [releases](https://github.com/fsprojects/fantomas/releases) (v7.x stable line; 8.0.0 alphas target the next FSharp.Core/.NET and are not stable) | Exact official tool package executed as declared DLLs over the one managed .NET runtime cohort; no consumer runs `dotnet tool install` or any equivalent installer. Runtime compatibility bounds stay open until qualified. Maintainer acquisition must establish and record byte identity. |
| FSharpLint | [0.27.0 release](https://github.com/fsprojects/FSharpLint/releases) (Jun 2026; targets .NET 8.0) | Exact official tool package over the one managed .NET runtime cohort; console text parsing versus binding the `FSharpLint.Core` library API is open work under issue #417 itemized here, not silently dropped. Default ruleset with formatting rules off (Fantomas owns formatting) stays a provisional native-config input until qualified. Maintainer acquisition must establish and record byte identity. |
| clang-format | [hermetic-llvm v0.8.19](https://github.com/hermeticbuild/hermetic-llvm/blob/v0.8.19/README.md) (inspected release; LLVM tool targets) | Authoritative-toolchain class: resolves from the qualified hermetic-llvm LLVM distribution's tool targets, no separate acquisition. Version follows the qualified toolchain pin; recheck the qualified LLVM distribution when adding the adapter. |
| clang-tidy | [hermetic-llvm v0.8.19](https://github.com/hermeticbuild/hermetic-llvm/blob/v0.8.19/README.md) (inspected release; LLVM tool targets) | Authoritative-toolchain class: resolves from the qualified hermetic-llvm LLVM distribution's tool targets, no separate acquisition. clang-tidy needs compile-commands context — target-coupled wiring versus check-only stays open work under issue #418 itemized here, not silently dropped. Version follows the qualified toolchain pin. |
| cppcheck | [2.21.0 release](https://github.com/cppcheck-opensource/cppcheck/releases) (Jun 2026; [2.21 sources](https://sourceforge.net/projects/cppcheck/files/cppcheck/2.21/)) | Checksummed standalone release artifact; `--xml --xml-version=2` on stderr is the machine-readable shape. Default enablement stays a provisional native-config input until qualified. Maintainer acquisition must establish and record byte identity and recheck latest stable when adding the adapter. |
| gofumpt | [v0.11.0](https://github.com/mvdan/gofumpt/releases) (observed Jul 2026; strict superset of gofmt) | Checksummed standalone release artifact; whole-file rewrite with check/diff mode. Maintainer acquisition must establish and record byte identity and recheck latest stable when adding the adapter. |
| staticcheck | [2026.2 release notes](https://staticcheck.dev/changes/2026.2) (Aug 2026; v0.8.1 observed) | Checksummed standalone release artifact; SARIF via `-f sarif`, JSON via `-f json`. Default checks versus the `SA`-only suggestion stay an unresolved conflict — neither is selected here; qualify before freezing. Maintainer acquisition must establish and record byte identity and recheck latest stable when adding the adapter. |
| govet | [Go 1.27.1](https://go.dev/doc/devel/release) (Sep 2026; ships with the Go toolchain) | Authoritative Go toolchain class: `govet` ships with the toolchain, no separate acquisition. Text-parse shape (`file:line[:col]: message`); check-only with the provisional sandbox-apply-and-diff fix flow. Version follows the qualified Go toolchain pin. |
| errcheck | [v1.20.0 release](https://github.com/kisielk/errcheck/releases) (May 2026) | Checksummed standalone release artifact; text-parse shape (`file:line[:col]: message`); check-only with the provisional sandbox-apply-and-diff fix flow. Complementary to `govet` for unhandled errors, not a default selection. Maintainer acquisition must establish and record byte identity and recheck latest stable when adding the adapter. |
| buf | [releases](https://github.com/bufbuild/buf/releases) (v1.72.0 observed Jul 2026; [1.71.0 npm](https://www.npmjs.com/package/@bufbuild/buf)) | Checksummed standalone release artifact; self-contained per-platform binaries with published checksums (Apache-2.0). `buf format --diff --exit-code` unified diff plus `--write`; `buf lint --error-format=json` JSONL (`path,start_line,start_column,end_line,end_column,type,message`) plus text `file:line:column:message` (no SARIF in 1.71.0). Needs no target compiler context; execution-platform lazy. Maintainer acquisition must establish and record byte identity and recheck latest stable when adding the adapter. |
| qmlformat | [qmlformat docs](https://doc.qt.io/qt-6/qtqml-tooling-qmlformat.html) (Qt 6.11.2 observed; [Qt 6.8 tooling](https://doc.qt.io/qt-6.8/qtqml-tooling.html)) | Authoritative-toolchain class: resolves from the qualified Qt distribution's tool targets, no separate acquisition. Whole-file rewrite with stdout plus `-i` inplace; `.qmlformat.ini` upward settings plus `--ignore-settings`. Version follows the qualified Qt distribution pin; exact Qt distribution identity, licensing, and platform artifact qualification remain pending. |
| qmllint | [qmllint docs](https://doc.qt.io/qt-6/qtqml-tooling-qmllint.html) (Qt 6.11.1 observed) | Authoritative-toolchain class: resolves from the qualified Qt distribution's tool targets, no separate acquisition. `--json <file>` (`-` for stdout) JSON with messages plus file/line/severity; `.qmllint.ini` plus `//qmllint enable/disable` comments. Check-only with the provisional sandbox-apply-and-diff fix flow. Version follows the qualified Qt distribution pin; exact distribution identity, licensing, and platform artifact qualification remain pending. |
| RuboCop | [releases](https://github.com/rubocop/rubocop/releases) (1.91.0 observed Sep 2026; [RubyGems](https://rubygems.org/gems/rubocop)) | Release-assembled Ruby closure member (gem plus transitive closure over the assembled Ruby runtime); consumer never runs Bundler or any installer. `--format json` versus text-parse stays open under issue #420. Bundle contents, lock inputs, manifest, SBOM, licenses, constituent provenance, and adapter qualification remain pending under issue #420; maintainer assembly must establish and record them and recheck latest stable when adding the adapter. |
| StandardRB | [releases](https://github.com/standardrb/standard/releases) (1.56.0 observed Jul 2026; [RubyGems](https://rubygems.org/gems/standard)) | Release-assembled Ruby closure member over the same assembled Ruby runtime (`standardrb` binary wrapping RuboCop with an unconfigurable ruleset); consumer never runs Bundler or any installer. Whole-file rewrite with check/diff mode; exact fix wiring stays open under issue #420. Bundle contents, lock inputs, and adapter qualification remain pending under issue #420; recheck latest stable when adding the adapter. |
| PSScriptAnalyzer | [releases](https://github.com/PowerShell/PSScriptAnalyzer/releases) (1.25.0 observed Mar 2026; [PowerShell Gallery](https://www.powershellgallery.com/packages/PSScriptAnalyzer)) | Exact upstream module package imported by explicit path over a portable `pwsh` runtime; no consumer runs `Install-Module`. Console-parse versus library-API binding stays open under issue #420 (per the adapter-input notes). Maintainer acquisition must establish and record byte identity and recheck latest stable when adding the adapter. |
| pwsh | [releases](https://github.com/PowerShell/PowerShell/releases) (7.6.5 LTS observed Aug 2026; 7.5.10 stable) | Portable runtime for the PSScriptAnalyzer module (per-platform binary archives, execution-platform lazy); exact runtime identity and platform artifact qualification remain pending under issue #420. Maintainer acquisition must establish and record byte identity. |
| cue | [releases](https://github.com/cue-lang/cue/releases) (v0.17.1 observed Jul 2026) | Checksummed standalone release artifact; `cue fmt` whole-file rewrite with check/diff mode. Maintainer acquisition must establish and record byte identity and recheck latest stable when adding the adapter. |
| jsonnetfmt | [go-jsonnet releases](https://github.com/google/go-jsonnet/releases) (v0.22.0 observed Mar 2026; C++ [jsonnet v0.21.0](https://github.com/google/jsonnet/releases)) | Checksummed standalone release artifact; go-jsonnet versus C++ implementation choice stays open under issue #420. Whole-file rewrite with check/diff mode. Maintainer acquisition must establish and record byte identity and recheck latest stable when adding the adapter. |
| pkl | [releases](https://github.com/apple/pkl/releases) (0.32.1 observed Jul 2026) | Checksummed standalone release artifact (per-platform binaries with published checksums). Maintainer acquisition must establish and record byte identity and recheck latest stable when adding the adapter. |
| djlint | [releases](https://github.com/djlint/djLint/releases) (v1.45.0 observed Sep 2026; [PyPI](https://pypi.org/project/djlint)) | Private wheel-only Python graph member over the shared managed Python runtime (no sdist fallback, no pip subprocess on the consumer path). `--lint` versus `--reformat` wiring stays open under issue #420. Maintainer acquisition must establish and record byte identity and recheck latest stable when adding the adapter. |
| Stylelint | [releases](https://github.com/stylelint/stylelint/releases) (17.14.1 observed Jul 2026; [npm](https://www.npmjs.com/package/stylelint)) | Private pure-JavaScript graph member over the shared managed Node runtime (standalone-artifact alternative stays open under issue #420). `--formatter json` shape stays an unproven mapping owned by issue #420. Maintainer acquisition must establish and record byte identity and recheck latest stable when adding the adapter. |
| prettier-plugin-gherkin | [npm](https://www.npmjs.com/package/prettier-plugin-gherkin) (observed; recheck latest stable when adding the adapter) | Private pure-JavaScript graph member (named plugin closure with Prettier over the shared managed Node runtime). Maintainer acquisition must establish and record byte identity. |
| prettier-plugin-sql | [npm](https://www.npmjs.com/package/prettier-plugin-sql) (0.15.1 observed) | Private pure-JavaScript graph member (named plugin closure with Prettier over the shared managed Node runtime). Maintainer acquisition must establish and record byte identity and recheck latest stable when adding the adapter. |
| prettier-plugin-xml | [npm](https://www.npmjs.com/package/prettier-plugin-xml) (observed; recheck latest stable when adding the adapter) | Private pure-JavaScript graph member (named plugin closure with Prettier over the shared managed Node runtime). Maintainer acquisition must establish and record byte identity. |
| modfmt | Upstream identity pending (recheck at implementation under issue #420) | Checksummed standalone release-artifact candidate for `go.mod` formatting; whole-file rewrite with check/diff mode. Maintainer acquisition must establish the exact upstream identity plus byte identity before adapter qualification. |
| terraform | [releases](https://github.com/hashicorp/terraform/releases) (v1.16.1 observed Sep 2026) | Checksummed standalone release artifact; `terraform fmt` whole-file rewrite with check/diff mode (fmt ships with the CLI). BUSL-1.1 license review stays pending under issue #420. Maintainer acquisition must establish and record byte identity and recheck latest stable when adding the adapter. |
| yamlfmt | [releases](https://github.com/google/yamlfmt/releases) (v0.21.0 observed Jan 2026) | Checksummed standalone release artifact (single binary, cosign-signed checksums from v0.14.0 on); `-lint` check mode. Maintainer acquisition must establish and record byte identity and recheck latest stable when adding the adapter. |
| yamllint | [releases](https://github.com/adrienverge/yamllint/releases) (1.38.0 observed Mar 2026) | Private wheel-only Python graph member over the shared managed Python runtime (no sdist fallback, no pip subprocess on the consumer path). Maintainer acquisition must establish and record byte identity and recheck latest stable when adding the adapter. |
| keep-sorted | [releases](https://github.com/google/keep-sorted/releases) (v0.10.0 observed Aug 2026) | Checksummed standalone release artifact; check-only with the provisional sandbox-apply-and-diff fix flow. Maintainer acquisition must establish and record byte identity and recheck latest stable when adding the adapter. |

Record compressed artifact identity separately from extracted executable identity. A versioned
release URL does not guarantee immutable bytes; checked-in digests must reject changed content.
If an upstream asset cannot satisfy a required platform, the existing reviewed release-CI source-build
route is the alternative, not consumer compilation or ambient libraries. Exact assets, checksums,
archive members, ABI floors, licenses, metadata schema, and regeneration command are tracked in
open work under issues #416-#420.
Adapter-specific issues are tracked in [initial adapter qualification](../quality/tool-integrations.md#initial-adapter-qualification).

## Delivery Classes

The acquisition and packaging routes below do not authorize replacement language, toolchain,
package-management, or framework stacks. Approved language maintenance includes focused patches,
packaging, and necessary reproducible upstream source builds under
[Product Scope](../product/scope.md#language-integration-maintenance). Candidate-specific effort,
maintenance ownership, and qualification evidence still require review
(open work under issue #505) before implementation;
general route permission does not prove a candidate feasible or admit an unbounded fork.

### Authoritative Toolchain Components

A compiler- or SDK-coupled tool comes from the authoritative toolchain selected for the
analyzed target. Initial members are `tsc`, gofmt, rustfmt, and Clippy. Qt, C/C++, Scala,
Buf, and similar integrations use this class when fixtures show that an independent tool
version would change target semantics or lose required compiler context.

These tools are supplied as prebuilt artifacts and hermetically acquired by their language rules.
An approved reproducible upstream release-CI build may supply a missing platform artifact through
that same authoritative toolchain; consumer acquisition does not build it.
`rules_dx` does not republish a second copy or maintain a parallel version selection
system. The adapter obtains the executable and runtime closure from tested public
toolchain/providers and includes their identity in the action key.

### Standalone And Complete Upstream Artifacts

A semantically independent native, self-contained, or complete upstream application uses a
checksummed release artifact when every required platform is available. Republishing it
additionally requires redistribution approval; otherwise Bazel downloads the immutable
artifact directly from upstream. Examples include Ruff, Ty, Biome, Buildifier, ShellCheck,
complete JVM CLI distributions, and exact .NET tool packages combined with a managed
runtime.

One generated checked-in metadata file per tool/platform records the exact immutable
URL, digest, size, archive member, upstream version, execution platform, ABI floor,
runtime files, and licenses, with a regeneration command (see `quality/artifacts/update.py`;
remaining asset qualification in open work under issues #416-#420).
One implementation repository exists per tool/platform, and an exec-configured dependency
or private toolchain selects by Bazel execution platform. Registration must not fetch
every artifact.

`rules_dx` may mirror an artifact when upstream retention, availability, or one-origin
policy requires it. Mirroring is not mandatory merely to make the consumer interface
uniform. If upstream lacks a required platform, release CI may build and publish that
platform artifact from pinned source; consumers never perform the build.

### Ruleset-Owned Ecosystem Graphs

An interpreted application with a mature Bazel package model uses a private, fully locked
dependency graph owned by the `rules_dx` module. This is the preferred path for curated
Python and Node applications when proof fixtures pass.

For Python, `rules_dx` owns one private `pyproject.toml`/`uv.lock` graph and exposes stable
entry-point targets through authoritative Python rules. Managed defaults accept prebuilt
wheels only: no sdist fallback, wheel compilation, pip subprocess, or consumer-provided
plugin is allowed. Ruff and Ty remain standalone artifacts rather than binary-wheel
entry-point targets.

For Node, `rules_dx` owns two single-importer pnpm workspaces sharing one
root `.npmrc` (`hoist=false`) and one managed Node runtime. Each workspace
keeps explicit `packages: ["."]` and fail-closed `allowBuilds: {}`; both
`package.json` files pin the same `packageManager` (`pnpm@10.34.5`, resolved
by the `pnpm` extension in `MODULE.bazel`); builds never invoke pnpm and
`package.json` files carry no scripts because Bazel owns execution.

- Authoritative application/test graph: root
  `package.json`/`pnpm-lock.yaml`/`pnpm-workspace.yaml` in hub `npm`
  (`//:node_modules`), owning Jest plus the framework runtimes.
- Private pure-JavaScript tool graph:
  `quality/tools/javascript/package.json`/`pnpm-lock.yaml`/`pnpm-workspace.yaml`
  in hub `npm_tools` (`//quality/tools/javascript:node_modules`), owning
  ESLint plus Prettier with stable `js_binary` entry points under
  `//quality/tools/javascript/bin`.

The split keeps the tool closure private: consumers never touch the tool
lock or hub labels. Separate `node_modules` facades preserve per-importer
layout instead of flattening the two graphs. Authoritative JavaScript rules
fetch exact package archives through Bazel and expose stable `js_binary`
targets. Each lock must require no lifecycle/install scripts or native addon
build. Shipped configuration imports the curated ESLint, Prettier, Stylelint,
and named plugin closure explicitly from that private graph.

The ruleset's own `MODULE.bazel` instantiates and imports both hubs
(authoritative `npm` plus private `npm_tools`). Consumers do
not call `use_extension`, `use_repo`, `npm_link_all_packages`, or console-script helpers,
and do not maintain a tool lockfile. Public `rules_dx` targets hide hub names and package
layout.

No arbitrary project plugin loading or custom executable replacement is promised in v1.
Native configuration files are supported only within the curated package/plugin closure;
they and every transitive include/import are explicit Bazel labels. An import of an
unshipped package fails clearly instead of triggering resolution or an ambient fallback.

### Assembled Release Bundles

When no mature no-install Bazel package path or complete upstream distribution satisfies
the contract, `rules_dx` release CI assembles a ready-to-run archive. This is the exception
for ecosystems such as Ruby and may apply to PowerShell, missing platform artifacts, native
package closures, or tools whose upstream installer is not a hermetic execution model.

Release CI may run an ecosystem resolver against a frozen lock, build missing artifacts,
and combine a runtime with installed packages. The published archive contains the complete
runtime/application closure, launch metadata, manifest, SBOM, licenses, and constituent provenance.
Final-archive attestations are published alongside it under
[Artifact Identity And Metadata](#artifact-identity-and-metadata).
Consumer builds only download, verify, extract, and execute it.

## Shared Runtimes

Managed interpreted tools in the same compatible version cohort share one exact hermetic
runtime rather than embedding it in every tool. The runtime is provisioned by an
authoritative Bazel runtime rule or, for an assembled bundle, by generated `rules_dx`
metadata. It is selected for the execution platform and fetched only when an applicable
action needs it.

Initial intended composition is:

- One managed Python runtime for djlint, flake8, pydoclint, pylint, and yamllint.
- One managed Node runtime for ESLint, Prettier, Stylelint, and curated plugins.
- One managed JDK for google-java-format, Checkstyle, PMD, SpotBugs, ktfmt, ktlint, and
  JVM-based Scalafmt where used.
- One managed .NET runtime for CSharpier and Fantomas.
- One assembled Ruby runtime/closure for RuboCop and StandardRB if the Ruby proof passes.
- One portable PowerShell runtime for PSScriptAnalyzer if direct module composition is
  selected by its proof.

The managed runtime may match an application runtime already present in the repository and
reuse downloader/cache bytes, but correctness never depends on an ambient installation.
Tools may use separate runtime versions when compatibility requires it; sharing is an
optimization constrained by tested compatibility, not a promise of one global runtime.
Parity requires representative proof that compatible tools really share bytes and runtime
identity; it does not force incompatible tools into one cohort.

## Complete Upstream Distributions

Prefer an upstream-tested complete CLI distribution over reconstructing an application
from package-manager modules. Initial JVM candidates include the google-java-format and
Checkstyle all-dependencies JARs, the PMD and SpotBugs binary distributions, the ktfmt
with-dependencies JAR, and the ktlint executable JAR. These share a managed JDK but remain
independently pinned artifacts.

CSharpier and Fantomas may use their exact official tool packages with a managed .NET
runtime when tests prove direct declared-DLL execution. PSScriptAnalyzer may use its exact
module package with portable `pwsh` and an explicit-path import. Neither route may run the
ecosystem's install command in a consumer build.

## First-Release Tool Routing

The planned first-release routing below is an implementation hypothesis governed by the
delivery-class proofs. It is broader than the initial dogfood routing and does not describe
current support. Accepted toolchain members are fixed unless superseded; provisional entries
do not count toward first-release parity until their focused proof selects and validates a
route.

| Delivery path | Initial tools |
| --- | --- |
| Authoritative selected toolchain | `tsc`, gofmt, rustfmt, Clippy, clang-format, clang-tidy |
| Checksummed native/self-contained artifact | Biome, `buf`, Buildifier, cppcheck, CUE, gofumpt, jsonnetfmt, keep-sorted, modfmt, Pkl, Ruff, shfmt, ShellCheck, Taplo, Terraform, Ty, Vale, yamlfmt |
| Complete upstream artifact plus shared JDK | google-java-format, Checkstyle, PMD, SpotBugs, ktfmt, ktlint |
| Exact upstream package plus shared .NET runtime | CSharpier, Fantomas |
| Exact upstream module plus portable PowerShell runtime | PSScriptAnalyzer |
| Private wheel-only Python graph plus shared managed Python | djlint, flake8, pydoclint, pylint, yamllint |
| Private pure-JavaScript graph plus shared managed Node | ESLint, Prettier, Stylelint, prettier-plugin-gherkin, prettier-plugin-sql, prettier-plugin-xml |
| Release-assembled Ruby closure | RuboCop, StandardRB |
| Focused proof selects authoritative toolchain or standalone artifact | qmlformat, qmllint |
| Focused proof selects compatible native/JVM artifact or target-coupled route | Scalafmt, Scalafix |

Ruff supplies both Python formatting and linting; Buildifier and Taplo each supply formatting
and linting; `buf` supplies format and lint; Prettier supplies its built-in file classes plus
the three named plugin integrations. A tool appearing once in this table is not downloaded
again for each capability or semantic file class. Acquisition deduplicates by tool/runtime
identity even when adapter metadata makes the tool applicable to many classes.

Decided route: `buf` takes the checksummed
native/self-contained artifact route. `buf` ships self-contained per-platform
release binaries with published checksums, needs no target compiler context
(unlike clang-tidy, which needs compile commands), and stays
execution-platform lazy; exact assets, digests, and adapter qualification
remain pending under issue #419 (live successor to closed #307 for the
`protobuf` class) and no adapter claims `protobuf` yet
(open under issue #419).

Decided route (Qt last): clang-format and
clang-tidy take the authoritative-toolchain route from the qualified
hermetic-llvm LLVM distribution's tool targets (no separate acquisition);
qmlformat and qmllint take the authoritative-toolchain route from the Qt
distribution, with exact Qt distribution identity, licensing, and platform
artifact qualification remaining pending under issue #419 (live successor
to closed #307 for the `qml` class) and no adapter claiming `qml` yet
(open under issue #419).
Qt closed that order (clang-format/clang-tidy, Buf, Scalafix routed to
the managed-JVM route, Qt last).

Decided route: google-java-format, Checkstyle,
PMD, SpotBugs, ktfmt, and ktlint take the complete-upstream-artifact plus
shared-JDK route. Each tool resolves to its upstream-tested complete CLI
distribution (google-java-format and Checkstyle all-dependencies JARs, the
PMD and SpotBugs binary distributions, the ktfmt with-dependencies JAR, the
ktlint executable JAR) sharing the one managed JDK cohort runtime; no tool
is reconstructed from Maven modules and no consumer runs an installer,
solver, or compiler. Exact artifact versions, digests, and adapter
qualification remain pending under issue #416 (live successor to closed #307
for this cohort) and no adapter claims `java` or `kotlin` yet
(open under issue #416).

Decided route: Scalafmt and Scalafix take the
managed JVM route. Scalafmt resolves to a
compatible JVM artifact and Scalafix to its semantic-rule artifacts over the
same shared managed JDK and Maven-lock story as the Scala foundation
(`maven_install.json` plus `fail_if_repin_required`); Scalafix semantic
rules additionally need semanticdb plus classpath wiring per the
adapter-input notes. Exact artifacts, rule-set/config qualification
(native-configuration review of the provisional Scalafix preset stays
required under issue #417), and adapter qualification remain pending
under issue #417 (live successor to closed #307 for this cohort) and no
adapter claims `scala` yet (open under issue #417).

Decided route: CSharpier and Fantomas take the
exact-upstream-package plus shared-.NET-runtime route. Each tool resolves to
its exact official tool package executed as declared DLLs over the one
managed .NET runtime cohort; no consumer runs `dotnet tool install` or any
equivalent installer. Exact package versions, runtime compatibility bounds
(Roslyn SDK analyzers stay SDK-default mode with StyleCop opt-in; FSharpLint
default ruleset with formatting rules off stays provisional), and adapter
qualification remain pending under issue #417 (live successor to closed #307
for this cohort) and no adapter claims `csharp` or
`fsharp` yet (open under issue #417).

Decided route: PSScriptAnalyzer takes the
exact-module plus portable-PowerShell-runtime route. The analyzer resolves
to its exact upstream module package imported by explicit path over a
portable `pwsh` runtime; the PowerShell application foundation stays
deferred beyond v1 (only this tool cohort is in scope). Exact module
version, runtime identity, console-parse versus library-API binding choice
(per the adapter-input notes), and adapter qualification remain pending
under issue #420 (live successor to closed #307 for the `powershell`
class) and no adapter claims `powershell` yet
(open under issue #420).

Decided route: RuboCop and StandardRB take the
release-assembled Ruby closure route (the exceptional bundle within the
approved packaging-effort boundary). Release CI assembles the complete
ready-to-run Ruby runtime/package closure with manifest, SBOM, licenses,
and constituent provenance; consumer builds only download, verify, extract,
and execute it. The Ruby application foundation stays deferred beyond v1
(only this tool cohort is in scope). Bundle contents, lock inputs, and
adapter qualification remain pending under issue #420 (live successor to
closed #307 for the `ruby` class) and no adapter claims `ruby` yet
(open under issue #420). This
closes that order (Python, Node, JVM including Scala/Scalafix managed
route, .NET, PowerShell, Ruby).

Decided route: clang-format, clang-tidy, and cppcheck take the
split native route. clang-format and clang-tidy resolve from the
qualified hermetic-llvm LLVM distribution's tool targets
(authoritative-toolchain class, no separate acquisition) per the Qt
record; clang-tidy needs compile-commands context — target-coupled
wiring versus check-only stays open under issue #418 and is recorded
explicitly here, never silent. cppcheck stays a standalone
checksummed-artifact candidate (`--xml --xml-version=2` on stderr).
Exact artifact version, digest, default-check/enablement qualification
(native-configuration review of the provisional clang-tidy/cppcheck
inputs stays required under issue #418), and adapter qualification
remain pending under issue #418 (live successor to closed #307 for the
`c`/`cpp` classes) and no adapter claims `c` or `cpp` yet
(open under issue #418). C/C++ MSVC-interop and SDK licensing stay with
the Windows platform issue — this route covers adapter behavior given a
qualified toolchain.

Decided route: gofumpt, staticcheck, govet, and errcheck take the
split Go route. gofumpt (strict superset of gofmt) stays a standalone
checksummed-artifact candidate with whole-file rewrite plus check/diff
mode; `govet` ships with the authoritative Go toolchain (no separate
acquisition); staticcheck and errcheck stay standalone
checksummed-artifact candidates. staticcheck default checks versus the
`SA`-only suggestion stay an unresolved conflict — neither is selected
here; qualify before freezing. `govet` and errcheck stay complementary
for unhandled errors (text-parse `file:line[:col]: message`), check-only
with the provisional sandbox-apply-and-diff fix flow. Exact artifact
versions, digests, rule-set qualification (native-configuration review
of the provisional staticcheck inputs stays required under issue #418),
and adapter qualification remain pending under issue #418 (live
successor to closed #307 for the `go` class) and no adapter claims `go`
yet (open under issue #418).

No listed private graph accepts arbitrary consumer packages or plugins in v1. The curated
lock and configuration are part of the `rules_dx` release. A tool whose required standard
behavior cannot fit that fixed closure must move to a proven complete artifact/assembled
route or be explicitly delayed.

## Artifact Identity And Metadata

Every fetched artifact has an immutable reference and cryptographic digest. Generated
metadata records tool and adapter identifiers, upstream version, delivery class, source
reference, execution-platform constraints, runtime compatibility, primary executable,
runtime files, licenses, and `rules_dx` release/schema versions.

An assembled archive additionally contains a versioned binary Protobuf manifest with a
complete relative file list, modes, sizes, digests, environment entries, launch metadata,
SBOM, and constituent provenance. Its identity is the digest of its exact published archive bytes.

The approved packaging boundary keeps constituent provenance inside the bundle and final-archive
attestations detached alongside it. Embedded evidence identifies constituent inputs or payloads,
not the containing archive's final digest. Finish the archive before producing its detached
attestations; each final-archive attestation binds to the digest of those exact published bytes.
Adding or changing embedded metadata changes the artifact identity and requires new final-archive
attestations. Detached evidence does not replace the required embedded inventory and provenance.

This separation avoids digest self-reference and is settled policy, not proof of a working signing
pipeline. Single correct path: payload-only embedded manifest plus detached final-archive attestation over exact
published bytes, with a completeness check proving no payload file is silently omitted.
Manifest self-entry treatment must still freeze without self-referential digest/size
requirements or silently omitting payload files. Exact profiles, trust identities, verification
inputs, and publication mechanics are tracked in
open work under issue #459.

Private ecosystem locks are repository inputs and part of the tool identity. Their exact
wheel/package archives and runtime identity participate in Bazel action keys. A ruleset
upgrade, lock change, runtime change, or platform selection therefore invalidates affected
actions without mutable CLI state.

## Provenance Profile Research

Read-only research recommends qualifying SPDX 2.3 JSON and SLSA Build
Provenance v1, each in an in-toto Statement v1, signed with the accepted Cosign keyless route
(see open work under issue #459).
This is a provisional wire-profile recommendation, not a dependency pin, assurance-level claim,
trusted-builder selection, or installer API. SPDX 3.0.1 is published; the 2.3 candidate aligns with
the existing license report and avoids an unneeded JSON-LD profile change, subject to qualification.

| Layer | Candidate identity and evidence |
| --- | --- |
| SPDX predicate | [`https://spdx.dev/Document/v2.3`](https://github.com/in-toto/attestation/blob/main/spec/predicates/spdx2.md), object-valued SPDX 2.3 JSON; distinguish SPDX element IDs (`SPDXRef-*`) from package URLs in external references. Cosign `spdx` preserves raw file bytes while `spdxjson` parses to a `structpb.Struct`; qualify both against the [generator](https://github.com/sigstore/cosign/blob/main/pkg/cosign/attestation/attestation.go). |
| SLSA predicate | [`https://slsa.dev/provenance/v1`](https://slsa.dev/spec/v1.2/build-provenance); specification v1.2 does not change the predicate to `/v1.2` or establish a Build assurance level. Required L1 fields are `buildDefinition{buildType, externalParameters}` plus `runDetails{builder{id}}`; `builder.id` plus the trusted signer determines the level. |
| Statement | [`https://in-toto.io/Statement/v1`](https://github.com/in-toto/attestation/blob/main/spec/v1/statement.md), with `_type https://in-toto.io/Statement/v1`, required `subject[].digest`, and the exact published artifact digest as subject. Subjects match purely by digest. |
| Signature evidence | Detached [Sigstore bundle](https://github.com/sigstore/protobuf-specs/blob/main/protos/sigstore_bundle.proto) carrying the signed statement and verification evidence; current media type `application/vnd.dev.sigstore.bundle.v0.3+json` while accepting `0.1`/`0.2` only if declared. Qualify single-leaf versus chain certificates, DSSE single-signature enforcement, expected `payloadType`, cert-validity timing, and verifier interoperability. Cosign aliases such as `slsaprovenance1` do not prove bundle-profile compatibility. |

The packaging boundary is settled in [Artifact Identity And Metadata](#artifact-identity-and-metadata):
embedded constituent evidence and detached final-archive attestations. The candidate wire profiles
above still need qualification; selecting the boundary does not freeze them. Mirroring evidence
must describe mirroring, not claim an upstream build
performed by rules_dx; assembly evidence must identify its component inputs without inventing their
build provenance.

Qualify actual producer output: [Cosign predicate generation](https://github.com/sigstore/cosign/blob/main/pkg/cosign/attestation/attestation.go)
has aliases and envelope versions that need not match these candidates. A CLI alias is not proof
of Statement/predicate compatibility. Verification must bind artifact bytes, predicate type, signer,
issuer, source, and builder policy to independently trusted expectations, not values trusted merely
because they are in the download. Keyless verification also requires authenticated log/time evidence;
[Rekor v2 uses a separate timestamp authority](https://docs.sigstore.dev/cosign/verifying/timestamps/).
Rekor v1 `integratedTime` is not externally verifiable timestamp evidence; qualify Rekor v1 versus
RFC3161 TSA versus Rekor v2 paths, offline verification with pinned trust roots, rotation with old-root
retention, and negative cases for digest mismatch, wrong predicate type, unknown `externalParameters`,
signer/`builder.id` mismatch, multi-signature DSSE, expired certificates without valid timestamps,
tampered time claims, and roots bundled where they must not appear. Self-attested provenance is L0/L1
at best and must not self-assert L2/L3 `builder.id`.

First-install verifier/trust bootstrap and exact verification inputs are tracked in
open work under issue #459, as are
required assurance level, trusted builders, complete inventory rules, reproducibility thresholds,
upstream-evidence exceptions, and manifest self-entry rules. Signing JSON alone proves neither
complete dependencies, hermeticity, reproducibility, redistribution permission, nor SLSA Build L2/L3.

## Platform And Laziness

Selection follows the Bazel execution platform, never repository-rule host probing. Host,
target, and execution platforms may differ. Tools used as actions are configured for
execution, and every required OS/CPU combination must have a compatible runtime and
artifact closure.

Adding `rules_dx` makes every supported foundation available but must not eagerly activate its
operational payload. Empty-cache and analysis fixtures prove each unused foundation creates no
configured target, action, module-extension package resolution, environment projection, usable
toolchain payload, compiler/runtime/package download, or application dependency fetch. An
applicable action fetches only its selected execution-platform runtime/application closure.
Module-resolution, lockfile, ruleset-source, and warm-start overhead from broad availability is
reasoned about for every added foundation; no timing is recorded per
[ADR 0022](../decisions/0022-no-benchmarking.md).

## Release And Update Policy

Each adapter initially selects the latest stable upstream release available when added.
Maintainer automation updates exact standalone metadata and private locks, verifies
licenses, and opens reviewable pull requests. Ruleset-owned package graphs are resolved
only by maintainer workflows; published module sources contain complete current locks.

Unchanged content-addressed artifacts may be reused across `rules_dx` releases. An update
must run the affected capability suite on every required platform and may not silently
change delivery class, runtime compatibility, or curated plugin membership.

Redistribution review applies only when `rules_dx` republishes bytes, but license and notice
metadata are still recorded for directly fetched third-party artifacts. Questionable
binary redistribution uses direct immutable upstream download, compliant source-built
publication, or explicit exclusion; building from source does not remove license duties.

## Verification

No delivery path becomes a default from documentation research alone. The authoritative
acquisition, toolchain, platform, and laziness evidence is defined by the
[Tool And Platform Test Matrix](../testing/tools.md). Capability-state definitions,
support promotion, and completion-report evidence are defined by the
[Testing Strategy](../testing/README.md#acceptance-evidence). Missing evidence delays the
affected route or integration; it never weakens the consumer contract.
