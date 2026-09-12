# M17 Completion Report: JavaScript And TypeScript Quality

Seed host: local Linux x86_64 glibc (same host class as the M00 seed report).
Local execution only, no remote. Non-Linux platforms were unavailable and are
recorded as gaps, not claimed. No capability-term transitions are claimed:
the adapters are adapter-tested first-party implementation under test, not a
product support claim. O28 stays open for its owner; this report supplies the
pending evidence (exact mappings, private graph/runtime proofs, measured
convergence order).

## WP1: Biome Artifacts, Private Node Graph, Authoritative tsc

Biome `2.5.12` standalone artifact
(`quality/artifacts/biome.linux_x86_64.bzl`, generated via
`quality/artifacts/update.py`, `dx_biome` repo in `MODULE.bazel`):
hermetic execution proven with no Node runtime in the Biome path. Private
pure-JavaScript tool graph (`quality/tools/javascript/`: `dx-node-tools`,
`eslint 10.10.0` + `prettier 3.9.6`, `pnpm 10.34.5`; `bin/BUILD.bazel`
wraps the managed `eslint`/`prettier` binaries): the graph is private to
`rules_dx`, never exported to consumers. Shared managed Node runtime is the
M16 `rules_js` toolchain (Node `22.22.2`); Node wrappers resolve through
`$0.runfiles` with `JS_BINARY__NO_CD_BINDIR=1`, and every spawn double
asserts the hermetic environment (`assert_hermetic`: `TMPDIR` set, `PATH`
absent). Authoritative `tsc` mapping (`quality/tools/typescript/BUILD.bazel`):
`tsc 5.9.3` resolves from the pinned `aspect_rules_ts` toolchain
(`typescript.deps` -> `@npm_typescript`) on the shared managed Node;
`bazel run @npm_typescript//:tsc -- --version` reports `Version 5.9.3`,
`bazel build //typescript/hello:hello_lib` emits `hello.js`,
`bazel test //typescript/hello:hello_lib_dx_upstream_typecheck_test`
passes, and `which node` finds no ambient Node. `tsc` stays pipeline-only
by design (`quality/runner/src/real.rs` documents: target-coupled `tsc`
stays pipeline-only): quality consumes the `typescript_project` context
(`TsConfigInfo` + typecheck output groups + the upstream typecheck test),
never a bare `tsc` file invocation.

## WP2: Diagnostics, Fixes, Formatting, Native Configs, Source Subsets

`quality/adapters.bzl`: `biome` lints and formats `javascript`, `json`,
`jsx`, `typescript`, `tsx`; `eslint` lints `javascript`, `jsx` only (no
TypeScript parser is installed, so `typescript`/`tsx` match no ESLint
configuration and stay out of ESLint stages);
`prettier` formats `javascript`, `json`, `jsx`, `typescript`, `tsx`;
`tsc` typechecks `typescript`, `tsx`. Class-to-family assignment:
`javascript`/`jsx` to the javascript family, `typescript`/`tsx` to the
typescript family, `json` to the json family. Curated defaults
(`quality/real_pipeline_tests.bzl` selections): lint `biome` for
`javascript`/`json`/`typescript`, format `biome` for
`javascript`/`typescript`, format `prettier` for `json`, typecheck `tsc`
for `typescript`; ESLint is a lint opt-in, Prettier a format alternative
for JS/TS. Command shapes (`quality/adapter/src/commands.rs`) pin the
hermetic transports: Biome `--reporter=json --colors=off
--vcs-enabled=false --error-on-warnings` (lint, check-only, no `--write`),
`--write` only for format-fix; ESLint explicit `-c` config plus `-f json`
(config required, missing config fails the action); Prettier `--no-config
--no-editorconfig` with `--check`/`--write`. Parsers
(`quality/adapter/src/parsers.rs`) are fail-closed: Biome lint/format JSON
reports with category/position/severity validation, ESLint JSON with
severity/rule/extent validation (null-`ruleId` fatal messages are syntax
errors with empty rule ID; null-`ruleId` non-fatal messages are ignored
files and fail), Prettier `--check` stderr `[warn]` lines. Real backend
dispatch and fix rounds (`quality/runner/src/real.rs`): Biome/Prettier
findings arrive working-directory-relative and are re-anchored to absolute
scratch paths; `--config-path` receives a directory holding exactly one
`biome.json`; Biome lint never rewrites; ESLint exit 1 re-reads (mirroring
the Ruff lint-fix contract) while other nonzero exits keep the original
text; Biome/Prettier nonzero fix exits keep the original text. Native
configs: `biome_config` (pinned `{}` defaults materialized as
`dx-biome-default`; hinted config wins and resolves to the hinted
parent, including a root-level `biome.json` resolving to the scratch
root) and `eslint_config` (`.js` flat-config transport, no usable default).
Source-subset fixtures (`quality/testdata/`): `real_clean`/`real_dirty`
for `.js`/`.json`/`.ts`, `real_clean` for `.jsx`/`.tsx`, plus
`biome_cfg/biome.json` and `eslint_cfg/eslint.config.js`; the evaluator
proves dirty-TS `noUnusedVariables` plus not-formatted findings while
cleans pass. Real aspects (`quality/real_aspects.bzl`) wire Biome, ESLint,
and Prettier lint/format aspects.

