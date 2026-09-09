# `dx init` And `dx hooks`

## `dx init`

`dx init` scaffolds a new repository: module and `//dx` target wiring, workspace config,
CI caller template, hermetic hook installation, devcontainer, the single-version `dx`
pin (the `dx` version equals the pinned `rules_dx` module version per
[O51](../../open-decisions.md)), the committed direnv `.envrc` defined in
[Direnv Integration](../../environments/environment.md#direnv-integration), and generated
VSCode configuration. The VSCode output is
generated settings only (`.vscode/settings.json` pointing rust-analyzer, `gopls`,
Python, and TypeScript integrations at `.dx/setups/current` projections, checked-in
native configs, and managed `.dx/bin` tools, plus `.vscode/extensions.json`
recommendations); no custom editor extension is installed. As a narrow exception to
[workspace discovery](../cli-contract.md#workspace-discovery), init may bootstrap a new
repository without an existing `MODULE.bazel`; ordinary commands still require it.
Bootstrap writes are absent-only and do not inspect Git to classify files as tracked
or untracked. Init never contacts the network beyond pinned artifact fetch.

Bootstrap destination mechanics and any `--force` syntax or managed-replacement behavior
remain pending under [O49](../../open-decisions.md). Unqualified force behavior is blocked:
force cannot authorize overwriting arbitrary existing files or unmanaged hooks.

## `dx hooks`

`dx hooks install` installs thin trigger shims (`pre-commit`, `pre-push`), bootstraps
the gitignored root overlay (`dx.local.toml`) and its gitignore entry through absent-only
file writes, and verifies the shims resolve to the pinned `dx`. It refuses unmanaged
existing hooks rather than replacing them, including when force is requested.
Exact hook destination/identity mechanics and handling of existing ignore configuration
remain pending under O49; this is not permission to overwrite existing files.
`dx hooks uninstall` removes only shims it installed. `dx hooks status` prints the
effective merged configuration and last-run timings per check. Native launcher shims
cover hosts where shell hooks do not execute.

Hook management and staged-file selection are the only exceptions to the common
prohibition on product Git inspection. All product Git operations for hooks, including
installation and staged selection, use hermetic managed Git, never ambient Git or a
PATH fallback. Prefer existing upstream Git rules when they satisfy the workflow;
otherwise qualify a managed fallback under O49. This does not authorize general Git
status checks or clean-worktree gates, including during init.

The approved bootstrap/hook exception narrows the earlier blanket discovery/Git rules so init can
create a module and hooks can select staged paths without weakening ordinary workflow
safety. It does not qualify bootstrap APIs or force behavior under O49.

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
