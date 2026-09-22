# Facade

Consumer facade mapping, Rust library boundary, and upstream authorities.
The [Architecture index](README.md) stays short; this document owns the facade record.

### Facade Twins (Post-Reorg Mapping)

`cli/` holds the Rust implementation (34 crates: 27 under `cli/` plus 5
under `quality/` plus 2 under `generation/`, `dx_*` crate names stable) and
`dx/` is the Starlark-only consumer-policy facade (`//dx:config`, `//dx:env`,
`//dx:codegen`, `//dx:generate`), landed as the facade reorganization under issue #76 with visibility
decisions recorded alongside. Only `//dx:generate` and `//dx:env` are
executable workflows (Gazelle binary, installer binary); `//dx:codegen` and
`//dx:config` are Accepted empty reservations (CLI selection identity,
workspace-flag default) with a qualification guard, not executable targets. The move
fixed the two true collisions (`dx/qual` → `cli/qualification`,
`dx/docs` → `cli/docgen`); the facade labels below are unchanged.
The reorganization is complete with no successor: `dx/` Rust to `cli/`, documentation IR to
`docs/ir`, and the mixed/hello fixture to `examples/mixed` all landed under issue #76
(recorded in ADR 0004); Rust library extraction decided internal-only under
issue #469 (see [ADR 0023](../decisions/0023-rust-libraries-internal.md)).
The e2e suite naming resolved
hermetically under issue #466: no `e2e/` tree, CLI-contract pins run under
`bazel test //...`.

| Facade label | Actual owner | Status |
| --- | --- | --- |
| `//dx:generate` / `//dx:generate_check` | Composed multi-language Gazelle wiring (`//gazelle/dispatch:gazelle` with every first-party extension plus the dispatch witness last, `mode=diff` only on the check twin, `dx/BUILD.bazel`). See [`dx generate`](../cli/commands/generate.md). | Accepted (repo-wide promise in [scope](../product/scope.md) delivered) |
| `//dx:env` | Alias to `//cli/env:env` installer binary (`dx/BUILD.bazel`) | Accepted |
| `//dx:codegen` | Empty filegroup reserving the CLI selection identity; real plan collector is the `dx_codegen_plan_aspect` plus `dx_codegen_plans` output group (frozen), effective roots on the frozen `//...` baseline (closed #506; successors closed #787 and #788) | Accepted (issue #423; guard `//tools/ci:dx_facade_qualification`) |
| `//dx:config` | Empty filegroup default for the `//config:workspace` label flag, failing fast until a consumer binds its typed workspace policy; typed per-family sections frozen in `//quality:policy.bzl` (issue #916) and `//quality:sources.bzl` (issue #916) | Accepted (issue #423; guard `//tools/ci:dx_facade_qualification`) |
| `//tools/coverage:coverage_gate` | Single crate after the unused `:coverage` wrapper removal | Accepted |
| `real_source_target(name="corpus_*")` splits | Per content type per package, Gazelle-owned via `dx generate`, shared `tags = ["corpus"]` (issue #15) | Accepted |

### Rust Library Boundary (Issue #469)

Rust libraries stay internal with a binaries-only public boundary: only
`//cli/cli:dx` and `//cli/env:env` are public tool entry points, and
consumers use those binaries plus the `//dx` Starlark facade, never the
`cli/*`, `quality/*`, or `generation/*` libraries directly. `libs/` is
Starlark-only with no Rust crates. The closed 34-crate inventory, boundary,
and wont-extract rationale live in
[ADR 0023](../decisions/0023-rust-libraries-internal.md); guard
`//tools/ci:rust_library_qualification` pins them.

### Upstream Authorities

Language foundations wrap pinned upstream rulesets for build, test, toolchain, provider, and IDE
semantics. Wrapper macros use bare conventional names (`python_library`, not `dx_py_library`)
loaded from `//<language>/rules:defs.bzl`; the load label is the namespace, per
[ADR 0004](../decisions/0004-naming.md). A file that also needs the same-named upstream symbol
aliases the upstream load (`_cc_library = "cc_library"`), never the wrapper. Ecosystem manifests, lockfiles, package-manager outputs, and public Bazel metadata remain
authoritative for dependency and target policy. Generation translates those facts into BUILD graph
edges but does not resolve versions, install packages, or invent missing ecosystem policy.

Tool acquisition and adapter boundaries are likewise centralized rather than embedded in language
wrappers. See [Tools](../tools/) and the
[tool integration model](../decisions/0007-tool-integration-model.md).

