# Adopt polyglot example

Foreign mixed-language tree adopted without upstream changes: a Python
`pytools` package, a Rust `native` crate, a JavaScript `widgets` package,
and a TypeScript `frontend` package side by side under one root carrying
all three manifests (`Cargo.toml`, `package.json`, `pyproject.toml`). The
tree arrived with no `MODULE.bazel` and no `BUILD` files; the
`*/BUILD.bazel` files are generator-owned (see below).

```sh
bazel run //dx/cli:dx -- init //examples/adopt-polyglot/...
bazel run //gazelle/python:gazelle -- update examples/adopt-polyglot/pytools
bazel run //gazelle/javascript:gazelle -- update examples/adopt-polyglot/widgets
bazel run //gazelle/typescript:gazelle -- update examples/adopt-polyglot/frontend
bazel run //dx/cli:dx -- generate //examples/adopt-polyglot/...
bazel build //examples/adopt-polyglot/...
bazel test //examples/adopt-polyglot/...
```

Evidence: `dx init` writes nothing inside the tree (absent-only). Each
extension claims only its own sources: the Python, JS, and TS BUILD files
hold one rule per own-language source with same-package local edges, and
the single scoped `dx generate` run emits the Rust crate plus `native_test`
without touching the other packages. `dx generate --check` passes scoped,
per-package `bazel build` selects only that package closure, and
regeneration is a no-op in every language (user-owned runtime edges
survive: pytest `# keep` deps, tsc `transpiler`/`tsconfig`/`declaration`,
Jest `config` plus ESM `node_options`). Build covers 25 targets; all 3
runnable tests pass (`shapes_test`, `widgets_test`, `native_test`).

Scope notes: TypeScript has no separate test wrapper yet, so
`totals_test.ts` is a `typescript_project` leaf. Module stems must stay
unique per language across the repo: reusing `adopt-js-ts` stems first
failed closed with an actionable ambiguous-import diagnostic, and the tree
uses distinct `sums`/`totals` stems instead. Python/JS/TS are not wired
into `dx generate` yet (that target runs the Rust extension only); use the
per-language commands above until the dx wiring lands.
