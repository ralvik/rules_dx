# Tool Acquisition

`rules_dx` supplies every managed quality tool as a pinned, hermetic,
ready-to-run Bazel dependency. Open work is tracked in GitHub issues.

## Consumer Contract

A consumer adds one `rules_dx` module dependency and uses the automatically
available foundations and curated tools. It does not select quality-tool
acquisition strategies, versions, runtimes, package locks, plugin sets, URLs,
checksums, or executable labels. A repository may still select its application
compiler/runtime versions; coupled quality tools follow that selection.

Consumer workflows never search ambient PATH, invoke an ecosystem installer,
solve a tool dependency graph, compile a managed tool, or download from an
action. Repository setup may use Bazel's downloader and established
language-rule repository logic to fetch exact artifacts from a `rules_dx`-owned
lock. It must not invoke `pip install`, `npm install`, `cargo install`,
`dotnet tool install`, Bundler, or an equivalent installer.

An adapter always receives a declared executable, its complete runtime inputs,
deterministic environment, and version identity. Acquisition details remain
private implementation details. Adapters needing Windows compiler or SDK
context obtain it through the selected upstream toolchain, not an installed
host SDK.

This contract does not state that any integration is currently implemented,
dogfooded, adapter-tested, or supported. A delivery class is accepted only
after its required proof fixtures pass.

## Bootstrap Maintenance And Promotion

There are no bootstrap-maintained seed tools and no seed-to-adapter parity
gate. Quality, audit, CI, and other integrations land directly through their
product adapters when the owning issue implements them; gaps are accepted until
then.

Product adapters must independently satisfy the managed acquisition matrix in
[Tool And Platform Test Matrix](../testing/tools.md) and the consumer contract
above. A product adapter becomes dogfooded only when this repository and CI use
the intended product path.

### Initial Dogfood Routing

These routes define dogfood ownership; they do not claim current availability
or support. Each adapter lands directly.

| Repository check | Product route | Disposition |
| --- | --- | --- |
| rustfmt, Clippy, and rustc typecheck | Authoritative selected Rust toolchain | Land directly through the selected Rust toolchain identity. |
| Buildifier, Taplo, and Vale | Checksummed standalone upstream artifacts | Land directly through each exact platform artifact and digest. |
| Markdown link and structure validation | None; repository maintenance only | Retain as a narrow repository check, separate from Vale. |

Prettier and the private Node tool graph are not bootstrap dependencies. They
remain in the first-release baseline but are deferred to JavaScript and
TypeScript quality.

### Initial Artifact Research

Read-only upstream research supports the candidate tools. Versions observed
during research are observations, not pins. Recheck latest stable and verify
actual bytes when adding each adapter. Record compressed artifact identity
separately from extracted executable identity: a versioned release URL does not
guarantee immutable bytes, so checked-in digests must reject changed content.
If an upstream asset cannot satisfy a required platform, the reviewed
release-CI source-build route is the alternative, not consumer compilation.
Exact assets, checksums, archive members, ABI floors, licenses, metadata
schema, and regeneration commands live with the adapter work.

## Delivery Classes

The routes below do not authorize replacement language, toolchain,
package-management, or framework stacks. General route permission does not
prove a candidate feasible.

### Authoritative Toolchain Components

A compiler- or SDK-coupled tool comes from the authoritative toolchain
selected for the analyzed target. Initial members are `tsc`, gofmt, rustfmt,
and Clippy. The adapter obtains the executable and runtime closure from tested
public toolchain/providers and includes their identity in the action key.
`rules_dx` does not republish a second copy or maintain a parallel version
selection system.

### Standalone And Complete Upstream Artifacts

A semantically independent native, self-contained, or complete upstream
application uses a checksummed release artifact when every required platform
is available. Republishing additionally requires redistribution approval;
otherwise Bazel downloads the immutable artifact directly from upstream.

