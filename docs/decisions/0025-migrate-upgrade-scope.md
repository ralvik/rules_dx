# ADR 0025: `dx migrate` Upgrade Scope

## Status

Accepted.

## Context

`dx migrate` shipped major-release-only (`migrate_is_major_bump`
requires `to.major > from.major`; minor/patch pairs ran through
`dx generate`) under issue #462 (see
[dx migrate](../cli/commands/migrate.md)). Issue #671 needs a
supported channel for wanted breaking-ish rollouts that do not merit
a major bump, e.g. strict-quality native-config packs (strict
`ruff.toml` / `biome.json` / `vale` styles / `clippy.toml` guidance)
and similar policy refreshes. Hand-copying files or cutting a major
for every strict refresh does not fit iterative hardening, and
`dx generate` is the wrong channel (declared-graph sync, not reviewed
breaking rewrites).

## Decision

`dx migrate --from <version> --to <version>` accepts any upgrading
pair (`to > from` Cargo-flavor semver via `dx_adopt::migrate_is_upgrade`;
see [dx migrate](../cli/commands/migrate.md)):

- Major bumps keep the per-major-hop manifest
  (`migrate-v<from_major>-to-v<to_major>.json` via
  `dx_adopt::migrate_manifest_name`); `migrate_is_major_bump` stays as
  the coarse subset predicate for that channel.
- Minor/patch (and prerelease-to-release) upgrades select one manifest
  per full version pair (`migrate-v<from>-to-v<to>.json` via
  `dx_adopt::migrate_manifest_name_full`, e.g.
  `migrate-v1.2.3-to-v1.3.0.json`).
- Downgrades, equal versions, and non-semver stay rejected
  (`migrate is upgrade-only`, exit `2`).
- Existing guarantees hold: `--dry-run` plans without writes, live
  execution fails closed with `migrate_failed` until a manifest lands,
  same generation edit-manifest write-outcome (create/modify with
  digests plus byte-range replacements, opt-in per repo, manual merge
  on user edits).

## Consequences

- `dx_adopt::plan_migrate` gates on `migrate_is_upgrade`; the
  `migrate is major-release-only` diagnostic becomes
  `migrate is upgrade-only`.
- `bazel run //tools/ci:migrate_qualification` pins the dual manifest
  selection plus the upgrade gate.
- Strict content itself lands in later manifests; no change to curated
  defaults or the adapter registry in this record.

## Rejected Alternatives

- Keep major-only migrate and ship strict via `dx init --strict`
  stamp only: rejected, no update path for existing repos.
- Ship strict via `dx generate`: rejected, wrong channel
  (non-breaking sync versus reviewed breaking rewrites).
- Cut a major for every strict refresh: rejected, too heavy for
  iterative hardening.
- Bazel-only strict flag: rejected, splits IDE truth (editors /
  `.dx/bin` follow upstream discovery from checked-in native files).
