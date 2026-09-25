# Support Matrix

Pre-release. No cell is `Supported`. Tracking lives in GitHub issues.


## Platform-qualified targets

| Consumer | Status | consumer self-call: test-disabled |
| --- | --- | --- |
| Linux x86_64 glibc | Platform-qualified | test-disabled |
| Linux arm64 glibc | Platform-qualified (qualified under issue #410) | test-disabled |
| macOS arm64 | Platform-qualified | test-disabled |
| macOS x86_64 | Not planned | test-disabled |
| Windows x86_64 MSVC-compatible | Platform-qualified (issue #414) | test-disabled |

## Status Lifecycle

`Planned` → `Seed-host-delivered` → `Platform-qualified` → `Supported`.

`Planned` is accepted scope only. `Seed-host-delivered` is implemented and
verified on the Linux x86_64 seed host. `Platform-qualified` adds
required-platform evidence per
[ADR 0014](../decisions/0014-tested-platform-release-stack.md).
`Supported` adds release evidence and occurs only during release
qualification. Status cells are owned by this matrix; promotion to `Supported` requires release evidence,
enforced by `bazel run //tools/ci:supported_evidence_gate`.

## Supported Today

Nothing is `Supported` yet.

Foundations in scope are Rust, Python, JavaScript, TypeScript, Vue, Svelte,
Astro, MDX plus Go, C/C++, Java, Kotlin, Scala, C#, F#, Ruby, and PowerShell.
Required platforms are owned by ADR 0014. Per-host and per-cell pins, hosts,
floors, SDK/CRT identities, routes, coverage, and consumer plus release
evidence live in their owning issues and qualification fixtures, not here.

## Out of Scope

- Swift: no hermetic toolchain over all required hosts.
- macOS x86_64: Not planned, never planned for support (#976).
- Windows arm64: out of v1 scope.
- Dynamic musl explicitly out of scope; static musl profiles only.
- Unqualified hosts: `dx` refuses cleanly with `unsupported_platform`
  before any Bazel work starts.

## Related Issues

Tracking lives in GitHub issues; this matrix stays the status source. See
[product scope](scope.md) for the boundary and ADR 0014 for required hosts.
