# Adopt JS/TS example

Foreign JS/TS tree adopted without upstream changes: an `app` package with
same-package local edges (`greet` -> `format`, test -> sibling) and a `web`
TypeScript package proving relative edges survive a Node-stdlib name
collision (`app` -> `:util` even though `util` is a builtin). The tree
arrived with no `MODULE.bazel` and no `BUILD` files; the `*/BUILD.bazel`
files are generator-owned (`//gazelle/javascript:gazelle` and
`//gazelle/typescript:gazelle` output, see below).

```sh
bazel run //cli/cli:dx -- init //examples/adopt-js-ts/...
bazel run //gazelle/javascript:gazelle -- update examples/adopt-js-ts/app examples/adopt-js-ts/app/__tests__
bazel run //gazelle/typescript:gazelle -- update examples/adopt-js-ts/web
bazel build //examples/adopt-js-ts/...
bazel test //examples/adopt-js-ts/...
```

Evidence: `dx init` writes nothing inside the tree (absent-only). Generation
emits one `javascript_library` / `typescript_project` per non-test source
plus one `javascript_test` / `typescript_test` per test, links
same-package relative imports (`:greet` -> `:format`, `:app` -> `:util`),
drops bare stdlib imports with no edges, and classifies only `*_test.js` as
`javascript_test` and only `*_test.ts` as `typescript_test`:
`__tests__/data.js` and `test_helpers.js` stay libraries,
and `types.d.ts`, source maps, and `widget.vue` are inert (no targets). The
runnable edges are user-owned and survive regeneration: `transpiler`,
`tsconfig`, and `declaration` on every `typescript_project` and
`typescript_test` (entries
precedent: cross-project `.js`-suffixed imports need declarations), plus
`config` and ESM `node_options` on each Jest test (hello precedent). Build
covers 41 targets; the 2 tests pass (`greet_test`, `app_test`).

Scope notes: the root `package.json` pins the workspace shape (npm
workspaces, `devDependencies` jest, `optionalDependencies` fsevents) but the
graph here is fully local, so generation needs no lockfile scope. JS/TS are
not wired into `dx generate` yet (that target runs the Rust extension only);
regenerate with the per-language commands above until the dx wiring lands.
TypeScript tests run via `typescript_test` over the tsc-compiled output
(execution reuses the Jest wiring, entries reuse the JS binary wrappers).
