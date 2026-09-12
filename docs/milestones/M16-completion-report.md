# M16 Completion Report: JavaScript And TypeScript Application Foundation

Seed host: local Linux x86_64 glibc (same host class as the M00 seed report).
Local execution only, no remote. Non-Linux platforms were unavailable and are
recorded as gaps, not claimed. No capability-term transitions are claimed:
the foundation is adapter-tested first-party implementation under test, not a
product support claim. Public repository/root/exact-target env planning,
collection, and orchestration remain M25.

## WP1: Upstream Providers, pnpm Importers/Stores, Jest, TypeScript, Versions

Pinned `aspect_rules_js 3.4.1`, `aspect_rules_ts 3.10.0`,
`aspect_rules_jest 0.26.0` (`MODULE.bazel`), per ADR 0013 latest-stable
selection. Toolchains resolved from executed runs: Node `22.22.2`
(rules_js default toolchain), pnpm `10.34.5` (root `packageManager`),
TypeScript `5.9.3` (`typescript.deps`, latest stable 5.x in
`TOOL_VERSIONS`; 6.x native preview stays out), Jest `30.2.0` +
`jest-junit 16.0.0` (authoritative pnpm graph). ADR 0012 release-default:
wrappers accept no version fields; unknown versions fail in upstream
toolchain resolution, never in the wrappers.

`javascript/rules/defs.bzl`: narrow `dx_js_library` / `dx_js_binary` /
`dx_js_test`. Each creates one private `<name>_dx_upstream`
(`js_library` / `js_binary` / `jest_test`) plus one public forwarding
rule. Libraries preserve `JsInfo` + `DefaultInfo` +
`InstrumentedFilesInfo` and add only
`QualitySourcesInfo(direct_sources = {"javascript"})` normalized from
direct `.js/.jsx/.mjs/.cjs` srcs. Binaries symlink the upstream
executable and forward `JsInfo`/`InstrumentedFilesInfo`/
`OutputGroupInfo`/`RunEnvironmentInfo` only when upstream carries them;
upstream `js_binary` carries no `JsInfo` (verified by cquery), so
libraries are the `JsInfo` carriers and thin entries report no direct
sources. Tests symlink the upstream launcher, rebuild
`testing.TestEnvironment` from mirrored `env_inherit` (always adding
`TESTBRIDGE_TEST_ONLY` for sharding/filter), forward
`InstrumentedFilesInfo` only when coverage enables it, and carry the
`_lcov_merger` magic attribute so coverage merges `coverage.dat`. The
private upstream test target is tagged `manual` so `bazel test //...`
exercises the public wrapper only. Jest runs CommonJS by default while
first-party `.js` is ESM (`"type": "module"`); tests covering ESM pass
`node_options = ["--experimental-vm-modules"]`, and the macro always
adds `//:package_json` (via the `//:package_json` `js_library` hop) so
jest's ESM scope detection resolves. `typescript/rules/defs.bzl`:
narrow `dx_ts_project` over `ts_project` (transpiler `tsc`,
`@npm_typescript`), preserving `JsInfo` + `TsConfigInfo` +
`DefaultInfo` + `InstrumentedFilesInfo` and adding only
`QualitySourcesInfo(direct_sources = {"typescript", "tsx"})`;
`.d.ts/.d.mts/.d.cts` are inert and must not be listed as srcs. There
is no separate `dx_ts_binary`/`dx_ts_test`; TypeScript execution reuses
the JavaScript wrappers over compiled outputs.

Authoritative pnpm graph (`package.json` + `pnpm-lock.yaml` +
`pnpm-workspace.yaml`, hub `npm` via `npm_translate_lock`,
`verify_node_modules_ignored` against `.bazelignore`): single importer,
`allowBuilds: {}`, `hoist=false` (`.npmrc`) so host layout matches the
managed tree. The lock was generated with the Bazel-pinned pnpm;
builds never invoke the package manager. Managed root facade
`//:node_modules` via `npm_link_all_packages`; with one importer the
root facade is also the complete importer-local facade (multi-importer
per-importer facades and orchestration remain M25).

