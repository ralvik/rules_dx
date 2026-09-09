# Tool And Platform Test Matrix

This matrix covers managed acquisition, authoritative toolchains, release pins,
platform execution, performance, and laziness. Quality adapter behavior remains in
[Quality Workflow Testing](../quality/quality-testing.md).

## Managed Tool Acquisition

Acquisition tests use external Bzlmod consumer workspaces rather than targets only inside
the `rules_dx` repository. A minimal consumer declares only `rules_dx`, with no module or
workspace language list and no quality-tool
runtime, lockfile, package graph, executable label, URL, checksum, or installer command.
It must run every default-selected tool through the public workflow.

Instrument repository setup and actions to distinguish Bazel downloads from prohibited
quality-tool package-manager behavior. Dedicated acquisition fixtures contain no unrelated
application dependency installation; process attribution and execution logs prove that the
private tool graph does not invoke pip, uv, npm, pnpm, Cargo, Maven, NuGet, Bundler,
PowerShell Gallery installation, or another ecosystem solver/installer. Maintainer-only lock
generation is tested separately and never runs when the external consumer evaluates the
published module. Normal application dependency behavior is governed by its language
foundation and does not count as quality-tool installation.

Authoritative-toolchain tests change default and per-target language versions and verify
that `tsc`, gofmt, rustfmt, and Clippy follow the selected target graph without an
independent quality-tool copy. `aquery`, providers, and execution logs prove matching
semantic identity, complete declared inputs, action-key changes, and correct host/target/
execution-platform behavior.

