# M20 Completion Report: Astro

Seed host: local Linux x86_64 glibc (same host class as the M00 seed report).
Local execution only, no remote. Non-Linux platforms were unavailable and are
recorded as gaps, not claimed. No capability-term transitions are claimed:
the integration is adapter-tested first-party implementation under test, not a
product support claim. O41 stays open for its owner; this report supplies the
milestone-specific evidence (upstream stack, supported syntax/targets,
provider mappings, regions, platforms, limitations).

## WP1: Upstream Parser/Compiler, Package, Provider, Target Boundary

Upstream stack (pinned): `aspect_rules_js 3.4.1` ruleset,
`@astrojs/compiler 4.0.0` (registry latest at implementation time), shared
M16 managed Node `22.22.2` toolchain, authoritative single-importer pnpm
graph (root `package.json` + `pnpm-lock.yaml`, `pnpm 10.34.5`,
`allowBuilds:{}`, `hoist=false`; lockfile delta +8 lines).
Decision: reuse the M16 graph rather than a separate Astro hub, and pin only
the standalone `@astrojs/compiler` instead of the `astro` runtime (63
dependencies avoided); rationale: the compiler package has zero dependencies
so the existing closure preserves hermeticity with one lock to audit, and
region extraction needs no runtime. Narrow difference from Vue/Svelte: the
Astro compiler ships as a Go+WASM native module with a dual entry
(`dist/node/index.js` async `parse`, `dist/node/sync.js` sync
`parse`/`transform`/`convertToTSX`) plus a `dist/node/utils.js` AST toolkit
(`is.*` guards, `walk`/`walkAsync`); `parse` returns `{ast, diagnostics}`
with `ast.type == "root"` and children typed `frontmatter`/`element`/...;
the frontmatter `value` is a plain string.

`astro/rules/defs.bzl` (`dx_astro_library`): one private
`<name>_dx_upstream` `js_library` plus one public forwarding rule
preserving `JsInfo`, `DefaultInfo`, `InstrumentedFilesInfo` unchanged and
adding `QualitySourcesInfo(direct_sources={"astro": <direct .astro>})`.
No `dx_astro_binary`/`dx_astro_test`: Astro execution/tests reuse the
`dx_js_*` wrappers over parsed outputs (narrowest API, mirrors Vue/Svelte).
`astro/hello` proves the boundary: `Hello.astro` (frontmatter importing
`./helper.js`, template div, style, client script) + `helper.js`,
`Hello.test.js` parses with the pinned `@astrojs/compiler/sync` (no regex,
no plain-JS fallback, no generation-time compile) and asserts
`ast.type == "root"`, `diagnostics` empty, frontmatter `value` references
`./helper.js`, and element children exactly `div`/`style`/`script`.
Decision: assert regions via `is.*` guards plus direct `children` mapping
rather than `walk`; rationale: `walk` fires its visitor asynchronously
without returning a promise, so a synchronous fixture cannot observe it.

## WP2: Ownership, Generation, Execution, Env Plan, Quality

Physical ownership: each checked-in `.astro` has one physical owner
(`dx_astro_library`); frontmatter/client-script regions never become
independent physical sources or core JS/TS targets (`.astro` is inert to
them). `gazelle/astro` (single kind `dx_astro_library`, `.astro` only):
frontmatter fence splitter (`splitFence`: file-start fence only, trailing
space/tab/CR tolerated, indented/late fences inert) plus a narrow client
block scanner (`ExtractScripts` honoring comments, quoted attrs,
case-insensitive names; `scan` for imports, never regex), source-only
golden (`testdata/source_only`: `demo.astro` imports `helper.astro` +
`fs` stdlib, `helper.astro` clean), strict resolution (self-deps dropped,
stdlib skipped, handwritten/override/ambiguity/ignore handled, errors
abort before emission), stale sweep (`mergeStale`/`checkClaims`),
single-word `dx_ignore_import` with inheritance and stale-fail-closed.
Per-adapter ownership policy recorded in `gazelle/astro/doc.go` (mirrored
adapters duplicate the policy text deliberately so each adapter reads
standalone).

