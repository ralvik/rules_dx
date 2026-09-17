# Adopt Rust example

Foreign Cargo workspace adopted without upstream changes: resolver 2 with
three path crates (`api`, `support`, `worker`) plus a manifest-free `solo`
directory. The tree arrived with no `MODULE.bazel` and no `BUILD` files; the
`crates/*/BUILD.bazel` and `solo/BUILD.bazel` files are generator-owned
(`dx generate` output, freshness enforced by CI).

```sh
bazel run //cli/cli:dx -- generate //examples/adopt-rust/...
bazel build //examples/adopt-rust/...
bazel test //examples/adopt-rust/...
```

Evidence: the workspace root has no `[package]`, so it emits no rules of its
own. Generation links `worker` to `//examples/adopt-rust/crates/api` with no
`use` item in scope (declared path deps mirror Cargo linking), and each `api`
integration test links the sibling `:api` library plus `:api_build_script`.
Build covers 14 targets; all 5 tests pass (`api_test`, `api_roundtrip_test`,
`common_test`, `helper_test`, `solo_test`).