Fixtures: `//javascript/hello` (`hello_lib`, thin binary `hello` with
`entry_point = "main.js"` + `data = [":hello_lib"]`, `hello_test` over
`//:node_modules`) and `//javascript/entries` (`helper`, `main` with
`deps = [":helper"]`, thin `main_bin`, `helper_test` with
`jest.config.cjs` matching the one-source `*_test.js` naming);
`//typescript/hello` (`hello_lib`, `transpiler = "tsc"`,
`tsconfig = "tsconfig.json"`) and `//typescript/entries` (`helper` +
`main` with `declaration = True`, `deps` edge). `bazel run
//javascript/hello:hello` prints `hello world`; `bazel run
//javascript/entries:main_bin` prints `entry <x>`; `bazel test
//javascript/hello:hello_test //javascript/entries:helper_test` pass;
`bazel coverage //javascript/...` passes with `coverage.dat` for both
tests; `bazel build //typescript/hello:hello_lib` emits `hello.js`
and the upstream typecheck test passes.

## WP2: Extensions, Ownership, Tests, Entries, Strict Imports, Merge

First-party `gazelle/javascript` and `gazelle/typescript` extensions
(`lang.go`, `parser.go`, `naming.go`, `stdlib.go`): JS discovers
`.js/.jsx/.mjs/.cjs`, TS discovers `.ts/.tsx/.mts/.cts`; declaration
files, source maps, and `.vue/.svelte/.astro/.mdx` containers are
inert. One reusable one-source `dx_js_library` / `dx_ts_project` per
supported non-test source (basename normalizer), `dx_js_test` per
`*_test` source only (prefix/directory placement never create tests;
TS test sources stay `dx_ts_project` leaves that never provide
imports), thin `dx_js_binary <library>_bin` for entry sources carrying
only `entry_point` + `data = [":<library>"]`. Same-package claims
across extensions/languages fail rather than gaining a suffix;
computed `import()`/`require()` names are the manual kept boundary.
`ParseImports` covers literal imports plus recognized literal
`import("name")`/`require("name")`; stdlib roots dropped at
collection; every other root resolves strictly against the
importer-selected scope or fails (`unresolved import ... add a local
one-source library or an exact # gazelle:resolve mapping`); lockfile
presence alone never activates a dependency. Exact
`# gazelle:dx_ignore_import` (`javascript [javascript] <import>` /
`typescript [typescript] <import>`): ignored literal contributes no
edge, mapping+ignore fails, stale ignore fails. Generated tests default
`node_modules = "//:node_modules"` (mergeable root facade); binaries
match by `entry_point` so Gazelle never deletes thin binaries as
empty. Source-only local graphs generate with no ecosystem metadata
and no sidecars (`find` shows none; generation contract forbids them).

Pinned by `lang_test.go` (source-only, entries, strict, ignore,
collision, merge/stale suites) plus golden `testdata/source_only` and
`testdata/entries` (`BUILD.in`/`BUILD.out`): JS `demo.js`/`helper.jsx`/
`helper_test.mjs`, TS `demo.ts`/`helper.tsx`/`helper_test.mts` plus
inert `widget.d.ts`. `//gazelle/javascript:generation_test`,
`//gazelle/javascript:javascript_test`,
`//gazelle/typescript:generation_test`,
`//gazelle/typescript:typescript_test` pass.

## WP3: Managed Root And Focused Provider-Derived Plans

`javascript/env/plan.bzl` (`javascript_env_plan`) and
`typescript/env/plan.bzl` (`typescript_env_plan`): for one wrapper
target, read analyzed `JsInfo.transitive_sources` (first-party
basenames), `QualitySourcesInfo` direct basenames, `JsInfo.npm_sources`
size, and (TS) `TsConfigInfo` presence. Materialize deterministic JSON
+ `JavaScriptEnvPlanInfo` / `TypeScriptEnvPlanInfo` + `DxSubjectInfo`.
No checkout scan, no pnpm invocation, no re-resolution, no `tsc`
execution, no mutation, no repository/root/exact-target orchestration
(M25).

