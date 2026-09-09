# ADR 0008: Dependency Currency

## Status

Accepted.

## Context

`rules_dx` exists to provide a working, current developer environment. Allowing
old default dependencies would shift setup and compatibility work back to
consumers, while floating versions would make builds and releases irreproducible.

## Decision

Every Bazel module, Rust crate and toolchain, integrated developer tool, and other
repository-managed dependency uses the latest stable release available when it is
added or deliberately updated. Selected versions are pinned exactly, with artifact
checksums where applicable.

These pins form ADR 0014's tested default platform stack. Normal root-module Bzlmod selection
and overrides remain consumer-owned and receive no mismatch warning merely for resolving a
different graph. The latest-stable requirement governs release defaults and release CI, not an
attempt to force transitive versions in consumer roots.

Prerelease, release-candidate, beta, and nightly versions are not considered stable.
A documented compatibility exception may retain an older stable version when the
latest release is incompatible with the supported Bazel version, required OS/CPU
matrix, another mandatory dependency, or an accepted product constraint. An
accepted ADR may approve a prerelease only when no stable release provides a
required foundation; it must define validation, upgrade, and release-blocking
conditions.

For the hermetic Windows native backend required by
[ADR 0014](0014-tested-platform-release-stack.md#decision), prefer a published stable upstream
release. When no suitable release supplies the required integration, an exact upstream commit pin
is permitted, with archive integrity and pinned acquisition inputs. Record why the release route is
insufficient, qualification results, maintenance ownership, and tracking for migration to a suitable
release. Missing required workflow, hermeticity, licensing, or interoperability evidence still blocks
release; this exception neither selects a backend nor authorizes an unbounded fork.

Maintainer automation checks for newer stable releases. A `rules_dx` release is
blocked while managed dependencies have pending stable updates unless each
exception is approved and recorded in the release report. Updates must pass the
relevant repository, consumer, and platform tests before adoption.

### Rustdoc Extraction Exception

The accepted documentation pipeline may use one exactly pinned, checksummed nightly Rust toolchain
for `rustdoc -Z unstable-options --output-format json`. Upstream documents JSON as an
[experimental output](https://doc.rust-lang.org/rustdoc/unstable-features.html#json); stable HTML
does not provide the selected machine-input contract. This exception is limited to documentation
extraction and does not change the normal application compiler or quality-tool selection.

O54 must qualify the extractor against each supported target compiler configuration and its dependency
metadata. Where compiler versions cannot share artifacts, extraction must obtain compatible declared
inputs through the qualified Bazel route; an ambient compiler or silent semantic fallback is not
permitted. Incompatibility blocks the affected integration rather than changing the application pin.

Maintainers own pin updates and the [documentation drift suite](../documentation/doc-ir.md#drift-policy).
Each release records the exact nightly, why stable extraction is still insufficient, and passing
completeness, compatibility, and deterministic-rebuild evidence. Missing evidence or an unqualified
pin update blocks release. Requalify a stable machine-input route when available; this exception
does not grant general nightly use or establish current extractor support.

## Consequences

- Bzlmod dependencies default to exact latest-stable versions rather than compatibility ranges or
  development commits; documented accepted exceptions retain exact immutable identities.
- Reproducibility comes from exact pins, not from retaining old defaults.
- Dependency update automation and cross-platform tests are release requirements.
- Consumer-selected dependencies outside `rules_dx` remain under normal Bzlmod or
  ecosystem resolution and are not silently rewritten by the project.

## Rejected Alternatives

- Floating dependency versions.
- Old conservative defaults without a concrete compatibility reason.
- Nightly or prerelease defaults without an explicit accepted exception.
- A special version policy only for `rules_rust` or developer tools.
