# Adopt Go example

Foreign Go module adopted without upstream changes: a `greet` package with
platform-selected sources plus package-level tests (internal `TestMain` and
shared helper coexisting with the external `greet_test` package in one
`go_test` via `embed`), and a stdlib-only `solo` package. The tree arrived
with no `MODULE.bazel` and no `BUILD` files; the `*/BUILD.bazel` files are
generator-owned (`//gazelle/go:gazelle` output, see below).

```sh
bazel run //cli/cli:dx -- init //examples/adopt-go/...
bazel run //gazelle/go:gazelle -- update examples/adopt-go/greet examples/adopt-go/solo
bazel build //examples/adopt-go/...
bazel test //examples/adopt-go/...
```

Evidence: `dx init` writes nothing inside the tree (absent-only). Generation
emits one `go_library` per directory (named after the directory basename)
plus one package-level `go_test` (`<library>_test`) owning every `*_test.go`
via `embed` — never per-file targets. Build constraints are preserved by
including every source (`greet_linux.go` with `//go:build linux` alongside
`greet_other.go` with `//go:build !linux`) and letting the pinned `rules_go`
toolchain select per platform; generation never emits `select()`. The
`importpath` comes from the enclosing `go.mod` (`example.com/adopt-go` plus
the subpath), so the libraries link with their upstream module identity.
The `greet` library carries one pinned module dep (`github.com/google/go-cmp
v0.6.0` via `go.mod` plus `go.sum` plus `@com_github_google_go_cmp//cmp:cmp`
through an exact `# gazelle:resolve` mapping); `Diff`/`Equal` plus
`TestDiffEqual` prove build and test over the lock. Depcheck `locks`
consistency (`go.mod` require matches `go.sum`), quality (`QualitySourcesInfo`),
and coverage (`InstrumentedFilesInfo`) flow through the shared wrappers.
Build covers 6 targets; both tests pass (`greet_test`, `solo_test`).

Scope notes: cross-package Go imports via full module paths resolve only
through an exact `# gazelle:resolve` mapping today. `dx generate` runs the
Rust extension only; regenerate with the per-language command above until
the dx wiring lands.
