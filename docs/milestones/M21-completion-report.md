# M21 Completion Report: MDX And Mixed Framework

Seed host: local Linux x86_64 glibc (same host class as the M00 seed report).
Local execution only, no remote. Non-Linux platforms were unavailable and are
recorded as gaps, not claimed. No capability-term transitions are claimed:
the integration is adapter-tested first-party implementation under test, not a
product support claim. O42/O43 stay open for their owners; this report supplies
the milestone-specific evidence (upstream stack, supported syntax/targets,
provider mappings, regions, platforms, limitations, mixed matrix).

## WP1: Upstream Parser/Compiler, Package, Provider, Target Boundary

Upstream stack (pinned): `aspect_rules_js 3.4.1` ruleset,
`@mdx-js/mdx 3.1.1` (25 dependencies, no install scripts), shared M16
managed Node `22.22.2` toolchain, authoritative single-importer pnpm graph
(root `package.json` + `pnpm-lock.yaml`, `pnpm 10.34.5`,
`allowBuilds:{}`, `hoist=false`).
Decision: pin the full `@mdx-js/mdx` compiler rather than
remark-mdx + remark-parse + unified separately; rationale: the milestone
assigns the upstream compiler, one pin keeps the single-importer graph
minimal, and the fixture proves region semantics end-to-end via
compiled-program output.

`mdx/rules/defs.bzl` (`dx_mdx_library`): one private
`<name>_dx_upstream` `js_library` plus one public forwarding rule
preserving `JsInfo`, `DefaultInfo`, `InstrumentedFilesInfo` unchanged and
adding `QualitySourcesInfo(direct_sources={"mdx": <direct .mdx>})`.
No `dx_mdx_binary`/`dx_mdx_test`: MDX execution/tests reuse the `dx_js_*`
wrappers over compiled outputs (narrowest API, mirrors Vue/Svelte/Astro).
`mdx/hello` proves the boundary: `Hello.mdx` (column-zero ESM import of
`./helper.js`, prose heading, JSX div, fenced `import fake`) + `helper.js`;
`Hello.test.js` compiles with the pinned compiler (`outputFormat program`)
and asserts the module top-level import set is exactly `./helper.js`, so one
assertion proves prose/fence inertness with no remark-internals coupling.
Narrow difference from Astro: no frontmatter fence exists; the ESM chunk
prefix is the only executable region and everything after the first
non-import chunk opener is inert.

## WP2: Ownership, Generation, Execution, Env Plan, Quality

Physical ownership: each checked-in `.mdx` has one physical owner
(`dx_mdx_library`); prose/fenced/JSX regions never become independent
physical sources or core JS/TS targets (`.mdx` is inert to them).
`gazelle/mdx` (single kind `dx_mdx_library`, `.mdx` only): narrow chunk
parser (column-zero `import`/`export` openers only, ESM single-line subset,
`scan` for imports, never regex), source-only golden
(`testdata/source_only`: `demo.mdx` imports `helper.mdx`, `helper.mdx`
clean), strict resolution (self-deps dropped, stdlib skipped,
handwritten/override/ambiguity/ignore handled, errors abort before
emission), stale sweep (`mergeStale`/`checkClaims`), single-word
`dx_ignore_import` with inheritance and stale-fail-closed. Per-adapter
ownership policy recorded in `gazelle/mdx/doc.go` (mirrored adapters
duplicate the policy text deliberately so each adapter reads standalone).
Decision: keep the chunk parser narrow and fail-closed (unterminated
`<!--` yields no edge, never compiles); the never-taken `inComment`
resumption path was removed as dead, with direct unit fixtures for
`isSetextUnderline`/`isFenceClose`/`scan` block/regex/require/quoted/
template paths; rationale: keeps the markdown-prose-inert contract
reviewable and holds the mandatory 100% gate without behavior change.

Execution: `bazel test //mdx/hello:hello_test` passes (Node + pinned
`@mdx-js/mdx` over runfiles). Env: `mdx/env/plan.bzl` (`mdx_env_plan` over
preserved `JsInfo` + `QualitySourcesInfo`, deterministic JSON +
`DxSubjectInfo`; mirrors `astro/env` reads so focused-target env stays
identical across frameworks and M25 orchestration stays adapter-agnostic);
`//mdx/env:hello_lib_plan` pinned (`direct=Hello.mdx`,
`transitive=Hello.mdx`, no npm closure). Quality: `mdx` class to the `mdx`
family (`quality/adapters.bzl`); no lint/format/typecheck adapter claims
`mdx` yet (classification only, no supported claim); regions handed to
execution-time integrations.

