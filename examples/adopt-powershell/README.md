# Adopt PowerShell example

Foreign PowerShell tree adopted without upstream changes: a `greet` package
with a module plus entry script and Pester-style tests, and a stdlib-only
`solo` package. The tree arrived with no `MODULE.bazel` and no `BUILD` files;
the `*/BUILD.bazel` files are generator-owned (`dx generate` output) plus
handwritten `pwsh_test` owners.

```sh
bazel run //cli/cli:dx -- init //examples/adopt-powershell/...
bazel run //cli/cli:dx -- generate //examples/adopt-powershell/...
bazel build //examples/adopt-powershell/...
bazel test //examples/adopt-powershell/...
```

Evidence: `dx init` writes nothing inside the tree (absent-only). Generation
emits one `pwsh_library` per directory (named after the directory basename)
with `srcs` as the sorted module sources (`*.psm1` plus at most one
`*.psd1`); `*.Tests.ps1` files stay out of the library and are owned by
handwritten `pwsh_test` targets that survive regeneration. Module imports
arrive via explicit-path `deps` on wrapper libraries (`PwshInfo` `imports`
for `PSModulePath` setup); ambient module discovery is rejected. The `greet`
package carries one pinned Gallery dep (Pester 5.7.1 via
`PSGallery.requirements.psd1` plus `third_party/powershell/PSGallery.lock.json`
with explicit-path import only, no `Install-Module`); `Describe`/`It` plus
`Invoke-Pester` prove build and test over the lock when the closure is
vendored, with a plain-assert fallback so the workspace stays green while
the Gallery fetch stays pending. Depcheck `locks` consistency (manifest vs
lock offline), quality (`QualitySourcesInfo`), and coverage
(`InstrumentedFilesInfo`) flow through the shared wrappers.
Build covers the greet plus solo targets; both tests pass (`greet_test`,
`pure_test`).

Scope notes: cross-package PowerShell imports resolve only through an exact
mapping today; no Gazelle extension is inferred (explicitly scoped
alternative per `docs/generation/powershell.md`). Regeneration is the
composed `dx generate` run.