## WP3: Measured Order, Convergence, Cycles, Iteration Limits

Direct-probe measurements (pinned defaults, no native config): Biome
formats with tabs, Prettier with two spaces, so the same JS input has no
common fixed point; JSON outputs agree byte-for-byte. The committed
composition benchmark models exactly this as stateful normalizers over the
shared `run_convergence` (`quality/runner/src/real.rs`): JSON converges
`Stable` identically, JS converges `Stable` last-writer-wins in 2 rounds
under the lexical `(biome, prettier)` order, and the reversed order
converges `Stable` to the other terminal in 2 rounds. Decision: certify
`Stable` (not oscillation) because each formatter leaves its own style
unchanged and the last stage wins; the order is material, so it stays
frozen lexically and independent of user declaration or result arrival
order (`quality/real_pipeline_tests.bzl` pins lexical JS lint opt-in,
format-alternative, and `tsc` stage order). Genuine cycles and iteration
limits stay certified by the shared protocol (`oscillation_is_detected`,
`iteration_limit_is_detected`, ten-round bound) and fail closed.
Coverage closure for the milestone: the gate stood at 27081/27081 only
after 53 previously unexecuted M17 lines were closed without behavior
change (two `parsed(...)?` restructures so no line holds a bare `?`,
deletion of one provably unreachable empty-warn-path guard after
`trim`, and fail-closed/error-path/fix-failure/idempotence tests).

## Evidence

Exact commands on this host, committed tree:

- `bazel build //...`: success (includes the rustfmt dogfood check).
- `bazel test //...`: 96/96 pass, including `//quality:real_pipeline_unit`,
  `//quality:policy_unit`, `//quality:native_config_unit`,
  `//quality/adapter:quality_adapter_test`,
  `//quality/runner:quality_runner_test`,
  `//quality/testdata:aspect_presence` (`real_aspect_presence` PASSED),
  `//typescript/hello:hello_lib_dx_upstream_typecheck_test`.
- `bazel coverage //...`: 87/87 pass (coverage-instrumented subset).
- Coverage gate (`//tools/coverage:check` over LCOV vs inventory vs
  Bazel-declared `*.rs`/`*.go`): **PASS 27081/27081** executable lines.
- `bazel run //dx:generate_check`: `EXIT=0` (clean).
- `bazel run //tools/bazelrc:preset.update -- --verify-only`: preset
  files verified.
- Corpus ownership audit (`comm` of applicable files vs
  `real_source_target` closure): clean, no output.

## Changed Components

- `MODULE.bazel`, `MODULE.bazel.lock` (`dx_biome` repo), `.bazelignore`.
- `quality/artifacts/` (`biome.linux_x86_64.bzl` generated metadata,
  `extension.bzl`, `metadata_tests.bzl`, `update.py`, `BUILD.bazel`).
- `quality/tools/javascript/` (private `dx-node-tools` graph:
  `package.json`, `pnpm-lock.yaml`, `pnpm-workspace.yaml`, `.npmrc`,
  `BUILD.bazel`, `bin/BUILD.bazel`).
- `quality/tools/typescript/BUILD.bazel` (`tsc` mapping + corpus owner).
- `quality/adapters.bzl`, `quality/BUILD.bazel`, `quality/fixtures.bzl`,
  `quality/native_config.bzl`, `quality/native_config_tests.bzl`,
  `quality/real_aspects.bzl`, `quality/real_pipeline_tests.bzl`.
- `quality/adapter/src/commands.rs`, `quality/adapter/src/parsers.rs`.
- `quality/runner/src/real.rs`.
- `quality/testdata/` (`BUILD.bazel`, `biome_cfg/biome.json`,
  `eslint_cfg/eslint.config.js`, `real_aspect_tests.bzl`,
  `real_clean.{js,json,jsx,ts,tsx}`, `real_dirty.{js,json,ts}`).
- `docs/quality/tool-integrations.md` (JS/TS integration evidence).
- This report.

## Open Items

- O28 stays open for its owner; the pending mappings, private
  graph/runtime proofs, and measured convergence order are supplied above.
- Non-Linux hosts, remote execution, and clean external-consumer
  evidence are unproven (same gap class as M00/M12/M14/M16).
- Prettier plugins outside framework needs and framework build adapters
  are out of scope and untouched. No adapter is called supported.