Mixed conformance: `mixed/hello` carries one container per v1 framework
(`Hello.vue`, `Hello.svelte`, `Hello.astro`, `Hello.mdx`) plus the shared
core `helper.js`, each container under its own wrapper (`dx_vue/svelte/
astro/mdx_library`) and the helper under `dx_js_library`; no container is
owned twice, no region becomes a core JS/TS target, no generic fallback
exists. `//mixed/hello:hello_test` (jest) proves the shared edge resolves
(`helper("world") == "hello world"`), every container references
`./helper.js`, and no container imports another container.
`gazelle/mixed` (`mixed.go`, no generator of its own) certifies the
partition mechanically: exact `path.Ext` switch (vue/svelte/astro/mdx/
javascript-variants/typescript-variants, case-sensitive, `.d.ts` owned via
final `.ts`); unknown extensions have no owner; `Owners` drops unowned
files and returns nil for empty/unsupported listings so unused adapters do
no eager work. Per-adapter shared-edge agreement stays covered by the
existing per-adapter parser suites plus the mixed runtime test; the mixed
package owns only the disjoint partition.

## WP3: Stale, Collisions, Unsupported Syntax, Laziness

Stale cleanup, kind-mismatch, duplicate-claim, read-error/empty-name
failures, unresolved/ambiguous imports, mapping vs ignore conflict, and
malformed directives are unit-proven (`gazelle/mdx/lang_test.go`) and
golden-proven (`generation_test`). Unsupported syntax (computed
`import(x)`/`require(x)`, template-literal specifiers, `import.meta`,
method `.require(`, prose/fenced/JSX-region imports, indented/late
openers, unterminated comments) produces no edge and no notice (parser
fixtures). Collision contract shared with all adapters: basename-derived
names preserve ASCII letters/digits/internal underscores, runs of anything
else become one underscore, leading/trailing underscores trimmed, empty
fails; same-package collisions fail with every claimant, never a suffix.
Laziness: empty directories generate nothing and sweep stale
`dx_mdx_library` only; `.mdx`-less targets and non-mdx kinds are untouched;
the adapter adds no test/binary inference. Mixed laziness: `Owners` on an
empty or unsupported-only listing returns nil; case-variant extensions
(`Hello.VUE`, `Hello.MDX`) are unowned, exactly like unknown extensions.

## Evidence

Exact commands on this host, committed tree:

- `bazel build //...`: success (508 targets).
- `bazel test //...`: 114/114 pass, including
  `//gazelle/mdx:mdx_test`, `//gazelle/mdx:generation_test`,
  `//gazelle/mixed:mixed_test`, `//mdx/env:env_plan_tests`,
  `//mdx/hello:hello_test`, `//mixed/hello:hello_test`.
- `bazel coverage //gazelle/mdx:mdx_test //gazelle/mixed:mixed_test`:
  mdx 898/898 non-test lines (lang 240, naming 58, parser 591, stdlib 9);
  mixed 37/37.
- `bazel run //tools/coverage:check` (CI invocation with generated
  sources list): informational line rate 100.00%.
- `bazel run //dx:generate_check`: clean.
- Coverage inventory registers `gazelle/mdx` + `gazelle/mixed` eligible +
  support paths.

## Changed Components

- `package.json`, `pnpm-lock.yaml` (MDX compiler pin).
- `mdx/rules/` (`defs.bzl`, `BUILD.bazel`), `mdx/hello/` (fixture +
  test), `mdx/env/` (`plan.bzl`, `plan_tests.bzl`, `BUILD.bazel`).
- `gazelle/mdx/` (adapter, parser, naming, stdlib, tests, golden).
- `mixed/hello/` (four containers + shared helper + jest edge test).
- `gazelle/mixed/` (partition proof library + tests, no generator).
- `quality/adapters.bzl` (`mdx` family classification).
- `tools/coverage/inventory.txt` (mdx + mixed paths).
- This report.

## Open Items

- O42/O43 stay open for their owners.
- Non-Linux hosts, remote execution, and clean external-consumer
  evidence are unproven (same gap class as M00/M12/M14/M16/M17/M18/M19/M20).
- No Vue/Svelte/Astro/MDX lint/format/typecheck adapter; no public
  repository/root/exact-target orchestration (M25); no supported claim.
