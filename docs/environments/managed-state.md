# Managed Environment State

## Scope And Delivery Order

This document is the authoritative contract for managed PATH, environment, generated-code, and
setup state. [Developer Environments](environment.md), [Generated Code](codegen.md), and concrete
language projections such as [Python Environment](python-environment.md) define what their plans
contain; this document defines how those plans are identified, installed, reused, selected, and
retained.

Delivery follows the repository's dogfood-first milestone order. Managed `.dx/bin` and the
PATH/environment bootstrap come first, Rust supplies the first language-native environment
projection, and Python's `.venv` is a later concrete language projection. Full multi-language env,
codegen, and setup orchestration follows those foundations. This order does not make Python's native
layout the generic environment model.

## Directory Layout

Managed state uses the following logical layout:

```text
.dx/bin/
.dx/environments/<env-hash>/.venv/
.dx/environments/<env-hash>/node_modules/
.dx/environments/<env-hash>/apps/web/node_modules/
.dx/generated/<codegen-hash>/path/to/generated/module.py -> <bazel-output>
.dx/setups/<setup-hash>/environment -> ../../environments/<env-hash>/
.dx/setups/<setup-hash>/generated -> ../../generated/<codegen-hash>/
.dx/setups/current -> <setup-hash>/
.venv -> .dx/setups/current/environment/.venv
node_modules -> .dx/setups/current/environment/node_modules
apps/web/node_modules -> ../../.dx/setups/current/environment/apps/web/node_modules
```

The environment examples are ecosystem-native layouts inside one complete multi-language
generation, not required directories in every generation. Generated generations are
workspace-shaped mirrors rather than top-level language trees. Implementations use correct relative
link text for each facade's host location.

`.dx/setups/current` is the sole mutable pointer for environment and generated-code selection.
Environment facades and language/LSP configuration resolve through its `environment` and
`generated` links. `.dx/bin` is managed independently: bootstrap may install the module-matched `dx`
and configured PATH tools before a setup exists, and changing the current setup does not replace the
PATH-tool tree.

## Identities And Empty Generations

Environment and generated-code generations are immutable and content-addressed by plan identity.
Each generation directory hash is the 64-character lowercase hexadecimal BLAKE3-256 digest of a
versioned, deterministic identity encoded in binary Protobuf management metadata. The exact
identity field schemas are frozen with their implementing milestones (M11 for environment,
M25 for codegen and setup scope) and stay deterministic and versioned; no ad-hoc digest
input is accepted.

An environment identity contains its canonical repository-default or exact-target scope and the
complete selected plan for every represented language integration. Each language contributes its
native semantic inputs; the environment contract does not impose Python, Node, or Rust storage
semantics on another ecosystem.

A generated-code identity contains its canonical scope and target labels, configuration identity,
producer artifacts, and logical path, import-root, and source-root mappings. Generated artifact bytes
remain Bazel-owned and are not copied into the identity.

Each immutable setup record references exactly one environment identity and one generated-code
identity. Its versioned binary Protobuf identity contains those identities rather than duplicating
their artifact lists. The setup hash depends on the two generation identities, and the record
contains managed links to the corresponding generation directories.

Versioned managed empty environment and generated-code generations provide complete setup records
when one side has never been prepared. They are real immutable identities, not absent links or
special mutable directories. First-run `dx env` pairs its environment with the empty generated
generation; first-run `dx codegen` pairs its projection with the empty environment generation. The
independent command does not execute the other workflow. `dx setup` prepares both non-empty sides
when the selected scope provides both capabilities.

## Preparation, Validation, And Reuse

Every refresh performs its required Bazel request before reuse or selection. Bazel remains
authoritative for inputs, action-cache, CAS, output freshness, and artifact bytes. Working
from the already-selected state needs no Bazel request; only the decision to reuse or
rebuild revalidates through the current BEP result. The CLI obtains
plans and locally materialized artifacts through the current BEP result; it does not independently
hash wheels, package stores, generated outputs, or other Bazel artifacts and does not implement a
second remote-cache downloader.

Preparation builds an immutable generation candidate in a staging location and validates every
requested artifact and mapping before selection. A mixed-language environment validates every
represented integration, and combined setup validates both sides, before any user-visible pointer
changes. Build, preparation, path-collision, validation, interruption, or commit failure leaves the
current setup unchanged. Successfully prepared but unselected generations may remain for reuse or
explicit cleanup because they are not user-visible selection state.

