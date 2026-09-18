# `dx migrate`

Implementation status: planning library delivered (`dx_adopt::plan_migrate`,
major-release-only gate plus manifest selection); CLI execution fails
closed until the first major-release manifest lands. Owned by
open work.

## Direction

V1 ships `init` plus `generate` plus `migrate`; no `dx new` app or
service templates in v1.

## Syntax

`dx migrate --from <version> --to <version>`: both versions are
Cargo-flavor semver. The pair must be a major-release bump
(target major exceeds source major); minor or patch bumps,
downgrades, and equal versions run through `dx generate`, not
`migrate`. Scope selection reuses generation scope resolution
verbatim; external scopes are rejected like workflow commands.

## Manifest Selection

One manifest per major hop: `migrate-v<from_major>-to-v<to_major>.json`.
Manifests carry generation edit-manifest records (create or modify
with digests and byte-range replacements) applied through the same
write-outcome and completion reporting as `dx generate`.

## Current State

No manifests exist yet (module at `0.0.0`, no releases cut), so
execution fails closed — the same discipline as `audit_deferred`
(`dx update` now executes live with `update_failed` per-set failures).
Planning (`migrate_is_major_bump`,
`migrate_manifest_name`, `plan_migrate`) is pinned by unit tests.
