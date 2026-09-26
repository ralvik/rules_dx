# `dx migrate`

Implementation status: delivered CLI (`dx migrate --from <version>
--to <version>` parsed plus `dx_adopt::plan_migrate` upgrade-only
gate plus manifest selection); live execution fails closed with
`migrate_failed` until the first manifest lands. Owned
under issue #462 (live successor to closed #4), upgrade scope under
issue #671 per ADR 0025.

## Direction

Project scaffolding lives in [`dx new`](new-upgrade.md#dx-new) and the
one-shot composition in [`dx upgrade`](new-upgrade.md#dx-upgrade);
this page owns the migrate rewrite contract.

## Syntax

`dx migrate --from <version> --to <version> [scope...]`: both
versions are required Cargo-flavor semver. The pair must be an
upgrade (target exceeds source under the `semver` crate `to > from`
gate); downgrades and equal
versions plus non-semver fail pre-exec. Prerelease and build
metadata ride the same upgrade gate
(`1.9.9` to `2.0.0-alpha.1` qualifies; `1.0.0-alpha` to `1.0.0`
qualifies; build metadata orders for a total order, so `1.0.0+build.1`
to `1.0.0+build.2` qualifies, pinned by
`migrate_prerelease_and_build_metadata_table`). Scope selection reuses
generation scope resolution verbatim
(empty scope selects `//...`); external scopes are rejected like
workflow commands.

| From | To | Gate | Manifest |
| --- | --- | --- | --- |
| `1.2.3` | `2.0.0` | upgrade | `migrate-v1-to-v2.json` (major hop) |
| `1.2.3` | `1.3.0` | upgrade | `migrate-v1.2.3-to-v1.3.0.json` (full pair) |
| `1.9.9` | `2.0.0-alpha.1` | upgrade (prerelease rides gate) | `migrate-v1-to-v2.json` |
| `1.0.0-alpha` | `1.0.0` | upgrade (release exceeds prerelease) | `migrate-v1.0.0-alpha-to-v1.0.0.json` (full pair, same major) |
| `1.0.0+build.1` | `1.0.0+build.2` | upgrade (build orders) | `migrate-v1.0.0+build.1-to-v1.0.0+build.2.json` |
| `2.0.0` | `1.0.0` | reject (downgrade) | none |
| `1.2.3` | `1.2.3` | reject (equal) | none |

Usage errors (missing `--from`/`--to`, non-semver, downgrade/equal pairs)
exit `2` before any write. `--dry-run` plans without touching the
tree; `--output json` emits `command_started` plus a `migrate_planned`
notice plus `command_finished`. `--output diff`, `--check`,
`--fail-on`, `--report`, and `--` Bazel forwards are rejected;
`--from`/`--to` are rejected on every other command.

| Invocation | Exit | Meaning |
| --- | --- | --- |
| `--dry-run` plan (valid upgrade pair) | `0` | `migrate_planned` notice, no writes |
| live, valid pair, manifest present | `0` | rewrites applied (no manifests exist yet; live delivery owned under #462) |
| live, valid pair, no manifest yet | `1` | `migrate_failed`, no writes |
| missing/invalid `--from`/`--to`, downgrade/equal | `2` | usage error, before any write |

## Manifest Selection

One manifest per major hop (`migrate-v<from_major>-to-v<to_major>.json`)
plus one manifest per full version pair for minor/patch upgrades
(`migrate-v<from>-to-v<to>.json`, e.g.
`migrate-v1.2.3-to-v1.3.0.json` for strict/config rollouts without a
major bump). Multi-major jumps select one manifest
(`1.0.0` to `3.0.0` selects `migrate-v1-to-v3.json`), never a
`v1-to-v2` plus `v2-to-v3` chain: one invocation applies one manifest.
Manifests carry generation edit-manifest
records (create or modify with digests and byte-range replacements)
applied through the same write-outcome and completion reporting as
`dx generate`.

## Current State

No manifests exist yet (module at `0.0.0`, no releases cut), so live
execution fails closed with `migrate_failed` — the same discipline as
`audit_failed` (`dx security`/`dx license` plus `dx update` execute live with
per-family and per-set failures). Syntax plus manifest selection is
pinned by `dx_adopt` unit tests (`migrate_is_upgrade`,
`migrate_is_major_bump`, `migrate_manifest_name`,
`migrate_manifest_name_full`, `plan_migrate`) plus `dx_cli` parse/execution
fixtures and `bazel run //tools/ci:migrate_qualification` (issue
#462, upgrade scope issue #671).

## Bump Cross-Hint

Major dependency widens hint this command : `dx bump`
semver plans print `if major bump, run dx migrate --from <old> --to
<new>` with the missing-manifest mapping (`migrate_failed` exit `1`
live without a manifest; missing `--from`/`--to` exit `2`
`missing-versions`). See
[`dx bump`](audit-update-bazel.md#dx-bump) plus
`cli/bump/tests/fixtures/bump_chain/`.
