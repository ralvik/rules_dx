# ADR 0005: Explicit Command Mutation Semantics

## Status

Accepted.

## Context

Formatting, automatic lint and type-check fixes, dependency updates, repository
generation, generated-source projection, and developer-environment materialization
write files, which differs
from hermetic, cacheable CI checks. Command names and documented defaults make
mutation explicit even when no `--fix` flag is present. Workspace mutation does not
fit ordinary remote Bazel actions.

## Decision

`dx lint` applies automatic lint fixes by default and does not format or type-check.
`dx typecheck` applies genuine type-check fixes by default and does not add
suppression comments. `dx format` formats by default and does not apply lint or
type-check fixes. Their `--check` modes are non-mutating and run checks directly
executable through Bazel. Check mode calculates the same normalized replacements as default
mode, exposes exact proposed edits through machine output, and fails when any change is
proposed without writing it. Bazel actions produce cacheable reports and replacements
without writing the workspace; the Rust CLI validates the complete collected result set,
groups replacements by workspace path, and applies each path as an independent atomic unit.
A stale or conflicting file remains unchanged without blocking valid unrelated files.

Source snapshots use BLAKE3-256 over the exact file bytes. The internal unified result
carries the digest as 32 raw bytes per the
[Quality Result Protocol](../quality/quality-result-protocol.md#file-identity); public JSON
`change` events carry the same digest as 64 lowercase hexadecimal characters per the
[Output Protocol](../cli/output-protocol.md#change). The apply engine hashes current workspace bytes with
the same algorithm immediately before staging that path; a mismatch rejects only that path.

Replacements are normalized byte-range edits against those exact original bytes,
not unified diffs or whole-file artifact references. Each edit has a half-open
`start_byte`/`end_byte` range and inline replacement bytes. Action-local edits are
sorted, non-overlapping, and relative to the original stable-candidate snapshot. Each pipeline
produces one original-to-stable candidate per changed path. The CLI deduplicates identical
candidates across owner contexts and rejects that path for candidate disagreement, digest
disagreement, invalid ranges, stale bytes, or atomic-replace failure. Formatters may represent
their final output as one edit covering the complete original file. Valid paths commit
independently in deterministic path order. Any rejected path makes
the command fail but does not roll back applied paths or block later independent paths.
Interruption may therefore leave a prefix of complete per-file commits, never a partially
written file.

The CLI validates that all selected result artifacts are present, decodable, protocol-valid,
and complete before beginning any file mutation. A collection, execution, or result-envelope
failure that prevents construction of the complete candidate-path set writes nothing.
Per-file partial success begins only after this global validation boundary.

JSON output emits one proposed-change event per path in check and default modes after the same
digest, range, candidate-consensus, and path validation used for mutation. Each event carries the
original BLAKE3-256 digest and exact inline byte-range replacements; a consumer can reconstruct
the candidate without parsing a human diff. Default mode follows changes with terminal
mutation outcomes, while check mode performs no apply step. Direct Bazel result evaluation
fails on any proposed replacement independently of diagnostic severity.

`dx lint`, `dx typecheck`, and `dx format` perform bounded convergence entirely inside their
Bazel pipeline actions. Actions report initial and stable terminal findings and mark an initial
diagnostic fixable only when the converged candidate proves it resolved. There is no unknown
fixability. The CLI derives whether a diagnostic was fixed, remains, or had a fix that was not
applied from that guarantee and the file mutation outcome. After applying validated per-file
replacements, the CLI does not launch another Bazel invocation. The command fails when an
action reports unresolved terminal findings at or above
the configured threshold or the
apply phase fails, and succeeds when all findings were fixed and every candidate path was
applied. A later invocation or `dx typecheck --check` validates
the resulting workspace as a new Bazel request.

`dx update` and `dx generate` are also explicitly mutating commands. Update
delegates to authoritative dependency resolvers and standard manifests/lockfiles;
generate delegates to canonical `//dx:generate`. Neither command
moves dependency or BUILD-file semantics into the CLI.
Update preserves declared version requirements and applies upstream-resolved updates without a
confirmation prompt under the [update contract](../cli/commands/audit-update-bazel.md#dx-update).
The approved update failure policy makes a narrow exception to ordinary fail-fast orchestration:
failure in one selected dependency set does not stop independent selected sets. Skip
operations dependent on the failed set, preserve successful changes without repository-wide
rollback, and fail the invocation overall. This preserves useful independent progress
without claiming dependent work succeeded. Exact aggregate exit code, backend mapping,
and report qualification remain pending under [O12](../open-decisions.md); ordinary
fail-fast behavior, including `dx check` and `dx fix`, is unchanged.

Ordinary CLI commands, including non-mutating commands and `dx bazel`, avoid inspecting Git
or requiring a clean worktree. The only product Git-inspection exception is hook management
and staged-file selection under the [hooks contract](../cli/commands/hooks.md), using
hermetic managed Git for all product hook Git operations, never ambient Git. It does not
authorize general status inspection or clean-worktree gates. Commands operate on current
workspace bytes and Bazel graph state whether files are tracked, modified, staged, or untracked. Source mutations
remain safe through exact digest validation and per-file atomic apply; managed-state mutations
remain safe through ownership validation and atomic pointer replacement. Authoritative
generators and dependency resolvers report their own file conflicts. In particular,
`dx generate` does not preflight ownership markers or override Gazelle's directives and
merge behavior.

The approved bootstrap/hook exception permits init without an existing `MODULE.bazel` and the narrow
hook exception so bootstrap can create the module and hooks can select staged paths.
Bootstrap writes are absent-only, without Git-based tracked-file inspection. Unmanaged
hooks are refused; force cannot overwrite arbitrary existing files or hooks. Bootstrap
destination mechanics and force syntax/managed-replacement behavior remain unqualified
under [O49](../open-decisions.md), not an unrestricted overwrite API.

`dx codegen` is explicitly mutating only in managed `.dx/` state. Bazel actions build
declared generated artifacts without writing source packages; Rust validates their
provider-derived root mappings and atomically selects a read-only generated-source
projection. No-argument codegen invokes canonical `//dx:codegen`; exact-target codegen
uses the selected configured closure. It does not invoke `dx generate` or `dx env`.

`dx setup` is explicitly mutating managed state and composes codegen and environment
preparation without composing their command implementations sequentially. One Bazel
request analyzes the union of required roots with both aspects/output groups and one BEP
stream. The local materializers stage an
immutable setup selection referencing both prepared generations and atomically replaces
the sole `.dx/setups/current` pointer only after both validate. Failure selects neither
new generation. It does not invoke Gazelle.

Every setup selection contains environment and generated references. When no prior
selection exists, first-run `dx env` pairs its environment with a versioned managed
empty generated generation, and first-run `dx codegen` pairs its generated projection
with a versioned managed empty environment generation. This initializes stable paths
without executing the other workflow. `dx setup` prepares both selected sides.

`dx env` is explicitly mutating and delegates to canonical `//dx:env`. Language
rules derive environment contents from Bazel-owned
providers; the CLI does not duplicate dependency selection or package-manager
behavior.

Environment tools are atomically materialized under `.dx/bin`, and stale entries
from the previous generated tree are removed. The complete directory is owned by
`rules_dx` after a valid versioned `.rules_dx_managed` marker is installed. An
unmarked or invalid pre-existing directory is never overwritten. The workflow never
modifies global PATH or shell configuration.

Environment and codegen projections use filesystem symlinks exclusively on every host,
including Windows. There is no launcher, junction, copy, probing, or fallback
projection. Missing Windows symlink capability fails before mutation with setup
guidance. Links refer to the current Bazel output tree, so Bazel cleaning or output-
base changes require refresh rather than copying runtime closures into workspace state.
Consequently, remote-cache or remote-execution requests locally materialize every
artifact referenced by the selected environment and codegen output groups before
commit, but need not download unrelated outputs. Bazel, not `dx`, performs that
materialization.

Bazel remains authoritative for artifact freshness and byte integrity. Every env,
codegen, or setup refresh obtains its current plan and artifacts through Bazel and BEP;
the CLI does not duplicate CAS or action-cache verification by hashing all artifact
contents. Reusing managed generations validates their exact versioned plan metadata and
owned projection links against the current BEP result. Reusing a setup validates its
versioned pair of generation identities and two managed links. A path digest or ownership
marker alone is insufficient.

Concurrent commands may perform Bazel work and prepare immutable candidates in parallel.
They serialize only installation and `.dx/setups/current` replacement under one workspace
commit lock. After acquiring it, an independent env or codegen command re-reads the
current setup and carries forward the newest opposite generation, preventing lost
updates. Combined setup commits its two prepared generations. Hash-addressed installation
is idempotent and conflicting existing records fail without changing the current pointer.
Lock acquisition waits for a fixed short timeout, whose exact value is owned by
[O36](../open-decisions.md), and then reports the workspace as busy.
An OS-released advisory lock provides crash recovery; the CLI does not break locks based
on PID files, timestamps, or deletion of the lock path.

Managed Rust rust-analyzer discovery/flycheck and Go package-driver integration may invoke Bazel
automatically for editor analysis in a separate IDE output base. They do not mutate selected
environment/codegen projections or tracked build/dependency metadata. Exact boundaries and recovery
are defined in [Ownership And Refresh](../environments/environment.md#ownership-and-refresh).
Ordinary environment entries do not acquire self-repair wrappers; a dangling symlink has the host's
ordinary missing-target behavior. Further automation follows the
[automatic-workflow policy](../product/scope.md#automatic-workflows) with explicit domain-contract
updates, not implicit changes to the command mutation semantics above.

Persistent language artifacts are implemented per approved language around its
authoritative rules and providers. No generic materializer invocation is accepted yet.
Each concrete integration stages and validates its native layout inside one immutable
`.dx/environments/<hash>` generation. The hash identifies the complete versioned multi-
language plan, including carried-forward state for absent previously selected integrations. Every
represented integration must prepare successfully before Rust commits all user-visible
language facades by atomically replacing `.dx/setups/current`. A failed
preparation or commit leaves the prior pointer unchanged; unselected prepared
generations may remain as cache state. Each concrete integration must also state and
test its own storage mutation safety before implementation.

Scope comes from Bazel ownership and tools/configuration are pinned action inputs.
Actions may run remotely, but the final apply step is local and does not run tool
analysis independently from Bazel.

## Consequences

- Command documentation and help identify `lint`, `typecheck`, `format`, `update`,
  `generate`, `codegen`, `env`, `setup`, and `fix` as mutating by default
  (`fix` per [ADR 0018](0018-umbrella-check-fix-cleanup-clean.md)).
- CI runs `lint --check`, `typecheck --check`, `format --check`, `generate --check`, direct
  Bazel aspect workflows, or other explicitly non-mutating commands, not write modes.
- Workspace, dirty-tree, symlink, generated-file, and interruption safety require
  dedicated tests.
- Quality source mutation is atomic per file, not per command. Mixed `applied` and
  `not_applied` outcomes are valid, any rejection fails the command, and incomplete result
  collection prevents all writes.
- Mixed-language environment tests prove preparation-before-commit and preservation of
  the shared selection pointer without mixed old/new selections.
- Rust and Go IDE tests prove automatic analysis remains isolated to a separate output base and
  never mutates environment/codegen selection or tracked build/dependency metadata.
- Setup tests prove one Bazel request shares analysis and actions across both plans and
  that either output-group or local-materializer failure prevents selection.
- Convergence tests verify no-change completion, cycle detection, iteration limits, stable
  stage ordering, initial-to-final edits, and that no automatic post-apply Bazel invocation
  occurs.
- Only Bazel-owned files participate; no fallback scans unowned files.

## Rejected Alternatives

- `bazel run` or locally launched tools writing source files directly.
- Materializing a pinned tool and re-running analysis outside Bazel.
- Applying any output after incomplete collection or global result-envelope validation
  failure. Path-local edit or apply rejection still permits unrelated valid paths.
- Automatically rerunning selected tools through a new Bazel invocation after applying source
  fixes to the real workspace. Virtual in-action convergence is required instead.
- Applying unified diffs with context or fuzz semantics.
- Treating differing whole-file outputs as the only cross-linter merge unit.
- Independently committing each successful language facade during one `dx env`
  invocation.
- Sequential facade replacement with a journal or best-effort rollback instead of one
  atomically replaceable selection pointer.
- Making `env` invoke codegen or codegen invoke environment preparation implicitly
  instead of the explicit atomic setup workflow.
