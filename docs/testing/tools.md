# Tool And Platform Test Matrix

This matrix covers managed acquisition, authoritative toolchains, release pins,
platform execution, and laziness. Quality adapter behavior remains in
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
[managed runtime cohort](../tools/tool-acquisition.md#shared-runtimes), qualified in
open work under #796-#800 (successors to closed #510).
Fixture membership selects no new tool or product default and does not depend on the audit
qualification open. The [private-graph requirements](../tools/tool-acquisition.md#ruleset-owned-ecosystem-graphs)
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
invalidates old final-archive attestations. Manifest self-entry rules are frozen under issue #307
(no self-referential digest/size via `packaging_uses_single_correct_path` plus manifest
completeness via `manifest_covers_payload` in `cli/qualification`); remaining trust/profile
policy (trusted builders, rotation, offline roots, rebuild thresholds) stays owned by the issue tracker
before asserting those additional outcomes.

## Laziness

Unused-foundation tests begin with an empty repository cache and observable downloader logs.
Adding `rules_dx` while leaving each supported foundation unused must create no configured
target, action, module-extension application dependency resolution, environment projection,
usable toolchain payload, compiler/runtime/package download, or application dependency fetch.
An applicable action may fetch only its selected execution-platform artifact, compatible
runtime, and declared application closure.

Laziness fixtures compare standalone artifacts, private Python/Node graphs, complete
JVM distributions, and assembled bundles by declared inputs and outputs: cold bytes
and file count, extraction, runfiles construction, Windows behavior, remote upload,
warm no-change behavior, and one-source invalidation by action graph, not wall time.
A consolidated release archive replaces a private package graph only when reasoning
shows a material benefit and the full capability suite remains equivalent, per
[ADR 0022](../decisions/0022-no-benchmarking.md).

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

## Shell And Host-Tool Contract

CI and test harness shell is bash-only (decided
under issue #299; Windows native execution via shell bash under issue
#414; no product behavior change). Every bash
`sh_binary`/`sh_test` carries
`target_compatible_with = ["@platforms//os:linux"]`, so non-Linux hosts
skip honestly instead of failing obscurely; POSIX `#!/bin/sh` fixtures
(`env/doctor.sh`, `env/tool.sh`, deploy fixtures) stay portable with no
constraint. macOS
best-effort hardening rides along with no Linux behavior change:
`realpath` probe (`realpath` → `readlink -f` → python3),
`sha256sum` → `shasum -a 256` fallback, portable `sed` tmpfile edits
(no `sed -i -e`), `cp -RPp` (no `cp -a`), and portable timing
(`$EPOCHREALTIME` → `date` fallback). Windows x86_64 MSVC-compatible is
qualified for native `dx`/CI execution under issue #414
(`windows-latest` runners with shell `bash`, per-host cache scope,
portable forms only); Windows shell stays bash-only with no `.ps1`/`.bat`
plus no `rules_powershell` per
[ADR 0014](../decisions/0014-tested-platform-release-stack.md#required-platforms);
product runtime is Rust and shell-free except
generated deploy launchers plus the managed doctor shim.
`//tools/ci:shell_contract` machine-checks this contract.

Bootstrap requires bash by design under issue #450 (`BASH_SOURCE`, `[[`,
arrays, `printf -v` plus the runfiles fallback never run under POSIX
`sh`): every `tools/sh/lib.sh` driver carries `#!/usr/bin/env bash` plus
`set -euo pipefail` plus one identical `tools/sh/bootstrap.sh` loader plus
`dx_bootstrap` lines with `data = ["//tools/sh:lib"]` (single-sourced
bootstrap with no per-file depth adjustment under issue #654). Floor is
bash 3.2+ with Linux execution; macOS/Windows run the same bash with no
behavior change. Intentional lib-free exceptions are POSIX `#!/bin/sh`
fixtures (no bootstrap, no constraint) plus the deploy hermetic
python-only runtime (bash + python3 + coreutils, no lib bootstrap per the
host-tool contract below) plus standalone renderers needing no
workspace/runfiles.

Platform policy is decided under issue #320 (no silent cfg hacks):
symlink copy fallback follows the portable route (`quality/adapter`
copies unreadable-link closures, proven by the fallback test plus the
non-unix no-symlink assertion); `/proc` scan plus clean-measure and
resolve-symlink fixtures run portably via OS-selected planters, while
unix-socket, POSIX-mode, permission-bit, and byte-path fixtures stay
unix-gated with fail-fast reasons; release archives stay hermetic
Python (`archiver.py` tarfile dereferences like `tar -h`, no host
`tar`, proven by `archive_verify` plus `//tools/ci:shell_contract`).
`//tools/ci:shell_contract` machine-checks the no-host-tar pin. There
are no standing benchmarks per
[ADR 0022](../decisions/0022-no-benchmarking.md).

CI shell dedup plus portable forms are delivered (closed issue #323):
shared `tools/sh/lib.sh`, shellcheck plus shfmt, portable realpath, hashing,
sed, cp, and timing with no per-file copies. `//tools/ci:shell_contract`
machine-checks this contract.

Shell-harness elimination stays wont-fix under issue #667 (CI/harness only,
no product behavior; affirms decided #299, not a reversal). Inventory at
decision time is about 176 `*.sh` (about 172 bash) with about 131 in
`tools/ci` (about 86 `*_qualification.sh`) plus shared `tools/sh/lib.sh`,
`snapshot.sh`, `guards.sh`, `bootstrap.sh`; four POSIX `#!/bin/sh` fixtures
plus the deploy hermetic Python runtime stay as-is. Wholesale Rust-ify would
rewrite static clean-tree guards as `rust_test` with no product gain while
re-opening #323/#450/#653/#654; POSIX-only still forks with fewer helpers;
per-OS shells double the harness. Incremental per-tool Rust-ify stays allowed
where a product CLI already owns the behavior, never a harness-wide
migration. `.shellcheckrc` stays `shell=bash`, the support-matrix plus
verification-matrix shell rows and the ci.yml prove `shell contract` step
stay as-is; `//tools/ci:shell_contract` owns this rule.

Guard maintenance owns shared helpers plus snapshot versus grep policy under
issue #450, extended with the table-driven guard rows under issue #653:
shared shell logic lives once in `tools/sh/lib.sh`
(`dx_expect_file`, `dx_expect_contains`, `dx_expect_absent`) plus
`tools/sh/guards.sh` (`dx_guard_*`/`dx_guards_*` table rows with
reason/issue, single-file plus tree plus regex forms) plus
`tools/sh/snapshot.sh` (`snapshot_diff`, canonical JSON with
`UPDATE_EXPECT`); snapshot is for byte-identical golden outputs with
refresh, `dx_expect_*`/`dx_guard_*` fixed-string pins are for doc/code
contract sentences/symbols (fail-closed, no refresh). Drivers extend the
shared files instead of copying; `//tools/ci:shell_contract` owns the rule.

Snapshot versus grep policy: use snapshot (`snapshot_diff` with
`UPDATE_EXPECT`) when whole-file byte identity matters (renderer output,
generated fragments, canonical JSON: the reviewer sees the diff and
refreshes explicitly). Use literal grep table rows (`dx_guard_*` with
`grep -F -e`, one row per file plus reason) when a few contract
sentences/symbols must hold in a known file. Use regex rows
(`dx_guard_re_*` with `grep -E -e`) only for shapes (SHA pins, version
alternatives, anchors). Use tree rows only when the location is unknown
(repo-wide absence with an `--include` glob); prefer single-file pins
otherwise.

Runfiles and workspace-root probing is consolidated under issue #319 (one shared
`tools/sh/lib.sh` `dx_workspace_root`/`dx_runfiles_root`/`dx_resolve_runfile`
plus `rlocation` usage; shell drivers load it via `tools/sh/bootstrap.sh`
`dx_bootstrap` with `data = ["//tools/sh:lib"]`; Rust binaries share
`dx_process::workspace_start` and use standard `runfiles` `rlocation`,
never `TEST_SRCDIR` in prod).

Host-tool actions are hermetic under issue #318 (archive/SBOM/BCR
genrules run toolchain-provided Python archiver/hasher/generators as
declared `tools` with deterministic bytes and no host
`tar`/`sha256sum`/`shasum`/`python3` probing; `extension.bzl` uses
Bazel-native `ctx.download(executable=True)` plus archive-carried modes,
with one `chmod +x` for the gzip single-file member (taplo, no mode in
gzip header); deploy runtime needs bash + python3 +
POSIX coreutils only with hashing, realpath, and tar listing through
python3).

Generated deploy launchers use `sh_binary` plus `runfiles.bash` `rlocation`
with `shell.quote` (issue #317).
