# `dx new` And `dx upgrade`

Implementation status: delivered CLI (`dx new <language> [name]`
absent-only scaffolding plus `dx upgrade --from <version> --to
<version>` pin plus migrate plus setup composition with dry-run and
recovery pointer; live upgrade fails closed until the first manifest
lands).

## `dx new`

`dx new <language> [name]` scaffolds a minimal qualified project for
one language. `language` is one of `rust`, `python`, `javascript`,
`typescript`, `go`, `java`, `kotlin`, `scala`, `csharp`, `fsharp`,
`c`, `cc`, `cpp` (`c`/`cc` alias `cpp`; `c#`/`f#` spellings normalize
to `csharp`/`fsharp`). `name` defaults to `my_project` when absent;
more than two positionals fail pre-exec (exit `2`).

Templates stay stdlib-only so `dx generate` mappings accept them
without ecosystem manifests or locks: one hello source plus the
minimal foreign-layout marker (`Cargo.toml`, `pyproject.toml`,
`package.json`, `go.mod`, `pom.xml`, `build.sbt`, SDK-style project,
or C++ pair), plus the `dx init` repo wiring (including `.dx/version`
and editor configuration) prefixed under `<name>/`.

Writes are absent-only like `dx init`: existing files are left
untouched with a `refused:` note on stderr. There is no `--force`
flag. Unknown languages fail pre-exec (exit `2`) with the supported
list. `--dry-run` lists `would write <name>/...` without writing;
dry-run plans are summaries, suppressed under `--quiet`. `new` is
text-only: `--output json|diff` is rejected at parse time.

`dx new` is not an alias for `dx init`: `init` wires a repository in
place while `new` scaffolds a per-language project under `<name>/`
(including the `init` wiring prefixed). Typo hints reuse the shared
`jaro > 0.7` command suggestion over the accepted spellings, so close
misspellings point at the intended verb without an alias table.

## `dx upgrade`

`dx upgrade --from <version> --to <version>` composes pin plus
migrate plus setup in one invocation with no positional scopes
(repository-wide). Both versions are required Cargo-flavor semver;
the pair must be an upgrade (target exceeds source). Manifest
selection matches [`dx migrate`](migrate.md#manifest-selection)
verbatim (one per major hop, one per full version pair for
minor/patch); the pin target is `--to`, and the setup step is the
atomic `dx setup` selection. Composition only: no new version
semantics beyond the migrate gate.

`--dry-run` plans the composition without touching the tree: text
prints `Would upgrade <from> -> <to> via <manifest> (pin <to>,
migrate, setup)`; JSON emits `command_started` plus an
`upgrade_planned` notice plus `command_finished`. Dry-run plans are
summaries, suppressed under `--quiet`.

No manifests exist yet (module at `0.0.0`, no releases cut), so live
execution fails closed with `upgrade_failed` and no writes. The
diagnostic carries the manual recovery pointer: rerun `dx upgrade
--from <from> --to <to>` (idempotent); to discard a landed pin run
`git checkout -- .dx/version` then rerun setup with `dx setup`. JSON
mode emits `command_started` plus an `upgrade_failed` error plus
`command_finished`. `--output diff`, `--check`, `--fail-on`,
`--report`, `--pin`, and `--` Bazel forwards are rejected; missing
`--from`/`--to`, non-semver, and downgrades/equal versions fail
pre-exec (exit `2`).

Both versions stay explicit by contract: there is no auto pin-detect
one-shot. Reading the installed pin plus discovering the target would
require Git inspection plus network plus manifest discovery, which
stay rejected; the caller names `--from` plus `--to` and the recovery
pointer above makes the retry idempotent.

## Editor coverage

Dispositions follow the [automatic-workflow
policy](../../product/scope.md#automatic-workflows): automatic where
upstream supports it without a custom watcher or resolver engine,
otherwise an explicit snapshot, projection, or manual wiring. The
scaffolded `.vscode/settings.json`, `.vscode/extensions.json`, and
devcontainer extensions cover every row; see
[Ownership And Refresh](../../environments/environment.md#ownership-and-refresh)
for the managed drivers. Scaffolding is VSCode-only by contract:
`.idea/` plus Neovim layouts stay manual and are never generated.
Go plus TypeScript are covered through `.dx` projections: `golang.go`
with `GOPACKAGESDRIVER` plus `node_modules` with the `tsdk` pointer
(TypeScript language support itself ships with VSCode, so no extra
recommendation is needed). This repository checks in no `.vscode/`
directory: it predates the scaffold and consumes the same projections
through `dx setup` plus the devcontainer snapshot instead of a
checked-in settings copy.

| Language | Disposition |
| --- | --- |
| `rust` | automatic: rust-analyzer discovery plus flycheck (Bazel-backed, separate IDE output base) |
| `go` | automatic: upstream `GOPACKAGESDRIVER` (Bazel-backed, pure-Go boundary, cgo out of scope) |
| `c`, `cc`, `cpp` | snapshot: action-derived compile commands via aquery plus managed clangd, manual `dx setup` refresh |
| `python` | projection: `.venv` interpreter plus imports via `dx setup` |
| `javascript` | projection: `node_modules` via `dx setup` |
| `typescript` | projection: `node_modules` plus `tsdk` via `dx setup` |
| `java`, `kotlin` | manual: setup projection plus checked-in native config, no automatic Bazel-backed driver yet |
| `scala` | manual: setup projection plus semanticdb/classpath wiring via `dx setup`, no automatic Bazel-backed driver yet |
| `csharp`, `fsharp` | manual: setup projection plus Paket lock wiring via `dx setup`, no automatic Bazel-backed driver yet |