One generated checked-in metadata file per tool/platform records the exact
immutable URL, digest, size, archive member, upstream version, execution
platform, ABI floor, runtime files, and licenses, with a regeneration command.
One implementation repository exists per tool/platform, and an
exec-configured dependency or private toolchain selects by Bazel execution
platform. Registration must not fetch every artifact.

`rules_dx` may mirror an artifact when upstream retention, availability, or
one-origin policy requires it. If upstream lacks a required platform, release
CI may build and publish that platform artifact from pinned source; consumers
never perform the build.

### Ruleset-Owned Ecosystem Graphs

An interpreted application with a mature Bazel package model uses a private,
fully locked dependency graph owned by the `rules_dx` module. This is the
preferred path for curated Python and Node applications when proof fixtures
pass.

For Python, `rules_dx` owns one private `pyproject.toml`/`uv.lock` graph and
exposes stable entry-point targets through authoritative Python rules. Managed
defaults accept prebuilt wheels only: no sdist fallback, wheel compilation,
pip subprocess, or consumer-provided plugin is allowed.

For Node, `rules_dx` owns two single-importer pnpm workspaces sharing one
root `.npmrc` and one managed Node runtime: the authoritative
application/test graph and the private pure-JavaScript tool graph. Consumers
do not maintain a tool lockfile; public targets hide hub names and package
layout. Each lock must require no lifecycle/install scripts or native addon
build.

No arbitrary project plugin loading or custom executable replacement is
promised in v1. Native configuration files are supported only within the
curated package/plugin closure; every transitive include/import is an explicit
Bazel label.

### Assembled Release Bundles

When no mature no-install Bazel package path or complete upstream distribution
satisfies the contract, `rules_dx` release CI assembles a ready-to-run
archive. This is the exception for ecosystems such as Ruby and may apply to
PowerShell, missing platform artifacts, or tools whose upstream installer is
not a hermetic execution model. The published archive contains the complete
runtime/application closure, launch metadata, manifest, SBOM, licenses, and
constituent provenance. Consumer builds only download, verify, extract, and
execute it.

## Shared Runtimes

Managed interpreted tools in the same compatible version cohort share one
exact hermetic runtime rather than embedding it in every tool. The runtime is
selected for the execution platform and fetched only when an applicable action
needs it. Sharing is an optimization constrained by tested compatibility, not
a promise of one global runtime.

Initial intended composition is one managed Python runtime, one managed Node
runtime, one managed JDK, one managed .NET runtime, one assembled Ruby
runtime/closure, and one portable PowerShell runtime.

## Complete Upstream Distributions

Prefer an upstream-tested complete CLI distribution over reconstructing an
application from package-manager modules. JVM candidates use complete
all-deps/binary distributions over the one managed JDK cohort. CSharpier and
Fantomas use exact official tool packages with a managed .NET runtime when
tests prove direct declared-DLL execution. PSScriptAnalyzer uses its exact
module package with portable `pwsh` and an explicit-path import. No route runs
the ecosystem's install command in a consumer build.

## First-Release Tool Routing

The planned first-release routing below is an implementation hypothesis
governed by delivery-class proofs. Provisional entries do not count toward
parity until their focused proof selects a route.

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

A tool appearing once above is not downloaded again for each capability.
Acquisition deduplicates by tool/runtime identity even when adapter metadata
makes the tool applicable to many classes.

## Artifact Identity And Metadata

Every fetched artifact has an immutable reference and cryptographic digest.
Generated metadata records tool and adapter identifiers, upstream version,
delivery class, source reference, execution-platform constraints, runtime
compatibility, primary executable, runtime files, licenses, and release/schema
versions.

An assembled archive additionally contains a versioned binary Protobuf
manifest with a complete relative file list, modes, sizes, digests,
environment entries, launch metadata, SBOM, and constituent provenance. Its
identity is the digest of its exact published archive bytes. Finish the
archive before producing its detached attestations; each final-archive
attestation binds to the digest of those exact published bytes.

