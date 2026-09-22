# CI workflow notes (split from `ci.yml`). No behavior change.

The following policy notes lived as header comments in `ci.yml`:

```yaml
# Sharded CI: build, test, coverage, prove, and dogfood stages run as
# separate jobs with per-job summaries (issue #210).
#
# Host matrix (issue #415, after portable-shell #323): per-host jobs
# parameterized by runner plus cache scope plus qualified-host expectation.
# Qualified hosts: Linux x86_64 glibc (ubuntu-latest) plus Linux arm64
# glibc native (ubuntu-24.04-arm, issue #410) plus Linux static-musl
# profiles (issue #411, cross-built from Linux runners with per-profile
# cache scopes) plus macOS arm64 native (macos-14, issue #412) plus macOS
# x86_64 best-effort native (macos-15-intel, issue #413, non-blocking;
# `macos-13` retired December 2025, `macos-15-intel` until August 2027) plus
# Windows x86_64 MSVC-compatible native (windows-latest with shell bash,
# issue #414), local execution, no remote.
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
# Hygiene (issue #80, deduped under #915): job bootstraps go through the
# single `.github/actions/setup-checkout-bazelisk` composite (no-secrets
# checkout plus pinned Bazelisk via `.github/actions/setup-bazelisk`);
# every third-party action is pinned to a commit SHA with its tag in a
# trailing comment. Docs-publish plus ghcr keep their inline checkout
# (no Bazel there, so no installer).
# Bazel disk-cache restores go through the single
# `.github/actions/restore-bazel-cache` composite action (issue #953),
# which uses actions/cache, free-tier eligible
# for public repositories, per the infrastructure budget in
# docs/testing/README.md. The composite owns the cache-key hashFiles list
# once: every Bazel-affecting
# lock/config (issue #618): MODULE plus bazelrc plus toolchain plus all
# resolver locks, so lock/config changes bust the cache instead of reusing
# a stale disk entry; exact key only with no prefix fallback, so a bust
# starts cold; ci.yml passes only the per-host prefix, so the list cannot
# drift across jobs. Remote cache stays unwired (issue #618 wont-fix):
# paid remote services are not approved per the budget, and Apple/MS
# acquisition plus cache rights stay license-bounded (issue #496), so CI
# stays local execution with no remote cache/executor/BES flags
# (pinned by `coverage_qualification`). CI checkouts
# never contain `user.bazelrc`
# (gitignored local-only BCR mirror override), so every run resolves
# the canonical BCR registry plus MODULE.bazel.lock.
#
# Per-cell coverage summaries render through the single
# `.github/actions/render-coverage-summary` composite (issue #915: cell
# plus stem inputs; no cross-cell union). The seed cell keeps its own
# first-party block (coverage_bin gate plus PR comment).
#
# Sharding policy (issue #210): one logical stage per job for failure
# attribution without log-grep forensics. Seed jobs stay on ubuntu-latest
# with the `bazel-seed-` disk-cache scope; Linux arm64 native jobs
# (issue #410) stay on ubuntu-24.04-arm with the separate `bazel-arm64-`
# scope; static-musl profile jobs (issue #411) cross-build from Linux
# runners with per-profile `bazel-musl-x86_64-` plus `bazel-musl-arm64-`
# scopes; macOS arm64 native jobs (issue #412) stay on macos-14 with the
# separate `bazel-macos-arm64-` scope; macOS x86_64 best-effort native jobs
# (issue #413) stay on macos-15-intel with the separate
# `bazel-macos-x86_64-` scope; Windows x86_64 MSVC-compatible native jobs
# (issue #414) stay on windows-latest with shell bash and the separate
# `bazel-windows-x86_64-` scope (free-tier eligible, no paid services;
# best-effort gaps never block required-host release). `needs:` chains
# enforce
# fast-fail ordering so a broken build skips downstream stages instead of
# burning parallel runners. Each job writes a `$GITHUB_STEP_SUMMARY` block
# so triage starts from the summary, not the raw log.
#
# Flakiness plus timeout tuning (issue #619): every direct `bazel test`
# invocation carries `--flaky_test_attempts=3 --test_timeout=300` (bounded
# retries for transient flakes, per-test 300s cap; retry-until-green stays
# rejected); GitHub `timeout-minutes` stay tuned (seed test/coverage 45,
# per-host test/coverage 60, builds 30/60, no blanket 90); sharding stays
# per-host/per-stage jobs with Bazel intra-job test sharding under ordinary
# semantics (no `strategy.matrix`); long-timeouts-only stays rejected.
```
