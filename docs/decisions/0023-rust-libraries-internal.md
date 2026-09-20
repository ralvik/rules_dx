# ADR 0023: Rust Libraries Stay Internal

## Status

Accepted.

## Context

`docs/roadmap.md` carried the bare line `Rust library extraction` with no
owner, while `cli/*` crates plus `quality/runner` plus `libs/` boundaries
stayed unclear under issue #469.

The as-built inventory is 34 internal Rust crates, all `version = "0.0.0"`
(unpublishable): 27 under `cli/` (`dx_adopt`, `dx_apply`, `dx_atomic_fs`,
`dx_audit`, `dx_bep`, `dx_bump`, `dx_ci`, `dx_clean`, `dx_cli`, `dx_codegen`,
`dx_diff`, `dx_digest`, `dx_docs`, `dx_env`, `dx_env_plan`, `dx_fingerprint`,
`dx_lcov`, `dx_output`, `dx_path`, `dx_process`, `dx_proto_validate`,
`dx_qual`, `dx_roots`, `dx_schema`, `dx_setup`, `dx_test_scratch`,
`dx_update`), 5 under `quality/` (`quality_adapter`, `quality_evaluator`,
`quality_markdown`, `quality_result`, `quality_runner`), and 2 under
`generation/` (`generation_result`, `codegen_shard`). Crate names stay
`dx_*` stable per [ADR 0004](./0004-naming.md). `libs/` holds Starlark-only
helpers (`libs/starlark`, `libs/testing`) with no Rust crates, so it is out
of scope for extraction.

The public boundary is binaries plus Starlark, never Rust libraries: only
`//cli/cli:dx` and `//cli/env:env` carry explicit public visibility (see
`docs/contributing/build-conventions.md#visibility` and
`//tools/ci:visibility_guards`); every `cli/*` library is scoped to `//cli`
plus narrow named consumers, and `quality/*` plus `generation/*` libraries
stay scoped to their owners. The module remains one Bzlmod release unit
(see [ADR 0004](./0004-naming.md)), and the roadmap carries no publication
pressure. No external Rust consumer exists.

## Decision

Rust libraries stay internal; there is no extraction, no `crates.io`
publication, and no separate repository. The 34-crate inventory above is
the closed extraction candidate list with a wont-extract disposition. The
public Rust boundary stays the two tool entry points; consumers migrate
nowhere because they never depend on the libraries directly — they use the
`dx`/`env` binaries and the `//dx` Starlark facade.

Extraction reopens only with a concrete external caller and an accepted
successor record, per the architecture rule that shared code is extracted
only with a concrete cross-package caller (see
[Architecture](../architecture/README.md#ownership-boundaries)).

## Consequences

- All Rust `Cargo.toml` versions stay `0.0.0`; no publish metadata is added.
- `cli/*`, `quality/*`, and `generation/*` library visibilities stay
  scoped; only the two binaries keep explicit public visibility.
- The roadmap entry resolves to this decided record under issue #469.
- Guard `//tools/ci:rust_library_qualification` pins the inventory,
  the binaries-only boundary, the `libs/` Starlark-only fact, and the
  roadmap plus architecture links.

## Rejected Alternatives

- Publish reusable subsets (`dx_digest`, `dx_atomic_fs`, `dx_diff`,
  `dx_process`, `dx_output`, `dx_path`, and similar): rejected, no external
  caller exists and versioning plus release overhead contradicts the single
  release unit with no publication pressure.
- Split repositories per crate family: rejected, same absence of caller plus
  breaks the dogfood path that validates the quality core on this tree.
- Silent monolith (leave the roadmap line bare): rejected per issue #469,
  the roadmap promises an explicit outcome.