Private ecosystem locks are repository inputs and part of the tool identity.
A ruleset upgrade, lock change, runtime change, or platform selection
invalidates affected actions without mutable CLI state.

Maintainer fetches are https-only with retries plus backoff; the checked-in
digest is the trust anchor, never first-seen bytes. Consumer fetches verify
the outer asset digest plus Bazel retry and re-hash the extracted executable
against the recorded inner digest. One pinned upstream URL per tool/platform
is intentional: the digest is the trust anchor and the URL is availability
only.

## Provenance Profile Research

Read-only research recommends qualifying SPDX 2.3 JSON and SLSA Build
Provenance v1, each in an in-toto Statement v1, signed with the accepted
Cosign keyless route. This is a provisional wire-profile recommendation, not a
dependency pin or assurance-level claim. The packaging boundary above is
settled; the candidate wire profiles still need qualification.

## Platform And Laziness

Selection follows the Bazel execution platform, never repository-rule host
probing. Adding `rules_dx` makes every supported foundation available but must
not eagerly activate its operational payload. An applicable action fetches
only its selected execution-platform runtime/application closure.

## Release And Update Policy

Each adapter initially selects the latest stable upstream release available
when added. Maintainer automation updates exact standalone metadata and
private locks, verifies licenses, and opens reviewable pull requests.
Ruleset-owned package graphs are resolved only by maintainer workflows.

### Repinning

Every managed lock dialect repins through one resolver-owned command
(maintainer only; requires network). `bazel run //tools:repin-all` sequences
the whole table and stops at the first failure.

| Dialect | Manifest | Lock | Repin command |
| --- | --- | --- | --- |
| cargo | `rust/tests/fixtures/hello/Cargo.toml` | `Cargo.lock` plus `cargo-bazel-lock.json` | `CARGO_BAZEL_REPIN=1 bazel build //rust/tests/fixtures/hello:hello` |
| npm | `package.json` | `pnpm-lock.yaml` | `bazel run @pnpm//:pnpm -- update` |
| npm tools | `quality/tools/javascript/package.json` | `quality/tools/javascript/pnpm-lock.yaml` | `bazel run @pnpm//:pnpm -- --dir quality/tools/javascript install --lockfile-only` |
| maven | `third_party/jvm/pins.bzl` | `third_party/jvm/maven_install.json` | `REPIN=1 bazel run @maven//:pin` |
| nuget | `third_party/dotnet/paket.dependencies` | `third_party/dotnet/paket.lock` | `bazel run @rules_dotnet//tools/paket2bazel -- --dependencies-file $PWD/third_party/dotnet/paket.dependencies --output-folder $PWD/third_party/dotnet/deps` |
| go | `third_party/go/go.mod` | `third_party/go/go.sum` | Intentional no-op (pinned module lock tracks Gazelle; widen via `dx bump`) |
| uv | `python/tests/fixtures/hello/pyproject.toml` | `python/tests/fixtures/hello/uv.lock` | `uv lock` in `python/tests/fixtures/hello` |
| uv tools | `quality/tools/python/pyproject.toml` | `quality/tools/python/uv.lock` | `uv lock` in `quality/tools/python` |

An update must run the affected capability suite on every required platform
and may not silently change delivery class, runtime compatibility, or curated
plugin membership. Redistribution review applies only when `rules_dx`
republishes bytes, but license and notice metadata are still recorded for
directly fetched artifacts.

## Verification

No delivery path becomes a default from documentation research alone. The
authoritative acquisition, toolchain, platform, and laziness evidence is
defined by the [Tool And Platform Test Matrix](../testing/tools.md).
Capability-state definitions, support promotion, and completion-report
evidence are defined by the [Testing Strategy](../testing/README.md#acceptance-evidence).
Missing evidence delays the affected route or integration; it never weakens
the consumer contract.
