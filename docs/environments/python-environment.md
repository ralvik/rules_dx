# Python Environment

## Purpose

Python development uses a conventional persistent environment while Bazel remains
authoritative for interpreter, dependency, source, generated-code, and build/test
semantics. The environment is an IDE and interactive-development projection of an
analyzed Bazel graph, not a second resolver or execution model.

Python is a later concrete language projection after the PATH/environment bootstrap and Rust
foundation. Its `.venv` layout does not define the language-neutral managed-state model; shared
storage and selection are defined in [Managed Environment State](managed-state.md).

The implementation consumes verified `aspect_rules_py` interpreter, wheel, source,
and import providers. Upstream `py_venv` remains authoritative inside Bazel. The root
environment does not use upstream `py_venv_link` because its v2 contract exposes a
nested virtualenv beneath a linked complete runfiles tree rather than conventional
root paths.

## Conventional Layout

The selected environment exposes:

```text
.venv/bin/python                 # POSIX
.venv/bin/activate
.venv/Scripts/python.exe         # Windows
.venv/Scripts/Activate.ps1
```

Python lives inside the selected language-neutral environment generation. Root `.venv`
is a stable managed directory symlink to the current environment generation and contains no
duplicate environment. The complete [directory layout](managed-state.md#directory-layout) and
[atomic selection contract](managed-state.md#selection-and-carry-forward) are language-neutral.
Existing active shells are not retroactively changed. The command prints the generation path and
selected default or target identity.

Python's contribution to the complete identity includes its integration schema/version,
selected interpreter and ABI mode, dependency hub/group and exact dependency artifacts,
and ordered checked-in import projections. Generated artifacts are excluded because
they belong to the independently selected codegen projection. Checked-in source file
contents are excluded because the environment exposes live links to them.

The complete cross-language identity, metadata validation, reuse, ownership, and retention rules are
defined in [Managed Environment State](managed-state.md). Python contributes the inputs above but
does not independently authorize or select a generation.

## Dependency Materialization

The integration uses the Bazel-selected interpreter and exact resolved wheel
artifacts. It invokes pinned uv without network access or a second dependency
resolution. It does not maintain another dependency list.

Python prepares and validates its contribution before shared selection. A managed environment uses
uv's supported link strategy only when it preserves the shared symlink-only contract. An artifact
that cannot be exposed through links blocks the integration; copying is not a fallback.

## Source And Codegen Visibility

Checked-in provider-derived import roots are represented by deterministic managed
symlinks beneath `.venv/src`. One managed `.pth` file exposes those links in Bazel
import-precedence order, so source edits are visible immediately without rebuilding
the environment.

The managed `.pth` also names provider-derived Python import roots inside the stable
workspace-shaped `.dx/setups/current/generated` mirror after checked-in roots. `dx env` neither
builds nor changes that projection; `dx codegen` owns it. A missing, empty, stale, or
differently scoped codegen projection does not prevent environment selection, but
`dx env` reports the mismatch and corresponding codegen command.

The integration derives all roots from configured target providers. It never scans
the checkout for Python packages or exposes unrelated source trees.

## Default Environment

The no-argument generation's `.venv` represents one repository-wide IDE partition: the
union of analyzed Python target closures using the workspace's default interpreter
version, ABI/free-threaded mode, dependency hub, default dependency group, and explicit
`ide_groups` from typed Python configuration. Bazel repository-root
selection identifies candidate Python targets; analyzed provider/configuration data
determines partition membership and contents. The physical root mechanism is selected
only after `//...`, a query-produced target-pattern file, and aggregate candidates pass equivalent-
semantics performance fixtures.

`ide_groups` names only dependency groups intended for repository-wide test, editor,
and development imports; it does not imply every optional or isolated group. Unknown
groups fail configuration. Targets selecting another partition are reported with the
reason and do not enter the
root union. `dx env <target>` bypasses repository root selection. Both root and exact
commands use only currently declared BUILD graph targets and never require a Gazelle
freshness check.

The root union fails before mutation when closures contribute incompatible versions
of one distribution, conflicting console scripts, or non-mergeable checked-in import
paths. Diagnostics identify every artifact and claimant target.
Valid PEP 420 namespace merges are accepted only when verified provider semantics
identify them as mergeable. No arbitrary traversal-order winner is selected, and the
existing `.venv` selection remains unchanged on failure.

## Exact Target Environments

`dx env //app:server` and `dx env //app:server_test` materialize exact conventional
environments from those targets' configured transitive provider closures. This works
for default, non-default, and isolated graphs without any `python_venv` rule,
`expose_venv` attribute, generated sibling target, or BUILD boilerplate.

Exact-target env uses the selected closure's dependency group or groups and does not
automatically add repository `ide_groups`. This preserves target isolation; users
select a test target when its test-only dependency closure is required.

Target environments create or reuse another complete generation after successful
preparation. Running no-argument `dx env` selects a generation containing the Python
default partition and every other application integration default represented in the repository
graph.

## Ownership And Safety

Python follows the shared [installation and ownership](managed-state.md#installation-and-ownership)
and [commit](managed-state.md#commit-lock-and-concurrency) contracts. It never replaces `.venv`
independently; failure in Python or another represented integration leaves `.venv` on the previous
shared selection.

Ordinary build, test, lint, typecheck, and audit actions never create or mutate
`.venv`. No environment entry automatically invokes Bazel to repair stale links.

## Test Requirements

Python environment tests must cover:

- Conventional POSIX and Windows interpreter and activation paths.
- Exact Bazel-selected interpreter, ABI, dependency group, and wheel artifacts.
- No network access, second resolution, duplicate dependency list, or workspace scan.
- Native wheels and supported uv link strategies on every required host.
- Deterministic `.venv/src` links, one managed `.pth`, immediate source-edit
  visibility, import mappings, precedence, namespaces, and paths with spaces.
- Stable workspace-shaped codegen projection roots through `.pth`, mismatched or
  missing codegen diagnostics, and no implicit generator execution or projection
  selection.
- Default-partition membership, non-default reporting, aliases, filegroups, and
  provider-only Python targets.
- Default group plus configured IDE-group union, unknown-group rejection, test/dev
  import availability, and exact-target exclusion of unrelated IDE groups.
- Distribution, console-script, regular-package, source, and same-path
  conflicts with every claimant reported and no mutation on failure.
- Exact target closure, version, ABI, and dependency-group isolation without sibling
  venv targets.
- Python identity changes for every environment-relevant provider input and live source edits without
  identity churn.
- Default restoration and no retroactive change to active shells.

Language-neutral environment and cross-cutting evidence requirements remain in
[Developer Environments](environment.md), [Managed Environment State](managed-state.md), and
[Testing Strategy](../testing/README.md).
