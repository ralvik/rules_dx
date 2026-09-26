# Workflow Notes

CI policy notes split from `ci.yml` header comments.
The [workflow index](../../.github/workflows/README.md) stays short; this document owns the policy-note record.

The following policy notes lived as header comments in `ci.yml`:

```yaml
# Stages: dogfood (four-host consumer self-call), generation
# freshness, and devcontainer parity, each a
# separate job with per-job summaries (issue #210), plus one aggregate
# `ci` job.
#
# Host matrix (issue #415, after portable-shell #323): lean per-stage
# jobs (seed plus arm64 cells in ci.yml; macOS plus Windows
# hosts run via the consumer self-call) plus qualified-host expectation.
# Qualified hosts: Linux x86_64 glibc (ubuntu-latest) plus Linux arm64
# glibc native (ubuntu-24.04-arm, issue #410) plus macOS arm64
# native (macos-14, issue #412) plus Windows x86_64 MSVC-compatible
# native (windows-latest with shell bash, issue #414), local execution, no remote executor.
# Platform qualification for the remaining ADR 0014 host stays open under
# issue #298 (closed; per-host successors own each host): out-of-v1 Windows
# arm64 is the unqualified gap with clean dx refusal, not claimed here.
#
# CI runs deterministic generation freshness through the
# canonical public path before the quality commands converge anything: a
# clean checkout must already be generator-stable (issue #644, successor
# to closed #503; customer check flow only, custom freshness harness rejected).
#
# Dogfood invokes the same entrypoints plus scope as the consumer
# (issue #408): `dx <check> --check //...` verbatim, no corpus-query or
# converge. The repository `//...` scope is the dogfood scope (fixtures
# and testdata ride the same checks as production code; target-coupled
# tsc delegates to the upstream build plus test, ty resolves via staged
# deps). CI installs no
# quality tools: Buildifier, Taplo, Vale, rustfmt, Ruff, Biome, and Clippy
# execute only as Bazel-resolved pinned actions (actions run with an empty
# PATH, unit-pinned in `quality_adapter::exec`). Corpus-local configs stay
# authoritative: aspect runs resolve Vale/Clippy policy solely from the
# `aspect_hints` bound by each corpus target, never from CI-local copies.
# Own-tree runs resolve workspace-level native policy (`//:ruff_config`,
# `//:biome_config`, `//:rustfmt_config`) through direct `aspect_hints`
# on normal targets (issue #12, lane A).
#
# Hygiene (issue #80, deduped under #915): job bootstraps are an inline
# no-secrets `actions/checkout` as step 1 plus the pinned
# `bazel-contrib/setup-bazel` action (commit SHA plus trailing tag
# comment); every third-party action is pinned to a commit SHA with its
# tag in a trailing comment. Docs-publish plus ghcr keep their inline
# checkout (no Bazel there).
# Bazelisk downloads go through setup-bazel's cached launcher
# (`bazelisk-cache: true`, `cache-save` off for pull requests) keyed by
# `.bazelversion`; the Bazelisk version is single-sourced per workflow
# through `BAZELISK_VERSION` (canonical `.devcontainer/Dockerfile.prebuilt`,
# enforced by `//tools/ci:pin_consistency_test`).
# Shared remote cache: BuildBuddy through the workspace `.bazelrc`
# `ci` config (`common --remote_cache=grpcs://remote.buildbuddy.io`),
# passed inline on every Bazel plus dx call as `$BAZEL_CONFIG $BB_ARGS`
# (dx invokes Bazel with `--nohome_rc`, so only the
# workspace-level rc channels survive; `BB_ARGS` carries
# `--remote_header=x-buildbuddy-api-key` only when the
# `BUILDBUDDY_API_KEY` secret exists, and with no secret both vars stay
# empty so no remote-cache flag is set at all), and pull requests add
# `--config=ci-pr` so
# `--noremote_upload_local_results` keeps a poisoned entry from ever
# being published (issue #1059). No `--remote_executor` plus no
# `--bes_backend`:
# execution stays local (pinned by `coverage_qualification`). CI checkouts
# never contain `user.bazelrc`
# (gitignored local-only BCR mirror override), so every run resolves
# the canonical BCR registry plus MODULE.bazel.lock.
#
# Per-cell coverage summaries render in the consumer coverage job
# (issue #1062: one marker-owned PR comment per cell plus a
# `$GITHUB_STEP_SUMMARY` copy, deduped and never overwriting human
# comments; fork pull requests stay step-summary only so fork code never
# gets write credentials). Every cell runs the same
# `dx coverage --min-coverage` gate against its own versioned inventory
# with no cross-cell union, and `tools/coverage/coverage_comment.sh`
# stays the versioned first-party renderer proven by
# `bazel run //tools/ci:coverage_report_guards`.
#
# Sharding policy (issue #210): one logical stage per job for failure
# attribution without log-grep forensics. Seed jobs stay on ubuntu-latest;
# Linux arm64 native jobs (issue #410) stay on ubuntu-24.04-arm;
# macOS arm64 native jobs (issue #412) stay on macos-14; Windows x86_64
# MSVC-compatible native jobs (issue #414) stay on windows-latest with
# shell bash (all standard free-tier runners, no paid services). The
# aggregate `ci` job collects every cell through `needs:` plus
# `if: always()` so one failure still reports the rest. Each job writes a
# `$GITHUB_STEP_SUMMARY` block so triage starts from the summary, not the
# raw log.
#
# Flakiness plus timeout tuning (issue #619): bounded retries plus caps
# live in `.bazelrc` (`test --flaky_test_attempts=3`,
# `test --test_timeout=300`, `test --local_test_jobs=4`) so every
# entrypoint inherits them (retry-until-green stays rejected, per-test
# 300s cap); GitHub `timeout-minutes` stay tuned (build/coverage plus
# freshness 60, devcontainer 15, aggregate 5, no blanket 90);
# long-timeouts-only stays rejected.
```
