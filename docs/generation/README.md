# Generation

These documents are the authoritative contracts for first-party BUILD generation. The accepted
ADRs explain why the contracts exist; the test matrix defines the required evidence.

## Contracts

- [Common](common.md): Gazelle APIs, resolution, ownership, merge, lifecycle, resources, and entry
  points.
- [Rust](rust.md): crate and Cargo discovery, tests, examples, benchmarks, and build scripts.
- [Python](python.md): runtime sources, stubs, tests, and uv-backed dependency scope.
- [JavaScript and TypeScript](javascript-typescript.md): core extensions, pnpm scope, and tests.
- [Go](go.md): package-level libraries and tests with the build-constraint exception.
- [Java](java.md): directory libraries over the shared Maven scope.
- [Kotlin](kotlin.md): directory libraries over the shared Maven scope.
- [Scala](scala.md): directory libraries over the shared Maven scope.
- [C#](csharp.md): directory libraries over the Paket scope.
- [F#](fsharp.md): directory libraries over the Paket scope.
- [C/C++](cc.md): directory libraries with sources plus headers and no lockfile.
- [Ruby](ruby.md): directory libraries over the shared Bundler scope.
- [PowerShell](powershell.md): handwritten wrappers over the portable pwsh runtime with the explicitly scoped no-Gazelle alternative.
- [Mixed ownership](mixed.md): disjoint v1 container partition.
- [Framework adapters](framework-adapters.md): Vue, Svelte, Astro, and MDX container boundaries.

## Related Authorities

- [`dx generate`](../cli/commands/generate.md) defines the command workflow and output behavior.
- [ADR 0015](../decisions/0015-first-party-gazelle-extensions.md),
  [ADR 0013](../decisions/0013-rust-javascript-typescript-foundations.md), and
  [ADR 0010](../decisions/0010-python-foundation.md) record the rationale.
- [Generation test matrix](../testing/generation.md) defines conformance evidence.


## Qualification

Foundation-to-upstream mapping and lock/runner evidence lives in [Foundation Qualification](foundation-qualification.md).

```sh
bazel run //cli/cli:dx -- generate //...
bazel run //cli/cli:dx -- generate --check //...
```
