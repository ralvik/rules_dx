# M19 Completion Report: Svelte

Seed host: local Linux x86_64 glibc (same host class as the M00 seed report).
Local execution only, no remote. Non-Linux platforms were unavailable and are
recorded as gaps, not claimed. No capability-term transitions are claimed:
the integration is adapter-tested first-party implementation under test, not a
product support claim. O40 stays open for its owner; this report supplies the
milestone-specific evidence (upstream stack, supported syntax/targets,
provider mappings, regions, platforms, limitations).

## WP1: Upstream Parser/Compiler, Package, Provider, Target Boundary

Upstream stack (pinned): `aspect_rules_js 3.4.1` ruleset, `svelte 5.57.0`
runtime/compiler, shared M16 managed Node `22.22.2` toolchain,
authoritative single-importer pnpm graph (root `package.json` +
`pnpm-lock.yaml`, `pnpm 10.34.5`, `allowBuilds:{}`, `hoist=false`).
Decision: reuse the M16 graph rather than a separate Svelte hub;
rationale: Svelte ships as a plain npm package with no lifecycle hooks, so
the existing closure preserves hermeticity with one lock to audit. Narrow
difference from Vue: the `svelte` package itself is the authoritative
compiler (`svelte/compiler`: `parse`/`compile` for script/markup/style
regions); no separate compiler package exists, unlike Vue's
`@vue/compiler-sfc`.

`svelte/rules/defs.bzl` (`dx_svelte_library`): one private
`<name>_dx_upstream` `js_library` plus one public forwarding rule
preserving `JsInfo`, `DefaultInfo`, `InstrumentedFilesInfo` unchanged and
adding `QualitySourcesInfo(direct_sources={"svelte": <direct .svelte>})`.
No `dx_svelte_binary`/`dx_svelte_test`: Svelte execution/tests reuse the
`dx_js_*` wrappers over parsed outputs (narrowest API, mirrors Vue).
`svelte/hello` proves the boundary: `Hello.svelte` (runes instance script
importing `./helper.js`, template markup, style) + `helper.js`,
`Hello.test.js` parses with the pinned `svelte/compiler`
(`parse(source, {modern:true})`, no regex, no plain-JS fallback, no
generation-time compile) and asserts fragment non-empty, instance and css
present, and the instance slice references `./helper.js`.

## WP2: Ownership, Generation, Execution, Env Plan, Quality

Physical ownership: each checked-in `.svelte` has one physical owner
(`dx_svelte_library`); script/markup/style regions never become independent
physical sources or core JS/TS targets (`.svelte` is inert to them).
`gazelle/svelte` (single kind `dx_svelte_library`, `.svelte` only): narrow
block parser (`ExtractScripts` tag scanner honoring comments, quoted attrs,
case-insensitive names; `scan` for imports, never regex), source-only
golden (`testdata/source_only`: `demo.svelte` imports `helper.svelte` +
`fs` stdlib, `helper.svelte` clean), strict resolution (self-deps dropped,
stdlib skipped, handwritten/override/ambiguity/ignore handled, errors
abort before emission), stale sweep (`mergeStale`/`checkClaims`),
single-word `dx_ignore_import` with inheritance and stale-fail-closed.

Narrow difference from Vue, documented in `parser.go`: `ExtractScripts`
collects every script block because the instance `<script>` and the module
`<script context="module">` both execute as modules and resolve
identically; template markup and `<style>` stay inert; an unparseable block
boundary keeps the whole component inert with no fallback.

Recognized script syntax: side-effect/default/named/namespace `import`,
`export ... from`, literal `import("name")`/`require("name")`; comments,
inert strings/templates/regex skipped; template-literal specifiers stay
the manual kept-dependency boundary; module scripts recognized identically;
scriptless/unparseable components stay inert with no fallback.

Execution: `bazel test //svelte/hello:hello_test` passes (Node + pinned
`svelte/compiler` over runfiles). Env: `svelte/env/plan.bzl`
(`svelte_env_plan` over preserved `JsInfo` + `QualitySourcesInfo`,
deterministic JSON + `DxSubjectInfo`); `//svelte/env:hello_lib_plan`
pinned (`direct=Hello.svelte`, `transitive=Hello.svelte`, no npm closure).
Quality: `svelte` class to the `svelte` family (`quality/adapters.bzl`); no
lint/format/typecheck adapter claims `svelte` yet (classification only, no
supported claim); regions handed to execution-time integrations.

## WP3: Stale, Collisions, Unsupported Syntax, Laziness

Stale cleanup, kind-mismatch, duplicate-claim (`a-b.svelte` vs
`a_b.svelte`), read-error/empty-name failures, unresolved/ambiguous
imports, mapping vs ignore conflict, and malformed directives are
unit-proven (`gazelle/svelte/lang_test.go`) and golden-proven
(`generation_test`). Module-script imports are generation-proven
(`TestGenerateModuleScriptImports`). Unsupported syntax (computed
`import(x)`/`require(x)`, template-literal specifiers, `import.meta`,
method `.require(`, markup/style-region imports) produces no edge and no
notice (parser fixtures). Laziness: empty directories generate nothing and
sweep stale `dx_svelte_library` only; `.svelte`-less targets and non-svelte
kinds are untouched; the adapter adds no test/binary inference.

## Evidence

Exact commands on this host, committed tree:

- `bazel build //...`: success.
- `bazel test //...`: 104/104 pass, including
  `//gazelle/svelte:svelte_test`, `//gazelle/svelte:generation_test`,
  `//svelte/env:env_plan_tests`, `//svelte/hello:hello_test`.
- `bazel coverage //gazelle/svelte:svelte_test`: 781/781 non-test lines
  (lang/naming/parser/stdlib).
- `bazel run //dx:generate_check`: clean.
- Coverage inventory registers `gazelle/svelte` eligible + support paths.

## Changed Components

- `package.json`, `pnpm-lock.yaml` (Svelte pin).
- `svelte/rules/` (`defs.bzl`, `BUILD.bazel`), `svelte/hello/` (fixture +
  test), `svelte/env/` (`plan.bzl`, `plan_tests.bzl`, `BUILD.bazel`).
- `gazelle/svelte/` (adapter, parser, naming, stdlib, tests, golden).
- `quality/adapters.bzl` (`svelte` family classification).
- `tools/coverage/inventory.txt` (svelte paths).
- This report.

## Open Items

- O40 stays open for its owner.
- Non-Linux hosts, remote execution, and clean external-consumer
  evidence are unproven (same gap class as M00/M12/M14/M16/M17/M18).
- No Svelte lint/format/typecheck adapter; no Astro/MDX; no public
  repository/root/exact-target orchestration (M25); no supported claim.
