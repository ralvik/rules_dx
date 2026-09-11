# Native Configuration

## Authority

Native tool configuration files are the sole behavioral policy for quality tools. They remain
checked-in source files owned by their tools so Bazel actions, editors, language servers, and direct
tool invocations share one reviewable source of truth. Generated artifacts, generator-rule outputs,
and generated config constructors cannot back typed native-config targets or `aspect_hints`;
analysis rejects them before registering quality actions. Generated files may remain ordinary inputs
to other build actions, but they cannot configure quality adapters in v1.

Without an applicable native config, a selected tool uses its pinned upstream built-in behavioral
defaults, except for the explicit Vale requirement below. When a config applies, the pinned tool interprets it with normal native semantics,
including upstream defaults for omitted settings. `rules_dx` defines no hidden rule, style, severity,
or exclusion preset. Adapters add only
required transport and hermetic-execution settings, including isolated declared-config discovery or
an explicit config location, machine-readable output, deterministic path handling, disabled network
and VCS-ignore discovery, and the requested check or fix mode. Those settings must not alter
behavioral policy except where the native tool cannot separate the concerns; an exception requires
tool-specific documentation and conformance tests.

Vale is config-required: when selected for a nonempty applicable source
subset, it requires an explicitly bound, checked-in native config and its declared closure. Missing
config fails with actionable guidance to supply and bind native policy through the normal config
workflow. It must not generate policy, inject hidden styles, search ambient locations, or silently
skip the adapter. Explicit disablement or no applicable source remains the normal lazy no-op;
missing required config is not inapplicability. Vale has no upstream default configuration, as
documented in [initial adapter qualification](tool-integrations.md#initial-adapter-qualification).
Exact filenames, binding mechanics, and failure reporting still require O20 evidence.

Taplo's version-qualified, fail-closed text-diagnostic parser is a permitted tool-specific transport
exception, subject to the [adapter conformance requirements](tool-integrations.md#initial-adapter-qualification).
It does not weaken native policy, error handling, or the structured internal/public result contracts.

## Dogfood-First Delivery

Delivery preserves the repository's dogfood-first sequence without changing the final graph
contract. Initial adapters first consume checked-in native configuration through explicitly declared
typed config inputs. The repository then exercises those adapters through direct Bazel quality
actions before using the same result path through the quality CLI. First-party Gazelle generation
and the public generation workflow subsequently automate the equivalent typed config targets and
bindings. Automation does not introduce a second policy source or change adapter semantics that were
already dogfooded.

## Discovery

The explicit generation workflow is the discovery boundary. Tool-specific Gazelle extensions
recognize supported checked-in native filenames and semantics, generate typed local config targets,
and attach each target's minimal relevant config set through Bazel's standard `aspect_hints`
attribute. These checked-in BUILD bindings are ordinary explicit graph edges. Analysis and actions
never search parent directories, the workspace, the home directory, or conventional filenames for
configuration outside those bindings.

Recognition is conditional on workspace tool selection. Files for a supported but unselected tool
create no package, config target, hint, validation, action, or artifact fetch. Selecting the tool and
rerunning generation activates them; disabling it removes generated metadata through normal Gazelle
semantics without deleting BUILD files.

Within a selected tool, only exact documented built-in filenames are recognized. Unknown, custom,
case-variant, or heuristically similar filenames are silently inert and create no package, target,
hint, or validation. No v1 directive promotes an arbitrary filename to config status. The first-party
Rust Gazelle extension freezes five tool filenames: Buildifier `.buildifier.json`, Taplo `taplo.toml`,
Vale `.vale.ini`, rustfmt `rustfmt.toml`, and Clippy `clippy.toml`. Only Ruff's list
(`ruff.toml`, `.ruff.toml`) is additionally frozen now.

## Targets And Ownership

Every directory containing a recognized native config becomes a Bazel package, including a
config-only directory with no language source. Gazelle creates or updates the local `BUILD.bazel` and
declares the typed config target there. A new boundary may split parent globs, regenerate source
targets, and change labels or ownership in that subtree. This is the generated-graph contract rather
than an ownership shape adapters hide.

Each built-in config rule kind and package-local target has the stable name `<tool>_config`, producing
labels such as `//app:ruff_config`. Gazelle merges an existing same-kind rule normally. A same-name,
different-kind rule fails generation; no numeric, hashed, private-prefix, or filename-derived fallback
is generated.

Generated config targets explicitly use `//<package>:__subpackages__` visibility, or
`//:__subpackages__` at the repository root, independently of package default visibility. The
declaring package and descendants may consume them; siblings and external repositories cannot.
Gazelle owns this attribute under its normal merge and `# keep` behavior. Config targets carry no
special tags, including `manual`, and remain lightweight ordinary members of package and wildcard
selections.

Each `<tool>_config` target directly owns its checked-in config artifact through the same canonical
semantic file-class source facts as ordinary wrappers, in addition to providing typed policy
metadata. Applicable file-class adapters may therefore lint, typecheck, format, or audit it without
a second wrapper. If another target also owns the file, every applicable owner pipeline runs and the
normal stable-candidate consensus requirement applies.

## Binding

Gazelle writes relevant `<tool>_config` labels directly into each applicable source rule's
`aspect_hints`. There is no generated global registry, workspace native-config map, package-level
`quality_config` aggregator, wrapper, or intermediate config provider. Native config labels do not
belong in workspace policy. Repeated direct graph edges are preferable because users do not maintain
them and each source target's inputs remain inspectable.

Adapters combine workspace tool selection, the target's semantic source classes, and the explicit
local binding. A selected adapter creates a stage only for a nonempty supported direct-source subset,
and the stage receives only that subset. For a tool with proven hermetic per-file native discovery,
Gazelle computes the complete set of configs that could apply rather than selecting one config for
the target or reimplementing native precedence. The sandbox preserves workspace-relative paths, and
the tool resolves the effective config for each file in one target/capability pipeline. A tool
without equivalent proven discovery uses a tool-specific explicit contract; there is no generic
closest-file algorithm.

Config binding has no rule-level opt-out. The only quality tags are `no-lint`, `no-typecheck`,
`no-format`, and `no-audit`; each prevents only its corresponding action and does not alter source
ownership or generated config metadata. There is no combined opt-out or positive quality-
classification tag. The full tag and applicability contract is in
[Quality Sources and Applicability](quality-sources.md#tags).

## Merge And Lifecycle

The shared `aspect_hints` attribute follows normal Gazelle list-merge semantics. Surviving entries
and comments retain their positions, list-value `# keep` is honored, stale `rules_dx` entries are
removed, and new `rules_dx` labels append in deterministic canonical-label order. Third-party values
are preserved in relative order and the complete shared list is never globally reordered.

Gazelle manages lifecycle through desired rules and conservative empty stubs, not a separate
`rules_dx` ownership database. It preserves unfamiliar user content and honors `# keep` on rules,
attributes, and list values. A stale generated config rule disappears only when normal empty-rule
merging permits it. Kept stale targets and hints remain subject to ordinary analysis validation and
cannot authorize a missing, generated, malformed, or otherwise invalid config. Gazelle leaves the
existing `BUILD` or `BUILD.bazel` and its package boundary in place even when no rules remain.

Gazelle owns only `rules_dx` entries within `aspect_hints`, including on hand-written source rules.
It preserves all other rule content. A newly added, moved, or removed config does not affect quality
commands until explicit generation updates the declared graph; quality commands neither invoke
Gazelle nor perform a freshness check.

## Closures And Action Inputs

Every primary config and every file reached through include, extend, import, plugin-data, or an
equivalent mechanism must be a checked-in source in the typed config target's complete declared
dependency closure. Missing and undeclared references fail rather than reading ambient files.

Each action receives only the minimal relevant native config set and those configs' complete
closures. Config files outside that local set, Gazelle discovery metadata, and repository-wide
config indexes are not inputs. A target crossing a native-config boundary remains one
target/capability pipeline; each stage still receives only its effective direct-source subset and
the tool performs its native per-file resolution against preserved workspace-relative paths.

Editing a config invalidates every and only pipeline whose selected tool's local set contains it.
Adding, moving, or removing a config changes generated bindings and action keys only in the affected
tool-specific source-ownership subtree; packages outside that subtree retain their graph and action
keys. A config filesystem change made without regeneration cannot alter Bazel behavior because the
declared graph remains unchanged.

For Ruff, Gazelle parses static `extend` paths and generates the complete transitive closure without
a user-authored config-dependency list. Paths resolve relative to the declaring config and remain in
the main repository. Missing files, direct or transitive cycles, absolute paths, external-repository
references, and repository escapes fail generation. An extended config need not directly govern a
source, but it is an input only when a locally relevant closure reaches it.

## Dedicated Configs

When a tool offers a dedicated native file semantically equivalent to a section in a broader project
manifest, `rules_dx` supports only the dedicated form so unrelated project metadata cannot invalidate
quality actions. Ruff therefore recognizes only `ruff.toml` and `.ruff.toml`.
`pyproject.toml` is never a Ruff action input: generation rejects `[tool.ruff]` with migration
guidance and rejects both dedicated filenames in one directory instead of applying native filename
precedence. Tools without an equivalent dedicated format are not subject to this restriction.

## Placement

Because actions receive explicit config labels and never ambient-search, user-authored native
files are colocated near their owners (package-local, matching locality evidence), not
piled at the repository root. Root placement is used only where consumption outside Bazel
forces it: editor and language-server discovery and raw `.dx/bin` invocation follow
upstream discovery rules `rules_dx` does not control. `.dx/` is managed machine state
(binaries, setup pointers, environments), never user-authored configuration. The one
root-level personal file is the gitignored `dx.local.toml` overlay (see the
[hooks contract](../cli/commands/hooks.md)); no per-feature local files are added.

## Exclusions And Isolation

Native configuration is the sole source of file and path exclusions. `rules_dx` has no global ignore
or exclusion list and does not add another user-authored policy layer or alter native precedence.
Bazel scope and direct ownership identify candidate targets. Provider classes, adapter-supported
classes, workspace policy, and capability determine each stage's exact source subset before the tool
applies exclusions from the locally relevant configs. For tools permitting no config, only pinned
upstream built-in exclusions apply in that case apart from mandatory VCS-ignore isolation.
Vale instead follows the config-required failure contract above.

Every linter, formatter, type checker, and source-audit adapter disables Git and other VCS ignore
discovery through a tested tool-specific mechanism. VCS ignore files, global excludes, and repository
state are neither action inputs nor file-selection policy. Ruff forces `--no-respect-gitignore`; a
tool that cannot satisfy this contract is not integrated.

## Config Graph And Mutation

Gazelle attaches all applicable cross-tool config hints to config targets just as it does to other
targets of the same semantic file class. A TOML config can consume Taplo policy and a JavaScript
config can consume selected JavaScript quality policy. The resulting graph must be acyclic. Analysis
rejects direct and transitive cycles across config hints and config dependency closures, reports the
complete label cycle, and registers no quality action.

If a quality action proposes edits to a native config, the CLI uses the normal converged atomic
mutation protocol. It does not invoke Gazelle or rerun Bazel quality actions after applying the
current result. Rejected config files remain unchanged without blocking valid unrelated files. A
later explicit generation or quality command observes the new bytes and normal Bazel invalidation.
