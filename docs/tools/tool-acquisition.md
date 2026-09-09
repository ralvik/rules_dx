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
and O46 in the [open-decision register](../open-decisions.md), on every
[required platform](../decisions/0014-tested-platform-release-stack.md#required-platforms).
Swift and SwiftFormat are excluded from v1 by
[ADR 0019](../decisions/0019-first-release-additional-foundations.md); see
[Swift Feasibility](tool-baseline.md#swift-feasibility).

This contract does not state that any integration is currently implemented, dogfooded,
adapter-tested, or supported. A delivery class is accepted only after its required proof
fixtures pass; a plausible design or an entry in the tool matrix is not implementation evidence.

## Bootstrap Maintenance And Promotion

There are no bootstrap-maintained seed tools and no seed-to-adapter
parity gate. Quality, audit, CI, and other integrations land directly through their
product adapters when the owning milestone implements them; gaps are accepted until
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
| rustfmt and Clippy | Authoritative selected Rust toolchain | Land directly through the selected Rust toolchain identity. |
| Buildifier, Taplo, and Vale | Checksummed standalone upstream artifacts | Land directly through each exact platform artifact and digest. |
| Markdown link and structure validation | None; repository maintenance only | Retain as a narrow repository check, separate from Vale and outside the product adapter baseline. |

Prettier and the private Node tool graph are not bootstrap or initial-dogfood dependencies.
They remain in the frozen first-release baseline but are deferred to
[JavaScript And TypeScript Quality](../milestones/M17-javascript-typescript-quality.md).

### Initial Artifact Research

Read-only upstream research supports the following O20 candidates, not qualified
pins or platform support. Recheck latest stable and verify actual bytes when adding each adapter.

| Tool | Upstream evidence | Candidate acquisition and remaining risk |
| --- | --- | --- |
| Buildifier | [v8.5.1 assets](https://github.com/bazel-contrib/buildtools/releases/tag/v8.5.1) and [release targets](https://github.com/bazel-contrib/buildtools/blob/v8.5.1/buildifier/BUILD.bazel) | Raw platform executable from `bazel-contrib/buildtools`; pure-Go build intent still requires artifact linkage, runtime, and platform evidence. |
| Taplo | [0.10.0 assets](https://github.com/tamasfe/taplo/releases/tag/0.10.0) and [release workflow](https://github.com/tamasfe/taplo/blob/0.10.0/.github/workflows/releases.yaml) | Gzip executable on Linux/macOS, ZIP on Windows. Bazel supports [gzip extraction](https://bazel.build/rules/lib/builtins/repository_ctx#download_and_extract); qualify member naming/mode with the seed Bazel. Reviewed release metadata supplied no asset digest/checksum file, so maintainer acquisition must establish and record byte identity. |
| Vale | [v3.20.0 assets](https://github.com/vale-cli/vale/releases/tag/v3.20.0) and [release configuration](https://github.com/vale-cli/vale/blob/v3.20.0/.goreleaser.yml) | Platform archive plus published checksums from `vale-cli/vale`. Linux CGO/GNU builds are not evidence of static-musl compatibility; inspect the actual runtime closure before admission. |

Record compressed artifact identity separately from extracted executable identity. A versioned
release URL does not guarantee immutable bytes; checked-in digests must reject changed content.
If an upstream asset cannot satisfy a required platform, the existing reviewed release-CI source-build
route is the alternative, not consumer compilation or ambient libraries. Exact assets, checksums,
archive members, ABI floors, licenses, metadata schema, and regeneration command remain under O20.
Adapter-specific issues are tracked in [initial adapter qualification](../quality/tool-integrations.md#initial-adapter-qualification).

## Delivery Classes

The acquisition and packaging routes below do not authorize replacement language, toolchain,
package-management, or framework stacks. Approved language maintenance includes focused patches,
packaging, and necessary reproducible upstream source builds under
[Product Scope](../product/scope.md#language-integration-maintenance). O46 still requires review of
candidate-specific effort, maintenance ownership, and qualification evidence before implementation;
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
runtime files, and licenses, with a regeneration command per [O20](../open-decisions.md).
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

For Node, `rules_dx` owns one namespaced package graph and pnpm lock. Authoritative
JavaScript rules fetch exact package archives through Bazel and expose stable `js_binary`
targets. The lock must require no lifecycle/install scripts or native addon build. Shipped
configuration imports the curated ESLint, Prettier, Stylelint, and named plugin closure
explicitly from that private graph.

The ruleset's own `MODULE.bazel` instantiates and imports these private hubs. Consumers do
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
| Authoritative selected toolchain | `tsc`, gofmt, rustfmt, Clippy |
| Checksummed native/self-contained artifact | Biome, Buildifier, cppcheck, CUE, gofumpt, jsonnetfmt, keep-sorted, modfmt, Pkl, Ruff, shfmt, ShellCheck, Taplo, Terraform, Ty, Vale, yamlfmt |
| Complete upstream artifact plus shared JDK | google-java-format, Checkstyle, PMD, SpotBugs, ktfmt, ktlint |
| Exact upstream package plus shared .NET runtime | CSharpier, Fantomas |
| Exact upstream module plus portable PowerShell runtime | PSScriptAnalyzer |
| Private wheel-only Python graph plus shared managed Python | djlint, flake8, pydoclint, pylint, yamllint |
| Private pure-JavaScript graph plus shared managed Node | ESLint, Prettier, Stylelint, prettier-plugin-gherkin, prettier-plugin-sql, prettier-plugin-xml |
| Release-assembled Ruby closure | RuboCop, StandardRB |
| Focused proof selects authoritative toolchain or standalone artifact | `buf`, clang-format, clang-tidy, qmlformat, qmllint |
| Focused proof selects compatible native/JVM artifact or target-coupled route | Scalafmt, Scalafix |

Ruff supplies both Python formatting and linting; Buildifier and Taplo each supply formatting
and linting; `buf` supplies format and lint; Prettier supplies its built-in file classes plus
the three named plugin integrations. A tool appearing once in this table is not downloaded
again for each capability or semantic file class. Acquisition deduplicates by tool/runtime
identity even when adapter metadata makes the tool applicable to many classes.

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
O39 must still freeze manifest self-entry treatment without self-referential digest/size
requirements or silently omitting payload files. Exact profiles, trust identities, verification
inputs, and publication mechanics remain under O38/O39.

Private ecosystem locks are repository inputs and part of the tool identity. Their exact
wheel/package archives and runtime identity participate in Bazel action keys. A ruleset
upgrade, lock change, runtime change, or platform selection therefore invalidates affected
actions without mutable CLI state.

## Provenance Profile Research

O39 read-only research recommends qualifying SPDX 2.3 JSON and SLSA Build
Provenance v1, each in an in-toto Statement v1, signed with the accepted Cosign keyless route.
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

O38 retains first-install verifier/trust bootstrap and exact verification inputs. O39 retains
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
benchmarked and reported for every added foundation.

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
acquisition, toolchain, platform, laziness, and performance evidence is defined by the
[Tool And Platform Test Matrix](../testing/tools.md). Capability-state definitions,
support promotion, and completion-report evidence are defined by the
[Testing Strategy](../testing/README.md#acceptance-evidence). Missing evidence delays the
affected route or integration; it never weakens the consumer contract.
