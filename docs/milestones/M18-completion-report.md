# M18 Completion Report: Vue

Seed host: local Linux x86_64 glibc (same host class as the M00 seed report).
Local execution only, no remote. Non-Linux platforms were unavailable and are
recorded as gaps, not claimed. No capability-term transitions are claimed:
the integration is adapter-tested first-party implementation under test, not a
product support claim. O29 stays open for its owner; this report supplies the
milestone-specific evidence (upstream stack, supported syntax/targets,
provider mappings, regions, platforms, limitations).

## WP1: Upstream Parser/Compiler, Package, Provider, Target Boundary

Upstream stack (pinned): `aspect_rules_js 3.4.1` ruleset, `vue 3.5.42`
runtime, `@vue/compiler-sfc 3.5.42` SFC parser/compiler, shared M16 managed
Node `22.22.2` toolchain, authoritative single-importer pnpm graph (root
`package.json` + `pnpm-lock.yaml`, `pnpm 10.34.5`, `allowBuilds:{}`,
`hoist=false`). Decision: reuse the M16 graph rather than a separate Vue
hub; rationale: Vue ships as plain npm packages with no lifecycle hooks, so
the existing closure preserves hermeticity with one lock to audit.

`vue/rules/defs.bzl` (`dx_vue_library`): one private `<name>_dx_upstream`
`js_library` plus one public forwarding rule preserving `JsInfo`,
`DefaultInfo`, `InstrumentedFilesInfo` unchanged and adding
`QualitySourcesInfo(direct_sources={"vue": <direct .vue>})`. No
`dx_vue_binary`/`dx_vue_test`: Vue execution/tests reuse the `dx_js_*`
wrappers over parsed outputs (narrowest API, mirrors TS reusing JS
wrappers). `vue/hello` proves the boundary: `Hello.vue` (template/script/
style regions) + `helper.js`, `Hello.test.js` parses with the pinned
`@vue/compiler-sfc` (no regex, no plain-JS fallback, no generation-time
compile) and asserts template+script present, one style block, and the
`./helper.js` script import.

## WP2: Ownership, Generation, Execution, Env Plan, Quality

Physical ownership: each checked-in `.vue` has one physical owner
(`dx_vue_library`); script/template/style regions never become independent
physical sources or core JS/TS targets (`.vue` is inert to them).
`gazelle/vue` (single kind `dx_vue_library`, `.vue` only): narrow block
parser (`ExtractScript` tag scanner honoring comments, quoted attrs,
case-insensitive names; `scan` for imports, never regex), source-only
golden (`testdata/source_only`: `demo.vue` imports `helper.vue` + `fs`
stdlib, `helper.vue` clean), strict resolution (self-deps dropped,
stdlib skipped, handwritten/override/ambiguity/ignore handled, errors
abort before emission), stale sweep (`mergeStale`/`checkClaims`),
single-word `dx_ignore_import` with inheritance and stale-fail-closed.

Recognized script syntax: side-effect/default/named/namespace `import`,
`export ... from`, literal `import("name")`/`require("name")`; comments,
inert strings/templates/regex skipped; template-literal specifiers stay
the manual kept-dependency boundary; `<script setup>` recognized
identically; scriptless/unparseable components stay inert with no fallback.

Execution: `bazel test //vue/hello:hello_test` passes (Node + pinned
compiler-sfc over runfiles). Env: `vue/env/plan.bzl` (`vue_env_plan` over
preserved `JsInfo` + `QualitySourcesInfo`, deterministic JSON +
`DxSubjectInfo`); `//vue/env:hello_lib_plan` pinned
(`direct=Hello.vue`, `transitive=Hello.vue`, no npm closure). Quality:
`vue` class to the `vue` family (`quality/adapters.bzl`); no
lint/format/typecheck adapter claims `vue` yet (classification only, no
supported claim); regions handed to execution-time integrations.

## WP3: Stale, Collisions, Unsupported Syntax, Laziness

Stale cleanup, kind-mismatch, duplicate-claim (`a-b.vue` vs `a_b.vue`),
read-error/empty-name failures, unresolved/ambiguous imports, mapping vs
ignore conflict, and malformed directives are unit-proven
(`gazelle/vue/lang_test.go`) and golden-proven (`generation_test`).
Unsupported syntax (computed `import(x)`/`require(x)`, template-literal
specifiers, `import.meta`, method `.require(`, template/style-region
imports) produces no edge and no notice (parser fixtures). Laziness: empty
directories generate nothing and sweep stale `dx_vue_library` only;
`.vue`-less targets and non-`vue` kinds are untouched; the adapter adds no
test/binary inference.

## Evidence

Exact commands on this host, committed tree:

- `bazel build //...`: success.
- `bazel test //...`: 100/100 pass, including `//gazelle/vue:vue_test`,
  `//gazelle/vue:generation_test`, `//vue/env:env_plan_tests`,
  `//vue/hello:hello_test`.
- `bazel coverage //gazelle/vue:vue_test`: 774/774 non-test lines
  (lang/naming/parser/stdlib).
- `bazel run //dx:generate_check`: clean.
- Coverage inventory registers `gazelle/vue` eligible + support paths.

## Changed Components

- `package.json`, `pnpm-lock.yaml` (Vue pins).
- `vue/rules/` (`defs.bzl`, `BUILD.bazel`), `vue/hello/` (fixture + test),
  `vue/env/` (`plan.bzl`, `plan_tests.bzl`, `BUILD.bazel`).
- `gazelle/vue/` (adapter, parser, naming, stdlib, tests, golden).
- `quality/adapters.bzl` (`vue` family classification).
- `tools/coverage/inventory.txt` (vue paths).
- This report.

## Open Items

- O29 stays open for its owner.
- Non-Linux hosts, remote execution, and clean external-consumer
  evidence are unproven (same gap class as M00/M12/M14/M16/M17).
- No Vue lint/format/typecheck adapter; no Svelte/Astro/MDX; no public
  repository/root/exact-target orchestration (M25); no supported claim.
