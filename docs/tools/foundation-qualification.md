# Tool Foundation Qualification

Tool-graph mapping and per-foundation adapter evidence.
The [Tools index](README.md) stays short; this document owns the qualification record.

## Language Mapping Qualification

Accepted. Each foundation keeps its provisional upstream; no switch is approved here.

Core tool-graph: `rustfmt` and Clippy via the pinned Rust toolchain;
Ruff and Ty via checksummed standalone artifacts across four hosts
(`quality/artifacts/ruff.*.bzl`, `quality/artifacts/ty.*.bzl`: `linux_x86_64`,
`linux_arm64`, `macos_arm64`, `windows_x86_64` under issue
#616; macOS x86_64 removed per #976); Biome, Buildifier, Taplo, and Vale via their standalone artifacts
across the same four hosts.
Ty provenance is pinned: upstream `0.0.80` with URL, sha256, and licenses in
`quality/artifacts/ty.linux_x86_64.bzl` plus per-host siblings. Ty maps to `typecheck` over `python`/`python_stub`,
Ruff maps to `format`/`lint` over the same classes, and Biome/ESLint/Prettier/`tsc` map
JS/TS classes per `quality/adapters.bzl` with parsers in `quality/adapter/src/parsers/`
(`ty.rs`, `ruff.rs`, `biome.rs`, `eslint.rs`, `prettier.rs`, `tsc.rs`); required-core Rust
integration stays pinned under issue #470 with remaining native gaps owned under issues
#471, #472, #473, #474, #475.
Quality adapters plus parity plus packaging are qualified under issue #307 with
fixture evidence (`quality/testdata/runner_matrix_cases.bzl` pass/fail plus fix/format,
`quality/native_config.bzl` bindings or explicit config-free/delegated status,
`quality/artifacts/metadata_tests.bzl` plus `update.py --verify-only`, `cli/qualification`
single-correct-path plus SPDX/SLSA wire profiles; Buildifier/Taplo/Vale probes stay
provisional) with deferred implementation owned by ADR 0019.
Additional-language adapters (Java, Kotlin, Scala, C#, F#, C/C++, Go) have no
claimed adapter yet; foundation-side classification stays owned under issues #476-#484 and
per-tool qualification is qualified under issue #307 with deferred routes owned by ADR 0019.
Ruby and PowerShell tool cohorts stay v1 scope; their foundations are admitted
to v1 by [ADR 0032](../decisions/0032-ruby-powershell-bandit-swift.md)
(superseding the [ADR 0019](../decisions/0019-first-release-additional-foundations.md)
deferral), with Ruby foundation delivered in the Ruby track and PowerShell
foundation delivered provisionally under issue #972 over the portable `pwsh`
runtime with the Gallery lock;
PSScriptAnalyzer adapter wiring (console-parse vs library-API binding)
stays owned under #800;
Swift is excluded from v1 by the same record, re-evidenced by ADR 0032.
Deferred/excluded tool record is decided by [ADR 0032](../decisions/0032-ruby-powershell-bandit-swift.md):
retained RuboCop/StandardRB plus
PSScriptAnalyzer cohorts keep their frozen routes in
[Tool Acquisition](tool-acquisition.md#first-release-tool-routing) with `ruby`/`powershell`
classes classified with `ruby` foundation delivered plus no foundation adapter
claim yet (`quality/adapters.bzl` plus `quality/parity_tests.bzl`
with ADR 0019, foundation admission by ADR 0032); Swift/SwiftFormat stay excluded with host-toolchain
fallback never approved plus Bandit excluded from v1 by ADR 0019 and re-selected by ADR 0032 with wiring pending under #801.

Pinned by `bazel run //tools/ci:foundation_maps`.
