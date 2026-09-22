# Adopt Rust example

Foreign Cargo workspace adopted without upstream changes: resolver 2 with
three path crates (`api`, `support`, `worker`) plus a manifest-free `solo`
directory. The tree arrived with no `MODULE.bazel` and no `BUILD` files; the
`crates/*/BUILD.bazel` and `solo/BUILD.bazel` files are generator-owned
(`dx generate` output, freshness enforced by CI).

```sh
bazel run //cli/cli:dx -- init //examples/adopt-rust/...
bazel run //cli/cli:dx -- generate //examples/adopt-rust/...
bazel build //examples/adopt-rust/...
bazel test //examples/adopt-rust/...
```

Evidence: `dx init` writes nothing inside the tree (absent-only). The
workspace root has no `[package]`, so it emits no rules of its own.
Generation links `worker` to `//examples/adopt-rust/crates/api` with no
`use` item in scope (declared path deps mirror Cargo linking), and each `api`
integration test links the sibling `:api` library plus `:api_build_script`.
The workspace `Cargo.toml` plus `Cargo.lock` pin the three path crates while
`solo/` stays manifest-free and resolves source-only; the tree carries no
third-party crates, so there is no wider crate-universe lock scope here.
Depcheck `locks` consistency, quality (`QualitySourcesInfo`), and coverage
(`InstrumentedFilesInfo`) flow through the shared wrappers.
Build covers 14 targets; all 5 tests pass (`api_test`, `api_roundtrip_test`,
`common_test`, `helper_test`, `solo_test`).

Scope notes: cross-package path deps resolve through the local rule index
mirroring Cargo linking, and external imports resolve only through exact
crate mappings today. Regeneration is the composed `dx generate` run.
