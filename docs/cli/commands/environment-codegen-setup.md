# Environment, Codegen, And Setup Commands

## `dx codegen`

With no argument, `dx codegen` invokes canonical `//dx:codegen` and atomically selects
the repository-wide generated-source projection, including registered production,
test, example, and development generators. The workflow uses the benchmark-selected
Bazel root mechanism documented in [Generated Code](../../environments/codegen.md).

`dx codegen <target>` accepts exactly one explicit target label and selects generated
sources from that configured target's transitive provider closure. It does not require
a sibling projection target for a consumer. A registered bare schema target selects
all currently declared generated-language projection reverse dependents found through
Bazel query. Paths, patterns, multiple labels, profiles, and language selectors are
rejected. A target with no generated sources selects an empty exact projection rather
than retaining unrelated generated imports.

`dx codegen` neither invokes `dx generate` nor prepares language environments. It may
warn when the selected environment has a different scope. See
[Generated Code](../../environments/codegen.md) for providers, projections, root selection, and
performance.

## `dx env`

With no argument, `dx env` invokes canonical `//dx:env`, refreshes configured tools,
and selects repository defaults for every application integration represented by authoritative
repository manifests and targets. Before `dx` exists, users
bootstrap this mode with `bazel run //dx:env`.

`dx env <target>` accepts one explicit label and selects exact native projections for
every supported application integration represented in that target's analyzed transitive closure.
Absent languages retain their current selection. No sibling environment target or
wrapper attribute is required. Paths, patterns, multiple labels, profiles, and
language selectors are rejected; neither mode launches a shell.

`dx env` does not build or select generated code. It may warn when the current codegen
projection has a different scope and identify `codegen` plus the required scope kind as
the matching workflow, without rendering command-line arguments.

In NDJSON mode this warning is a `notice` with code `selection_scope_mismatch`; it does not
affect diagnostics, `--fail-on`, or exit status.

See [Developer Environments](../../environments/environment.md) and
[Python Environment](../../environments/python-environment.md) for the complete environment
contracts.

## `dx setup`

`dx setup` prepares repository-wide codegen and environment generations, then commits
both through one atomic `.dx/setups/current` replacement. `dx setup <target>` does the same for
one explicit target label. It accepts the same label-only restrictions as `dx env` and
`dx codegen`: paths, patterns, multiple labels, profiles, and language selectors are
rejected.

Preparation is all-or-nothing at the selection boundary. If either workflow fails to
build, materialize, or validate, the existing setup selection remains active. Prepared
unselected cache generations may remain. `dx setup` does not invoke Gazelle or update
tracked BUILD metadata; users run `dx generate` separately after structural changes.

Setup issues one Bazel build with the union of required roots, environment and codegen
aspects, both requested output groups, and one BEP stream. It does not launch separate
env/codegen Bazel commands or use separate output bases. Bazel deduplicates common
configured closures and actions; provider applicability keeps each aspect bounded to
targets with nonempty effective stage subsets.

For an exact target with only one capability, setup prepares that side and carries the
other current generation forward. A bare schema therefore builds all registered codegen
projections without replacing the selected environment. A missing prior side uses its
managed empty generation.

Before `dx` is installed, `bazel run //dx:env` remains the minimal bootstrap that
exposes the matching CLI and configured tools. The normal first complete developer
initialization is then `dx setup`.

If no setup selection exists, running only `dx env` or `dx codegen` creates a managed
empty generation for the unrequested side. The command does not execute that side, and
`.dx/setups/current/environment` and `.dx/setups/current/generated` remain valid paths.
