# In-Code Docs Audit (Issue #710)

Accepted. Audited under #710; no behavior changes, comment/docstring-only.

Rule: `docs/AGENTS.md` (enforceable). Non-test `Issue #` must live on a
`See:` or `Owning contract:` line; test pins (`pins.bzl`, `*_tests*`,
`tests/`, `testdata/`, unit tests) may keep bare pins. Crate roots link
the owning `docs/` contract once. `LCOV_EXCL_*` reasons stay short with
policy pointer; full rationale lives in `docs/testing/README.md#coverage`.

Checks:

- `rg "Issue #|issue #" --glob '*.rs' --glob '*.bzl' cli/ quality/ tools/`
  shows only `See:`/`Owning contract:` lines plus test pins.
- Every `cli/*/src/lib.rs`, `cli/*/src/main.rs`, `tools/*/src/main.rs`,
  `docs/ir/ir/src/lib.rs` links its owning `docs/` contract once.

## Per-area list

### `cli/` (Rust)

- Keep: signal-forwarding contract (`cli/cli/src/main.rs`), runner
  teardown ordering, platform-refusal pointers (`platform.rs`, now with
  `See:`), audit/bump/update policy invariants (advisory freshness,
  redaction-by-construction, widen-one plus auto-refresh, selective
  wont-fix), LCOV reasons (thin-shim, re-export-only, defense-in-depth,
  validated-above) as why-not-obvious.
- Shrink: all non-test `Issue #` to `See:` plus issue (e.g. bump
  `issue #638/#639/#640` to `audit-update-bazel.md#dx-bump`; audit
  `issue #628/#629/#632/#676/#679` to `audit-update-bazel.md#dx-audit`;
  migrate `issue #671` to `commands/migrate.md`; `--here` `issue #699`
  to `target-resolution.md`; completion `issue #202` to
  `commands/completion.md`; platform `issue #411-414/#298` to
  `support-matrix.md` with `See:`).
- Delete: none beyond shrinking; `main.rs` header already 3 lines with
  `Contract:`; `validation.rs`/`lifecycle.rs`/`diagnostics.rs`/`modes.rs`
  from proposal no longer exist (refactored).
- Crate roots: every `cli/*/src/lib.rs` links its owning contract once
  (`audit` to `audit-update-bazel.md#dx-audit`, `bump` to `#dx-bump`,
  `update` to `#dx-update`, `ci` to `github-ci.md#check-selection`,
  `docgen` to `documentation/README.md`, `path`/`schema` to
  `architecture/README.md`, `qualification` to
  `product/promotion-checklist.md`, `env` normalized to singular
  `Contract:`; remainder landed earlier).

### `quality/*.bzl`

- Keep: provider field docs, schema-validation docstrings (non-obvious
  invariants).
- Shrink: `parity_tests.bzl` header gains `Contract:`; deferred routes
  keep `issue #416-420` plus `See: ADR 0019`; `curated_defaults.bzl`
  `issue #321` gains `See:`; `native_config.bzl` `issue #589` gains
  `See:`.
- Delete: none; `policy.bzl`/`sources.bzl`/`adapters.bzl` already
  one-liner plus `Contract:`.

### `tools/` (Python/Starlark/shell, Rust shims)

- Keep: `tools/bazelrc/src/lib.rs` module docs (workspace env, minimal headers,
  version pins, regen/verify commands add signal; required by
  `pydoclint`); `depcheck/testdata/**/hello.py` fixture docstrings
  (dep-shape signal); `LCOV` reasons as above.
- Shrink: preset fragment line to
  `# Owned build profiles (issue #177; See: docs/decisions/0021-build-profiles.md).`
  synced across `src/lib.rs`, `preset_fragment.rs`, `preset.bazelrc`,
  `preset_tests.bzl`, `preset_parity_test.sh`; `ci_targets_b/c.bzl`
  `see docs/` to `See: docs/`.
- Delete: none; shell has no trivial docstrings.

## Follow-ups

None required; this PR completes #710. Future delete/shrink PRs must
reference #710 and stay comment-only.
