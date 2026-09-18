# Tools

Managed tool inventory and delivery policy:

- [Tool Acquisition](tool-acquisition.md): consumer delivery, bootstrap seed maintenance,
  promotion, and planned routing.
- [First-Release Tool Baseline](tool-baseline.md): frozen final integration scope and curated
  defaults, not current support status.

Verification is defined by the [Tool And Platform Test Matrix](../testing/tools.md) and
[Testing Strategy](../testing/README.md).

## Language Mapping Qualification

Accepted. Each foundation keeps its provisional upstream; no switch is approved here.

Core tool-graph: `rustfmt` and Clippy via the pinned Rust toolchain;
Ruff and Ty via checksummed standalone artifacts
(`quality/artifacts/ruff.linux_x86_64.bzl`, `quality/artifacts/ty.linux_x86_64.bzl`);
Biome, Buildifier, Taplo, and Vale via their standalone artifacts.
Ty provenance is pinned: upstream `0.0.80` with URL, sha256, and licenses in
`quality/artifacts/ty.linux_x86_64.bzl`.
Additional-language adapters (Java, Kotlin, Scala, C#, F#, C/C++, Go) have no
claimed adapter yet; per-tool qualification stays open under issue #307.
Ruby and PowerShell tool cohorts stay deferred beyond v1 by
[ADR 0019](../decisions/0019-first-release-additional-foundations.md);
Swift is excluded from v1 by the same record.

Pinned by `bazel run //tools/ci:foundation_maps`.