Reuse requires an exact match of the complete versioned identity metadata, never the directory name
or ownership marker alone. Environment and generated-code reuse also verifies every owned projection
entry against artifacts reported by the current BEP result. Setup reuse validates its complete pair
identity and both managed generation links; generation validation remains responsible for the
projection trees. Missing, unexpected, malformed, or incorrectly targeted entries force safe
reconstruction or failure before selection.

## Selection And Carry-Forward

Every selected setup is a complete environment/generated pair:

- `dx env` selects its new environment generation with the previously selected generated generation.
- `dx codegen` selects its new generated generation with the previously selected environment.
- `dx setup` selects the two generations resolved for its common root or exact-target scope.

Target-scoped environment composition carries forward selected language plans, including selected
Node importers, that are absent from the target's analyzed closure. A no-argument environment instead
uses every represented integration's release or effective default and all registered Node importers.
Carry-forward duplicates only identity references and symlinks, not dependency artifacts.

Combined setup is capability-aware. For each environment or codegen capability absent from an exact
target, setup carries the currently selected generation for that side while preparing each present
side. If no prior setup exists, an absent side uses its managed empty generation. Setup resolves this
pair before commit and commits both together; it does not execute a separate env or codegen command.

Independent env and codegen commits re-read the current setup under the commit lock and compose their
prepared generation with the newest selected opposite generation. This prevents a concurrently
completed independent command from being lost.

## Commit Lock And Concurrency

Env, codegen, and setup commands do not hold a workspace mutation lock while Bazel analyzes or
executes. They may prepare and validate immutable candidates in parallel. Immediately before
generation and setup installation and current-pointer replacement, a command acquires one workspace
commit lock and re-reads `.dx/setups/current`.

Under the lock, hash-addressed generation and setup installation is idempotent. An existing record at
the expected hash must match byte-for-byte in identity and validate structurally, or the commit fails
without replacement. The command then atomically replaces `.dx/setups/current` and releases the
lock. No command exposes a mixed old/new pair or uses sequential facade replacement with best-effort
rollback.

Lock acquisition waits at most ten seconds, measured with a monotonic clock, then fails with an
actionable busy diagnostic rather than hanging indefinitely. The selected implementation route is
Rust's standard-library [`File::try_lock`](https://doc.rust-lang.org/stable/std/fs/struct.File.html#method.try_lock),
stable since Rust 1.89, rather than an additional locking crate. It reports
`TryLockError::WouldBlock` for contention versus `TryLockError::Error` for other failures; retry
only contention until the deadline and fail other lock errors immediately. The mapping is Unix
`flock(LOCK_EX|LOCK_NB)` and Windows `LockFileEx` exclusive non-blocking, and locks are advisory and
released when all duplicated/inherited handles close. Same-handle relock behavior is unspecified,
append-only opens fail to lock on Windows, and crash release follows handle closure. Use a dedicated
lock file opened for reading and writing without truncation, not the setup record or current pointer.

The OS lock coordinates cooperating commands; it is not a security boundary. Ownership ends when
all handles to the locked file description close, so the lock handle must not be cloned or inherited
by child processes. PID files, stale-lock age checks, and deleting the lock path never authorize a
commit. This selection does not establish cross-platform correctness: O36 still requires the
selected Rust pin, platform semantics, contention, and crash-release evidence in
[Open Decisions](../open-decisions.md).

## Installation And Ownership

Every managed projection uses filesystem symlinks on every host. This includes `.dx/bin` tools,
generation and setup links, generated mirror leaves, `.venv`, and root or importer-local
`node_modules`. There is no launcher, junction, copy, materialization, capability-probing, or fallback
projection mode. Windows requires permission to create symlinks, normally through Developer Mode;
missing capability fails before mutation with actionable setup guidance. Windows CI hosts must
grant this capability; hosts that cannot are recorded as gaps under
[O14](../open-decisions.md) rather than receiving a fallback mode.

Initial installation or repair refuses an unmanaged `.dx/bin`, setup pointer, setup record,
generation directory, `.venv`, `node_modules`, importer facade, or other native facade. A directory
name, digest-shaped path, or unrelated symlink is not proof of ownership. Existing managed records
must pass the identity and structural validation defined above before reuse or idempotent
installation; conflicting paths remain unchanged.

