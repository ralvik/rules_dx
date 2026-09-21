# `dx migrate`

Implementation status: delivered CLI (`dx migrate --from <version>
--to <version>` parsed plus `dx_adopt::plan_migrate` upgrade-only
gate plus manifest selection); live execution fails closed with
`migrate_failed` until the first manifest lands. Owned
under issue #462 (live successor to closed #4), upgrade scope under
issue #671 per [ADR 0025](../../decisions/0025-migrate-upgrade-scope.md).

## Direction

V1 ships `init` plus `generate` plus `migrate`; no `dx new` app or
service templates in v1.

## Syntax

`dx migrate --from <version> --to <version> [scope ...]`: both
versions are required Cargo-flavor semver. The pair must be an
upgrade (target exceeds source semver); downgrades and equal
versions plus non-semver fail pre-exec. Prerelease and build
metadata ride the same upgrade gate
(`1.9.9` to `2.0.0-alpha.1` qualifies; `1.0.0-alpha` to `1.0.0`
qualifies as a minor-channel upgrade). Scope selection reuses
generation scope resolution verbatim
(empty scope selects `//...`); external scopes are rejected like
workflow commands.

Usage errors (missing `--from`/`--to`, non-semver, downgrade/equal pairs)
exit `2` before any write. `--dry-run` plans without touching the
tree; `--output json` emits `command_started` plus a `migrate_planned`
notice plus `command_finished`. `--output diff`, `--check`,
`--fail-on`, `--report`, and `--` Bazel forwards are rejected;
`--from`/`--to` are rejected on every other command.

## Manifest Selection

One manifest per major hop (`migrate-v<from_major>-to-v<to_major>.json`)
plus one manifest per full version pair for minor/patch upgrades
(`migrate-v<from>-to-v<to>.json`, e.g.
`migrate-v1.2.3-to-v1.3.0.json` for strict/config rollouts without a
major bump). Manifests carry generation edit-manifest
records (create or modify with digests and byte-range replacements)
applied through the same write-outcome and completion reporting as
`dx generate`.

## Current State

No manifests exist yet (module at `0.0.0`, no releases cut), so live
execution fails closed with `migrate_failed` — the same discipline as
`audit_failed` (`dx audit` plus `dx update` execute live with
per-family and per-set failures). Syntax plus manifest selection is
pinned by `dx_adopt` unit tests (`migrate_is_upgrade`,
`migrate_is_major_bump`, `migrate_manifest_name`,
`migrate_manifest_name_full`, `plan_migrate`) plus `dx_cli` parse/execution
fixtures and `bazel run //tools/ci:migrate_qualification` (issue
#462, upgrade scope issue #671).
