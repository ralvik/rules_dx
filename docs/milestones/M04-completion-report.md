# M04 Completion Report: Rust, Starlark, TOML, And Markdown Acquisition And Adapters

Seed host: local Linux x86_64 glibc (same host class as the M00 seed report).
Local execution only, no remote. Non-Linux platforms were unavailable and are
recorded as gaps, not claimed.

## Scope Completed And Deferred

All three work packages are complete: toolchain/standalone acquisition identity
(WP1), capability manifests, native config, diagnostics, and edits (WP2), and the
repository-owned Markdown link/structure checker (WP3). Prettier is deferred per
milestone scope, with no other silent deferrals.

Capability-term transitions: `rustfmt`, `clippy`, `buildifier`, `taplo`, `vale`,
and `markdown_check` are **Adapter-tested** (focused contract, acquisition, and
fixture suites pass on the seed host). None is **Supported** (no release-level
evidence) or **Dogfooded** (repository-wide enforcement is M05).

## Exact Pins

Bazelisk `v1.29.0` launching Bazel `9.2.0` via `.bazelversion`; `rules_rust 0.74.0`,
`rules_cc 0.2.22`, `rules_python 1.7.0`, `platforms 1.1.0`; Rust `1.98.0`
(`edition = "2021"`); `rustfmt 1.98.0`; Buildifier `8.5.1`; Taplo `0.10.0`;
Vale `3.20.0`; Clippy `0.1.98` (as shipped by the pinned toolchain). Standalone
URL/digest/size/archive/ABI/license records are pinned by
`//quality/artifacts:metadata` with lazy per-tool/platform repositories behind
the `@dx_tools` execution-platform hub; `rustfmt`/`clippy` ride the selected
stable toolchain (`rustfmt_version = "1.98.0"`, identity pinned by
`//rust/toolchains:identity`).

## Adapter Evidence Cells

Per-adapter cells (VCS-isolation, exact-input, no-config, edit, cache,
empty-`PATH`), all executed on the seed host:

- **Exact-input:** `bazel aquery <fixture> --output_groups=dx_results` shows
  every `DxRealQualityLint`/`DxRealQualityFormat` action's `Inputs` are exactly
  the runner, the stage tool binaries, the direct sources, and the hinted config
  closure (e.g. markdown lint: `quality_markdown` + `vale` binaries,
  `real_clean.md`, `vale_test.ini`, `styles/.keep`; hinted rust lint additionally
  `clippy.toml`). Analysis presence pins (`real_aspect_presence`) fix the
  capability shape per fixture: markdown 1 lint result; rust/starlark/toml/mixed
  2 results; `no-lint` tag yields format only.
- **VCS-isolation:** no action input references `.git`, `.gitignore`, home, or
  any undeclared path; the runner materializes only declared mirrors into a
  fresh scratch tree and never observes VCS state.
- **No-config:** unhinted actions carry no `--tool-config`. Unit tests pin
  `rustfmt` materialized empty defaults, Buildifier `--config=off`, Taplo
  `--no-auto-config`/`--no-schema`, Clippy scratch-root cwd, and the check-only
  config-free `markdown_check`. No-config golden executions agree with direct
  pinned-binary runs: `rustfmt --check` clean on `clean.rs`/`hinted.rs`;
  `clippy-driver` silent on `clean.rs`; Buildifier `success` with no warnings on
  `real_clean.bzl`; `taplo lint` exit `0` on `real_clean.toml`. Applicable Vale
  without a hint fails analysis with the actionable config-required error
  (`fixture_real_markdown_no_config_subject`, `--nobuild`).
- **Config binding:** hinted clippy pins cwd to the mirrored config's parent
  (`hint_dir`); the hint file must carry the discoverable basename
  (`clippy.toml`; a `clippy_test.toml` probe is silently ignored). End to end,
  `fixture_real_rust_hinted` (`too-many-arguments-threshold = 2`) yields
  `clippy::too_many_arguments` where the unhinted fixture stays silent
  (evaluator `--fail_on warning`: hinted exit `1`, unhinted exit `0`).
- **Edit:** `apply_fix` unit tests per tool (Clippy machine-applicable
  suggestion application, rustfmt, Buildifier, Taplo convergence sets,
  `markdown_check`/`vale` check-only identity).
- **Empty-`PATH`:** `exec::hermetic_env` never sets `PATH` (unit-pinned) and
  every backend spawn test asserts the hermetic environment.
- **Cache:** action keys are recorded per action (e.g. hinted rust lint
  `87284c6a…`); key content covers exact inputs, configs, and tool binaries.
  No-op/narrow-change/config-change/warm-cache behavior runs belong to M05
  dogfood evidence and are deferred there, not claimed here.

Evaluator agreement: 11 of 12 produced `dx_results` protobufs pass
`--fail_on warning`; the single failure is the designed hinted-clippy signal.

## Commands And Results

- `bazel build //...`: success (78 processes on the closing run).
- `bazel test //...`: 39/39 pass, including per-crate fmt/clippy tests,
  `real_aspect_presence`, pipeline/registry unit tests, and acquisition metadata
  tests.
- `bazel coverage //...` plus `bazel run //tools/coverage:check`: gate
  **PASS 6123/6123** executable lines against `tools/coverage/inventory.txt`.
- Buildifier `8.5.1 --mode=check` clean on all touched Starlark files.
- Work is staged, not committed (89 files, domain restructure
  `tools/{starlark,testing,artifacts,toolchains}` to `libs/` + `quality/` +
  `rust/` included).

## Changed Components And Public Labels

`quality/adapter` (commands, hermetic exec, parsers), `quality/runner`
(`--real` backend, scratch convergence), `quality/evaluator`, `quality/result`,
`quality/markdown` (`quality_markdown` binary), `quality/artifacts`
(`extension.bzl%dx_tools`, `update` generator), `rust/toolchains` (identity),
`libs/starlark`, `libs/testing`, fixture corpus under `quality/testdata`.
Public labels: `REAL_ADAPTERS`, `real_*_family`, `real_lint_aspect` /
`real_format_aspect`, `<tool>_config` rules with `DxNativeConfigInfo`,
`collect_native_configs`. No consumer-facing CLI or support claim is added.

## Documentation Changes And Unresolved Items

Updated `tool-integrations.md` (WP3 boundary, wiring contracts, frozen Clippy
filename), `native-configuration.md` (authority), `quality-testing.md`
(initial adapter evidence incl. the Markdown exit matrix), `open-decisions.md`
(O20: boundary, wiring, and config bindings frozen; execution dogfood done for
Markdown and Clippy-hint). O20 remaining: non-Linux platforms, Taplo parser
requalification on version update, Vale runtime-closure sandbox proof, and the
M05 cache-behavior matrix. Milestone exclusions and contract boundaries were
respected: no Prettier, no CLI quality commands, no repository-wide enforcement,
Markdown checks distinct from Vale.
