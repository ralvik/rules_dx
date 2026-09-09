# JavaScript And TypeScript Generation Contract

JavaScript and TypeScript follow the [common contract](common.md). See
[ADR 0013](../decisions/0013-rust-javascript-typescript-foundations.md) and
[ADR 0015](../decisions/0015-first-party-gazelle-extensions.md) for rationale and the
[generation test matrix](../testing/generation.md#generate-command-and-gazelle-extensions) for
required evidence.

## Core Sources And Ownership

The core JavaScript extension discovers `.js`, `.jsx`, `.mjs`, and `.cjs`. The core TypeScript
extension discovers `.ts`, `.tsx`, `.mts`, and `.cts`. Module-format semantics come from
authoritative package metadata and stable upstream rules.

Declaration files (`.d.ts`, `.d.mts`, and `.d.cts`), source maps, and `.vue`, `.svelte`, `.astro`,
and `.mdx` containers are inert to the core extensions. Containers are governed only by
[named framework adapters](framework-adapters.md); there is no plain-JavaScript/TypeScript fallback.

Every supported non-test source receives one ordinary reusable one-source target of its language
kind, named from the basename without the final language extension using the common normalizer.
Imports become edges between those targets. Same-package claims across extensions or languages fail
rather than gaining a language suffix.

Executable sources use the common single-library-owner and thin-binary shape. Resources remain
user-owned under the [common resource boundary](common.md#resources).

## pnpm Scope And Resolution

`package.json`, `pnpm-lock.yaml`, and public package/importer metadata from `aspect_rules_js` and
`aspect_rules_ts` are authoritative. Generation produces stable wrapper targets and pnpm importer
bindings, but does not run pnpm, select versions, reconstruct package exports, or implement
TypeScript compilation.

Production targets use their importer-selected production scope. Tests may additionally use the
applicable `devDependencies`. Optional and platform-specific packages must already be active for the
target importer and platform. Lockfile presence alone is insufficient, and imports never activate an
importer or optional dependency.

Ordinary module references and recognized literal `import("name")` and `require("name")` loads use
strict common resolution. Computed loads remain the manual kept-dependency boundary.

## Tests

A supported source is a test only when its basename ends in `_test` immediately before its language
extension. A `test_` prefix, test-directory placement, source syntax, runner configuration, or another
ecosystem naming convention does not create automatic test ownership: such files receive
ordinary non-test target ownership. Broader conventions remain O27 qualification, not
automatic recognition.

Each recognized source receives its own independently runnable `javascript_test` or
`typescript_test`, named from the basename without the final extension. Test-only references attach
only to that target, which depends on the ordinary one-source targets it imports. The initial test
wrappers use `aspect_rules_jest` and Bazel's standard test and coverage protocols; they are not
package-level aggregates.