The Python proof uses one private `rules_dx` lock to export pydoclint and another eligible
[baseline Python tool](../tools/tool-baseline.md#curated-differences) in a compatible
[managed runtime cohort](../tools/tool-acquisition.md#shared-runtimes), qualified under O26.
Fixture membership selects no new tool or product default and does not depend on O11's M26
audit qualification. The [private-graph requirements](../tools/tool-acquisition.md#ruleset-owned-ecosystem-graphs)
require wheel-only selection, no sdist action, no wheel compilation, no ambient Python,
one managed runtime shared by both tools, and isolation between tool packages and analyzed
target dependencies. Tests cover every required execution platform and a non-Linux host
selecting a Linux remote execution platform where remote support is claimed.
Every other Python lock member adds its own external-consumer case for wheel availability,
entry point, configuration, runtime range, dependency isolation, and no-install behavior.

The Node proof uses one namespaced private `rules_dx` lock to export ESLint and Prettier
with shipped configuration and explicit plugin imports. The external consumer has no tool
`package.json`, pnpm lock, local `node_modules`, lifecycle script, native addon build, or
ambient Node dependency. Tests verify module/config resolution in local sandboxes and
claimed remote execution, and prove one managed runtime serves both tools.
Stylelint and every named Prettier plugin add external-consumer cases for package
availability, explicit plugin import, shipped configuration, no lifecycle script, and the
same runtime constraints.

The JVM proof first composes google-java-format and Checkstyle complete upstream artifacts
with one managed JDK. Equivalent tests are required before PMD, SpotBugs, ktfmt, ktlint, or
JVM Scalafmt adopts the path. .NET and PowerShell proofs execute exact package contents
through declared runtimes without install/restore commands. Qt, Clang, Buf, and Scalafix
fixtures decide whether their tools must follow the target-selected toolchain.

An exceptional-bundle proof assembles at least one frozen ecosystem closure, initially
Ruby, in release CI and consumes it from a clean external workspace without a compiler,
installer, or ambient runtime. It verifies relocation, archive and file integrity,
licenses, SBOM/provenance, minimum host ABI, and every claimed platform. An ecosystem that
cannot pass this proof is delayed explicitly rather than given a non-hermetic fallback.

## Research Qualification Fixtures

The [initial artifact candidates](../tools/tool-acquisition.md#initial-artifact-research) require
actual-byte checksum checks, archive member and executable-mode checks, changed-release-byte
rejection, empty-PATH execution, runtime/ABI inspection (especially Vale Linux), and each required
platform. Upstream build recipes alone do not prove the published artifact's properties.

The [provenance profile candidates](../tools/tool-acquisition.md#provenance-profile-research) require
producer/verifier round trips asserting actual schema identifiers, artifact-digest binding, and
negative cases for wrong signer/issuer/builder/source, substituted SBOMs, invalid log/time evidence,
and missing trust material. Cover digest mismatch, wrong predicate type (including `/v1.2` or
unversioned SPDX `Document` where versioned output is required), unknown `externalParameters`,
signer/`builder.id` mismatch, multi-signature DSSE, legacy bundle promises in a newer-only ecosystem,
expired certificates without valid timestamps, tampered time claims, bundled roots, and SPDX
file-bytes versus parsed-object handling. Prove first-install trust without executing unverified downloads, valid
offline verification with authenticated trust roots, rotation with old-root retention, and independent rebuild evidence.
Test the approved [packaging boundary](../tools/tool-acquisition.md#artifact-identity-and-metadata):
constituent provenance remains embedded, final-archive attestations remain detached and bind to the
published bytes, missing required evidence fails verification, and changing embedded metadata
invalidates old final-archive attestations. Freeze manifest self-entry rules and remaining O38/O39
trust/profile policy before asserting those additional outcomes.

## Laziness And Performance

Unused-foundation tests begin with an empty repository cache and observable downloader logs.
Adding `rules_dx` while leaving each supported foundation unused must create no configured
target, action, module-extension application dependency resolution, environment projection,
usable toolchain payload, compiler/runtime/package download, or application dependency fetch.
An applicable action may fetch only its selected execution-platform artifact, compatible
runtime, and declared application closure.

Performance fixtures compare standalone artifacts, private Python/Node graphs, complete
JVM distributions, and assembled bundles. Record repository-evaluation time, cold bytes
and file count, extraction, runfiles construction, Windows behavior, remote upload, first
action wall time, warm no-change behavior, and one-source invalidation. A consolidated
release archive replaces a private package graph only when measurements show a material
benefit and the full capability suite remains equivalent.

Configuration tests prove all supported foundations are automatically available from one
`rules_dx` dependency. Explicit generation creates relevant initial target declarations from
supported sources alone; manifests remain authoritative for project and dependency metadata.
Separate analysis of declared targets/providers activates only their relevant wrappers,
environment contributors, target-coupled adapters, and lazy toolchain payloads. Generation alone
activates none of them, and unused foundations satisfy the complete zero-operational-work contract
above. Target-coupled adapters still require authoritative provider classes and a nonempty
intersection with their capability-specific supported classes and workspace policy.

Standalone quality adapters are tested independently from application-foundation activation. A
custom semantic-class provider can activate selected formatting/linting policy without compiler
rules, package dependencies, or environment projections; target-coupled adapters remain absent
without their authoritative application providers.

Default-policy fixtures omit a quality-family list and prove every built-in family is lazily
available. Only semantic classes in analyzed targets activate default stages and tool fetches;
family configuration changes defaults but is not required for activation.

Root-module override fixtures prove a resolved graph differing from release defaults receives no
special warning or rejection. Genuine provider/API/toolchain incompatibility still fails at its
normal Bazel boundary.

## Pins And Platform Tests

The only supported Bazel version is the one committed in `.bazelversion`; CI and
compatibility claims test that exact version. A pin update is accepted only after
the repository test suite passes against the proposed stable release.

The normal Rust compiler/toolchain is also an exact latest-stable pin. The separately pinned
nightly documentation extractor follows the narrow
[rustdoc exception](../decisions/0008-dependency-currency.md#rustdoc-extraction-exception).
Toolchain updates must pass the repository suite on every required OS/CPU pair before adoption;
extractor updates additionally require that exception's compatibility and drift evidence.

Exercise every declared host/execution OS and CPU pair. Unsupported combinations
must fail during analysis or toolchain resolution with actionable diagnostics.
The authoritative required-host set, including the macOS x86_64 best-effort
status, is defined in
[ADR 0014](../decisions/0014-tested-platform-release-stack.md#required-platforms);
this matrix adds no separate host list.

Native-stack fixtures qualify both Linux glibc and static-musl profiles, mixed Rust/C/C++
dependencies, and the [hermetic Windows baseline](../decisions/0014-tested-platform-release-stack.md#decision).
Record compiler/SDK/STL/CRT acquisition identities. Run Windows fixtures without host-installed
Build Tools or SDKs and prove their declared closure supplies all required native inputs. Missing or
incompatible components must fail with actionable diagnostics, never trigger installation in an
action or a host-SDK/MinGW fallback. Prove input changes invalidate affected work and qualify cache
and remote behavior separately. An unused native foundation must not acquire native tool payloads
merely to add the module or use unrelated tools. Cross-build fixtures name each route
and distinguish artifact production from tests actually executed on the target platform.

Use the provisional [native qualification plan](../native-toolchains.md) to select candidate fixtures,
not to infer support. Cover default and opted-out Rust build-script execution separately from script
compilation, third-party shell-environment isolation, Windows ABI constraints and execroot-relative
include/library/response-file paths. Test independently built MSVC libraries, native proc-macro
dependencies, runtime DLL deployment, and Rust/C/C++ coverage including shared objects and missed
lines. A successful empty report or ignored collection failure cannot satisfy coverage.

Acquisition fixtures compare clean re-resolution against frozen compiler/SDK/runtime package inputs,
not only a ruleset commit. Missing EULA acknowledgement must fail before restricted acquisition;
unrelated workflows must neither require acknowledgement nor acquire restricted payloads. Record
license-approved cache/mirror/remote-worker boundaries separately from technical download success.

Release tests execute each prebuilt `dx` binary on its matching host, verify its
published checksum, and prove standalone installation does not require Bazel or
Rust. Exercise the documented installation workflow on every advertised host against
the [mandatory authenticity-verification policy](../environments/environment.md#distribution).
Accept a valid artifact from the approved publisher and reject tampered bytes, a replaced
binary/checksum pair without valid authenticity evidence, an unapproved signer, and
missing, invalid, or unavailable verification inputs. Verify failure occurs before
installing or executing the downloaded binary, with no checksum-only fallback.
Include verifier bootstrap and trust-root provenance in qualification evidence rather
than assuming a verifier acquired alongside the binary is trustworthy.
Environment platform requirements are maintained in
[Developer Environments](../environments/environment.md).
