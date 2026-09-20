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
`quality/artifacts/ty.linux_x86_64.bzl`. Ty maps to `typecheck` over `python`/`python_stub`,
Ruff maps to `format`/`lint` over the same classes, and Biome/ESLint/Prettier/`tsc` map
JS/TS classes per `quality/adapters.bzl` with parsers in `quality/adapter/src/parsers/`
(`ty.rs`, `ruff.rs`, `biome.rs`, `eslint.rs`, `prettier.rs`, `tsc.rs`); required-core
quality mappings stay owned under issue #303.
Quality adapters plus parity plus packaging are qualified under issue #307 with
fixture evidence (`quality/testdata/runner_matrix_cases.bzl` pass/fail plus fix/format,
`quality/native_config.bzl` bindings or explicit config-free/delegated status,
`quality/artifacts/metadata_tests.bzl` plus `update.py --verify-only`, `cli/qualification`
single-correct-path plus SPDX/SLSA wire profiles; Buildifier/Taplo/Vale probes stay
provisional) with deferred implementation owned by ADR 0019.
Additional-language adapters (Java, Kotlin, Scala, C#, F#, C/C++, Go) have no
claimed adapter yet; foundation-side classification stays owned under issue #304 and
per-tool qualification is qualified under issue #307 with deferred routes owned by ADR 0019.
Ruby and PowerShell tool cohorts stay deferred beyond v1 by
[ADR 0019](../decisions/0019-first-release-additional-foundations.md);
Swift is excluded from v1 by the same record.
Deferred/excluded tool record stays owned under issue #305: retained RuboCop/StandardRB plus
PSScriptAnalyzer cohorts keep their frozen routes in
[Tool Acquisition](tool-acquisition.md#first-release-tool-routing) with `ruby`/`powershell`
classes classified but no adapter claim (`quality/adapters.bzl` plus `quality/parity_tests.bzl`
with ADR 0019); Swift/SwiftFormat plus Bandit stay excluded with host-toolchain
fallback never approved.

Pinned by `bazel run //tools/ci:foundation_maps`.
