# Generation

These documents are the authoritative contracts for first-party BUILD generation. The accepted
ADRs explain why the contracts exist; the test matrix defines the required evidence.

## Contracts

- [Common](common.md): Gazelle APIs, resolution, ownership, merge, lifecycle, resources, and entry
  points.
- [Rust](rust.md): crate and Cargo discovery, tests, examples, benchmarks, and build scripts.
- [Python](python.md): runtime sources, stubs, tests, and uv-backed dependency scope.
- [JavaScript and TypeScript](javascript-typescript.md): core extensions, pnpm scope, and tests.
- [Framework adapters](framework-adapters.md): Vue, Svelte, Astro, and MDX container boundaries.

## Related Authorities

- [`dx generate`](../cli/commands/generate.md) defines the command workflow and output behavior.
- [ADR 0015](../decisions/0015-first-party-gazelle-extensions.md),
  [ADR 0013](../decisions/0013-rust-javascript-typescript-foundations.md), and
  [ADR 0010](../decisions/0010-python-foundation.md) record the rationale.
- [Generation test matrix](../testing/generation.md) defines conformance evidence.

## Language Mapping Qualification

Accepted. Each foundation keeps its provisional upstream; no switch is approved here.

Core (Rust, Python, JavaScript, TypeScript) plus admitted additional foundations
(Go, Java, Kotlin, Scala, C#, F#, C/C++) are qualified by focused fixtures:
thin wrappers in `<lang>/rules/defs.bzl` preserving the upstream provider and
adding `QualitySourcesInfo`, Gazelle extensions in `gazelle/<lang>/` with
parser/naming/lang fixtures plus focused tests, hello builds in `<lang>/hello/`
as wrapper consumers, and lock authority (`rust/hello/Cargo.lock` plus
`cargo-bazel-lock.json` and `MODULE.bazel.lock`, `python/hello/uv.lock` plus
tools lock, `pnpm-lock.yaml` plus tools lock,
`third_party/jvm/maven_install.json`, `third_party/dotnet/paket.lock` plus
`paket.dependencies`). Go hello is stdlib-only with no ecosystem lock;
C/C++ has none (every `http_archive` carries `sha256`/`integrity`).
Upstream pins live in `MODULE.bazel`.
Ruby and PowerShell have no generation mapping: deferred beyond v1 by
[ADR 0019](../decisions/0019-first-release-additional-foundations.md).
Swift has none: excluded from v1 by the same record.

Pinned by `bazel run //tools/ci:foundation_maps`.
