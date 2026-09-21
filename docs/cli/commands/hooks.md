# `dx init` And `dx hooks`

Implementation status: implemented.
Delivered: `dx init` absent-only scaffolding plus `dx hooks`
install/uninstall/status/run dispatch with hermetic Git and 120s budgets
(`dx_adopt` gates plus CLI dispatch). Force/unmanaged refusal and two-layer
config are implemented as specified below;
platform evidence beyond Linux x86_64
remains a gap; no working multi-platform support is claimed until qualified
execution lands.

## `dx init`

`dx init` scaffolds a new repository: module and `//dx` target wiring, workspace config,
CI caller template, hermetic hook installation, devcontainer, the single-version `dx`
pin (the `dx` version equals the pinned `rules_dx` module version), the committed direnv `.envrc` defined in
[Direnv Integration](../../environments/environment.md#direnv-integration), and generated
VSCode configuration. The VSCode output is
generated settings only (`.vscode/settings.json` pointing editor
integrations at `.dx/setups/current` projections, checked-in native
configs, and managed `.dx/bin` tools, plus `.vscode/extensions.json`
recommendations covering the core plus admitted foundations; see
[new-upgrade](new-upgrade.md#editor-coverage)); no custom editor
extension is installed. As a narrow exception to
[workspace discovery](../cli-contract.md#workspace-discovery), init may bootstrap a new
repository without an existing `MODULE.bazel`; ordinary commands still require it.
Bootstrap writes are absent-only and do not inspect Git to classify files as tracked
or untracked. Init never contacts the network beyond pinned artifact fetch.

Bootstrap destination mechanics are implemented as specified here: absent-only bootstrap writes,
unmanaged refusal with no overwrite flag. There is no `--force` flag; existing files
or unmanaged hooks are never overwritten.

## `dx hooks`

`dx hooks install` installs thin trigger shims (`pre-commit`, `pre-push`), bootstraps
the gitignored root overlay (`dx.local.toml`) and its gitignore entry through absent-only
file writes, and verifies the shims resolve to the pinned `dx`. It refuses unmanaged
existing hooks rather than replacing them; there is no `--force` flag to override this.
Hook destination/identity mechanics and handling of existing ignore configuration
are implemented as specified here: managed shims only, unmanaged refusal; this is not permission to overwrite existing files.
`dx hooks uninstall` removes only shims it installed. `dx hooks status` prints the
effective merged configuration and last-run timings per check. Native launcher shims
cover hosts where shell hooks do not execute.

`--dry-run` plans without mutating or reading config: `install` prints
`would install …`, `uninstall` prints `would remove …`, `status` prints
`would show hooks status`, `run` prints `would run <trigger>`. Plans are
summaries, suppressed under `--quiet`.

Installation stays manual opt-in: neither `dx init` nor CI installs hooks, and CI
never requires installed shims. `dx init` only scaffolds hook configuration; CI
enforces the committed baseline directly and remains ground truth whether or not
shims exist locally. Forcing hook installation in CI is rejected.

Hook management and staged-file selection are the only exceptions to the common
prohibition on product Git inspection. All product Git operations for hooks, including
installation and staged selection, use hermetic managed Git, never ambient Git or a
PATH fallback. Prefer existing upstream Git rules when they satisfy the workflow.
This does not authorize general Git
status checks or clean-worktree gates, including during init.

The approved bootstrap/hook exception narrows the earlier blanket discovery/Git rules so init can
create a module and hooks can select staged paths without weakening ordinary workflow
safety. Bootstrap APIs are implemented as specified above; no overwrite flag exists.

## Triggers And Default Checks

Hooks run non-mutating verification only and never mutate commits. Scope is the staged
file set (`git diff --cached --name-only`) resolved through affected-target closure over
worktree content; the exact pushed tree is verified by CI, which remains ground truth.
Both triggers are fully configurable in both configuration layers.

| Trigger | Default checks | Rationale |
| --- | --- | --- |
| `pre-commit` | `format --check` plus `lint --check` on the affected closure | Seconds warm; runs dozens of times daily |
| `pre-push` | `typecheck --check` on the affected closure plus `generate --check` | Wider static net at a natural boundary |

Tests stay CI-only by default and are opt-in per repository or per person. Audit never
runs in hooks. No hook requires network access.

## Two-Layer Configuration

The committed typed `hooks` workspace-policy section is the team baseline: trigger
enablement, check lists, scope default, and per-check budget. CI enforces this
baseline and never reads the personal overlay (which is gitignored and cannot reach
CI by construction).

The personal overlay is the gitignored root file `dx.local.toml`: general
local-only overrides with a `[hooks]` table, readable without Bazel analysis so hook
shims can merge it before and around Bazel execution. TOML is used (not JSON)
because it supports comments and matches `ruff.toml`/`Cargo.toml` conventions;
the implementation is planned to reuse TOML parsing with the committed
[`licenses.toml`](audit-update-bazel.md#license-family-dx-audit-license). The overlay uses the same schema as the
baseline and may add checks (for example, enabling affected tests) or locally relax
them. Local relaxation affects only that machine; it cannot weaken the shared gate.
`dx hooks status` always shows the effective merged result, so what runs is never a
mystery. One overlay file absorbs future local-only settings; no per-feature local
files are added.

Choice in either layer covers which hermetic checks run, never how tools resolve:
selection cannot punch a hole in hermeticity.

## Timeout Policy

A check that exceeds its configured budget blocks the commit or push. There is no
warn-and-pass mode. Git's native `--no-verify` remains the escape hatch; bypassed
pushes are still fully verified by CI.
