# M30b Completion Report: Post-Release Adoption (Delivery)

Seed host: local Linux x86_64 glibc (same host class as the M00 seed report).
Local execution only, no remote. Non-Linux platforms were unavailable and are
recorded as gaps, not claimed.

Delivered: `dx init` absent-only scaffolding with real file I/O, hermetic hook
shims with unmanaged refusal, devcontainer admission, `dx status`
diagnostics (never `doctor`), single-version `dx version` with rollback,
local watch planning with re-resolution, thin inspect forwarding over
`bazel query`, and single-source `dx completion` scripts. Ten CLI commands
are registered (`init`, `hooks`, `status`, `version`, `docs`, `watch`,
`owners`, `deps`, `why`, `completion`) with adoption-gated arg validation
and live execution through `dx_adopt`/`dx_docs` plus the query runner.

Capability transitions: adoption surfaces are Dogfooded (used through the
local `dx` binary and unit-tested with real filesystem I/O); no `Supported`
claim (requires required-platform + external-consumer + release evidence).

## WP2: `dx init` Scaffolding (delivered)

`plan_init_files` emits 8 files (`.dx/version`, `dx.local.toml`,
`dx.hooks.toml`, `.devcontainer/devcontainer.json`, `.vscode/settings.json`,
`.vscode/extensions.json`, `.github/workflows/ci.yml` caller pinning the
qualified reusable workflow `@v0.1.0`, `MODULE.bazel.snippet`);
`apply_init` writes absent-only with `refused:` entries for existing paths
even with force. `dx init [--dry-run] [module]` executes it; dry-run lists
without writing.

## WP3: Hook Runner (delivered)

Shims `.git/hooks/pre-commit` + `pre-push` carry `# managed by dx hooks`;
install refreshes managed shims only and refuses unmanaged hooks even with
force; uninstall removes managed shims only. `dx hooks run
<pre-commit|pre-push>` enforces hermetic Git (no ambient fallback) with a
120s per-check budget. `dx hooks status` renders baseline + overlay + timings
from `dx.hooks.toml` + `dx.local.toml`.

## WP4: Devcontainer, Diagnostics, Versioning (delivered)

Devcontainer scaffold pins the base image with Bazel-delegated
`postCreateCommand` and no ambient tools. `dx status [--output text|json]`
reports toolchain/platform/pin checks with `ok/warn/error` vocabulary plus
hints. `dx version` prints `dx`/`rules_dx`/pin; `--pin <ver>` re-pins from
verified release artifacts only (must equal module `0.1.0`); `--check`
fails on drift; rollback re-pins the previous release.

## WP6/WP7: Watch And Inspect (delivered)

`dx watch <cmd> [scope]` validates the wrapped command
(`build/test/run/lint/typecheck/format/check/fix`), refuses `CI=true`
(local-only), re-resolves scope per iteration with 200ms debounce and
`bazel-*`/`.dx`/`dx.local.toml` ignores. `dx owners/deps/why` forward to
`bazel query` (`rdeps` depth-1 / `deps` / `somepath`) with sorted deduped
canonical labels and external-scope rejection.

## Completion Scripts, Guides, Docs Pipeline (delivered)

`dx completion <shell>` renders from the single `Command` table for
`bash|zsh|fish|powershell`; unknown shells fail `unknown-shell`. Guides and
the docs pipeline are the M30a handoff, consumed unchanged.

## Evidence

Exact commands on this host, committed tree:

- `bazel build //...`: success (727 targets).
- `bazel test //...`: 186/186 pass, including `//dx/adopt:dx_adopt_test`
  (21 passed), `//dx/cli:dx_cli_test` (274 passed, incl. 9 adoption tests),
  plus `rustfmt`/`clippy` gates (warnings as errors).
- `bazel run //dx:generate_check`: clean.
- `bazel-bin/dx/cli/dx init --dry-run demo` lists 8 files without writing;
  `hooks status`, `status`, `version`, `completion bash` exercise live paths
  in unit tests with real temp-dir I/O.

Coverage inventory: no new uncovered executable lines beyond the reconciled
gate; `dx_adopt` I/O helpers are covered by co-located unit tests using
real temp dirs (no network). No container builds, no non-Linux runs, no
external-consumer runs; none claimed.

## Changed Components

- `dx/adopt/src/lib.rs` (delivered I/O: `plan_init_files`, `apply_init`,
  hook install/uninstall/status, version pin read/write, status checks +
  text/JSON render, watch/inspect/completion planners; constants
  `DX_VERSION`/`MODULE_VERSION` `0.1.0`, `HOOK_BUDGET_SECS` 120,
  `WATCH_DEBOUNCE_MS` 200).
- `dx/cli/src/adopt.rs` (new: adoption dispatch + 9 tests),
  `src/args.rs` (10 commands, `is_adoption`, `pin`/`serve`/`port`),
  `src/exec.rs` + `src/main.rs` (dispatch + usage), `src/plan.rs`
  (`adoption` capability), `BUILD.bazel` (adopt + docs deps).
- This report.

## Open Items

- O49/O50/O51/O55/O56/O61 exact surfaces are implemented as frozen
  (see `docs/open-decisions.md`); platform evidence beyond Linux x86_64
  and clean-external-consumer first-hour timing remain gaps.
- First-hour budget proof (clean-machine quickstart to green CI within the
  hour, timed and recorded) stays future work, reusing the M30a corpus.
- Milestone exclusions respected: no editor extensions, no remote-dev
  hosting, no quality/generation/environment behavior changes.