`//javascript/env:env_plan_tests` and `//typescript/env:env_plan_tests`
(analysis `starlark_test`) pin six plans: JS `hello_lib`
(`hello.js`), `helper` (`helper.js`), `main` (direct `main.js`,
transitive `helper.js,main.js`); TS `hello_lib` (direct `hello.ts`,
transitive compiled `hello.js`, `has_tsconfig=True`), `helper`,
`main` (transitive `helper.js,main.js`). All six record `has_npm=False`,
`npm_source_count=0`. Passes. Laziness: source-only closures project
zero npm payloads; the managed `//:node_modules` facade resolves only
through the test/jest edge, never through library builds. The JSON
outputs match the pinned `plan_tests.bzl` observations verbatim.

## Evidence

Exact commands on this host, committed tree:

- `bazel build //...`: success (376 targets, 435 actions).
- `bazel test //...`: 96/96 pass, including
  `//gazelle/javascript:generation_test`,
  `//gazelle/javascript:javascript_test`,
  `//gazelle/typescript:generation_test`,
  `//gazelle/typescript:typescript_test`,
  `//javascript/hello:hello_test`, `//javascript/entries:helper_test`,
  `//javascript/env:env_plan_tests`,
  `//typescript/env:env_plan_tests`,
  `//typescript/hello:hello_lib_dx_upstream_typecheck_test`.
- `bazel coverage //...`: 87/87 pass (coverage-instrumented subset).
- Coverage gate (`//tools/coverage:check` over LCOV vs inventory vs
  Bazel-declared `*.rs`/`*.go`): **PASS 25985/25985** executable lines.
- `bazel run //dx:generate_check`: `EXIT=0` (clean).
- `bazel run //tools/bazelrc:preset.update -- --verify-only`: preset
  files verified.
- `bazel run //javascript/hello:hello`: `hello world`.
  `bazel run //javascript/entries:main_bin`: `entry <x>`.
- `bazel build //javascript/env:hello_lib_plan
  //typescript/env:hello_lib_plan` JSON matches the pinned
  `plan_tests.bzl` observations verbatim.
- Provider probes: `//javascript/hello:hello_dx_upstream` and
  `//javascript/hello:hello` expose no `JsInfo` (upstream executable
  design); `//javascript/hello:hello_lib` preserves `JsInfo`;
  `//typescript/hello:hello_lib` preserves `JsInfo`/`TsConfigInfo`.
- Toolchain probes: toolchain `node --version` reports `v22.22.2`;
  `bazel run @pnpm//:pnpm -- --version` reports `10.34.5`;
  `@npm_typescript` package reports `5.9.3`.

## Changed Components

- `MODULE.bazel` (aspect_rules_js/ts/jest pins, TypeScript 5.9.3
  toolchain, pnpm + `npm` lock translation), `MODULE.bazel.lock`,
  `package.json`, `pnpm-workspace.yaml`, `pnpm-lock.yaml`, `.npmrc`,
  `.bazelignore`, root `BUILD.bazel` (`//:package_json` scope marker,
  `//:node_modules` root facade).
- `javascript/rules/defs.bzl`, `javascript/hello/`,
  `javascript/entries/` (`jest.config.cjs`), `javascript/env/`
  (`plan.bzl`, `plan_tests.bzl`, BUILD files).
- `typescript/rules/defs.bzl`, `typescript/hello/`,
  `typescript/entries/`, `typescript/env/` (`plan.bzl`,
  `plan_tests.bzl`, BUILD files).
- `gazelle/javascript/`, `gazelle/typescript/` (extensions + tests +
  golden testdata).
- `tools/coverage/inventory.txt` (new Go eligible sources), corpus
  `BUILD.bazel` ownership for new packages.
- This report.

## Open Items

- O27 mappings remain provisional pending required-platform and
  consumer evidence; no supported claim.
- Non-Linux hosts, remote execution, and clean external-consumer
  evidence are unproven (same gap class as M00/M12/M14).
- Multi-importer per-importer facades, repository/root/exact-target
  orchestration, JS/TS quality tools (M17), framework adapters
  (M18-M21), and audit/update (M26) are out of scope and untouched.