The environment bootstrap owns the complete `.dx/bin` directory, stages its complete replacement,
and atomically swaps the tree so removed tools leave no stale entries. Its
`.dx/bin/.rules_dx_managed` binary Protobuf marker records the metadata schema version and
environment identity and authorizes replacement only when valid and matching. User-managed
executables belong in another PATH directory. The command may print instructions for adding
`.dx/bin` to `PATH`, including the direnv snippet and `direnv allow` reminder defined in
[Direnv Integration](environment.md#direnv-integration), but it never edits shell profiles,
registry settings, or global environment configuration. The committed `.envrc` scaffold is not
managed state: it is written absent-only by `dx init`, never updated by `dx env`, and an existing
unmanaged `.envrc` is refused rather than overwritten.

Managed symlinks refer to current Bazel output artifacts rather than copied runtime closures.
Executable links preserve arguments, working directory, environment, exit status, and signal
behavior. Bazel requests locally materialize every artifact referenced by selected private
environment and codegen output groups before commit while allowing unrelated outputs to remain
remote-only.

## Retention And Recovery

Generations and setup records are retained without automatic pruning. This allows active shells,
editors, and other processes to continue using an older immutable selection and avoids duplicating
Bazel-owned generated bytes. Setup records contain only metadata and links, and generation
directories are link trees into Bazel outputs rather than artifact copies, so retained records
cost metadata plus links while Bazel's output lifecycle bounds artifact bytes. There is no
age policy, count limit, or automatic retention bound; explicit cleanup runs only through
[`dx clean`](../cli/commands/check-fix-clean.md).

A user may explicitly remove an unselected managed generation with `dx clean`
when it is no longer in use. A setup
record may be removed only when it is neither `.dx/setups/current` nor used by an active process; the
current link and its selected target are never cleanup candidates. `dx clean`
refuses unmanaged or digest-spoofed paths and never deletes tracked sources,
BUILD files, Bazel outputs (beyond an explicit `dx clean --bazel` forward),
shell profiles, or global PATH entries.

A user may explicitly remove an unselected managed generation when it is no longer in use. A setup
record may be removed only when it is neither `.dx/setups/current` nor used by an active process; the
current link and its selected target are never cleanup candidates.

`bazel clean`, an output-base change, or removed outputs can leave managed links dangling. Links have
the host's ordinary missing-target behavior and never invoke Bazel to repair themselves. Recovery is
an explicit `dx env`, `dx codegen`, `dx setup`, or bootstrap `bazel run //dx:env`, as applicable. The
approved Rust and Go editor integrations may automatically refresh analysis through Bazel while
their executables remain usable, as defined by
[Developer Environments](environment.md#ownership-and-refresh). This is not dangling-link repair
and does not mutate selected environment or codegen state.

## Test Requirements

Managed-state tests must cover:

- Immutable complete environment, generated-code, empty-counterpart, and setup identities, including
  malformed metadata, path collisions, and refusal of unmanaged state.
- Exact metadata and BEP-backed projection validation for fresh installation and reuse without
  independently hashing Bazel artifact contents.
- One sole atomic `.dx/setups/current` replacement, all-or-nothing combined setup, independent
  carry-forward, capability-absent carry-forward, and first-selection empty counterparts.
- Parallel preparation, newest-opposite-side composition under the commit lock, idempotent
   hash-addressed installation, the ten-second deadline, immediate non-contention errors,
   no child-handle inheritance, process-exit lock release, and crash or
  interruption preservation of the prior pointer.
- Symlink-only behavior on every required host, Windows capability diagnostics, relative link text,
  paths with spaces, and absence of junction, copy, launcher, or automatic fallback projection modes.
- Atomic whole-tree `.dx/bin` replacement, stale tool removal, valid and invalid ownership markers,
  independence from setup selection, and no shell-profile, registry, or global PATH mutation.
- Retention with explicit `dx clean` pruning only, safe explicit removal of unselected records, and stale-link
  recovery only through explicit workflows.

Workflow-specific tests remain in [Developer Environments](environment.md#test-requirements),
[Generated Code](codegen.md#test-requirements), and
[Python Environment](python-environment.md#test-requirements). Cross-cutting fixture, platform,
remote, and evidence requirements remain in [Testing Strategy](../testing/README.md).