Narrow difference from Vue/Svelte, documented in `parser.go`: the
frontmatter `---` fence and every client `<script>` both execute as modules
and resolve identically, so both are collected (frontmatter first, then
client scripts in source order); template markup and `<style>` stay inert;
`splitFence` scans `<script>` only in the post-fence remainder so a
`"<script>"` string inside frontmatter cannot forge a region; an unclosed
fence, or an `opens`-but-unparseable script (`opensScript` disambiguates a
leading `<script>` prefix from a merely script-like body), keeps the whole
component inert with no fallback and never partially attributed.

Recognized script syntax: side-effect/default/named/namespace `import`,
`export ... from`, literal `import("name")`/`require("name")`; comments,
inert strings/templates/regex skipped; template-literal specifiers stay
the manual kept-dependency boundary; markup/style-region imports produce no
edge (parser fixtures `markupOnlyImport`, `styleOnlyImport`,
`markupStyleInert`).

Execution: `bazel test //astro/hello:hello_test` passes (Node + pinned
`@astrojs/compiler` over runfiles). Env: `astro/env/plan.bzl`
(`astro_env_plan` over preserved `JsInfo` + `QualitySourcesInfo`,
deterministic JSON + `DxSubjectInfo`); `//astro/env:hello_lib_plan`
pinned (`direct=Hello.astro`, `transitive=Hello.astro`, no npm closure).
Quality: `astro` class to the `astro` family (`quality/adapters.bzl`); no
lint/format/typecheck adapter claims `astro` yet (classification only, no
supported claim); regions handed to execution-time integrations.

## WP3: Stale, Collisions, Unsupported Syntax, Laziness

Stale cleanup, kind-mismatch, duplicate-claim (`a-b.astro` vs
`a_b.astro`), read-error/empty-name failures, unresolved/ambiguous
imports, mapping vs ignore conflict, and malformed directives are
unit-proven (`gazelle/astro/lang_test.go`) and golden-proven
(`generation_test`). Client/server semantics certified:
`TestGenerateClientScriptImports` and `TestGenerateFrontmatterImports`
prove each region independently, and `TestGenerateMixedRegions` proves one
component carrying both a frontmatter import and a client-script import
resolves both edges (`client,server`) onto its single rule while siblings
stay edge-free; stdlib (`fs`) in either region is skipped. Unsupported
syntax (computed `import(x)`/`require(x)`, template-literal specifiers,
`import.meta`, method `.require(`, markup/style-region imports, bare `<`
and trailing-`<` bodies) produces no edge and no notice (parser fixtures).
Laziness: empty directories generate nothing and sweep stale
`dx_astro_library` only; `.astro`-less targets and non-astro kinds are
untouched; the adapter adds no test/binary inference.

## Evidence

Exact commands on this host, committed tree:

- `bazel build //...`: success.
- `bazel test //...`: 108/108 pass, including
  `//gazelle/astro:astro_test`, `//gazelle/astro:generation_test`,
  `//astro/env:env_plan_tests`, `//astro/hello:hello_test`.
- `bazel coverage //gazelle/astro:astro_test`: 859/859 non-test lines
  (lang/naming/parser/stdlib).
- `bazel run //dx:generate_check`: clean.
- Coverage inventory registers `gazelle/astro` eligible + support paths.

## Changed Components

- `package.json`, `pnpm-lock.yaml` (Astro compiler pin).
- `astro/rules/` (`defs.bzl`, `BUILD.bazel`), `astro/hello/` (fixture +
  test), `astro/env/` (`plan.bzl`, `plan_tests.bzl`, `BUILD.bazel`).
- `gazelle/astro/` (adapter, parser, naming, stdlib, tests, golden).
- `quality/adapters.bzl` (`astro` family classification).
- `tools/coverage/inventory.txt` (astro paths).
- This report.

## Open Items

- O41 stays open for its owner.
- Non-Linux hosts, remote execution, and clean external-consumer
  evidence are unproven (same gap class as M00/M12/M14/M16/M17/M18/M19).
- No Astro lint/format/typecheck adapter; no MDX; no public
  repository/root/exact-target orchestration (M25); no supported claim.
