# Quality Sources and Applicability

## Purpose

`QualitySourcesInfo` is the public source-ownership and classification boundary for custom
rules. It tells quality aspects which directly owned repository source artifacts may be linted,
typechecked, formatted, or audited. It does not select a capability or tool.

This document distinguishes four concepts that must not be conflated:

- A language application ecosystem controls wrappers, compiler/toolchain setup, dependency
  integration, environment projections, and target-coupled adapters.
- A quality policy family owns tool selection for one or more closely related semantic classes.
- A semantic file class identifies syntax and tool applicability, such as `typescript`, `tsx`,
  `json`, or `css`.
- A tool adapter declares which semantic file classes it supports for each capability.

Semantic file classes are neither raw filename extensions nor action boundaries.

## Provider API

The provider concept and source/classification boundary are accepted. The constructor
(`QualitySourcesInfo(direct_sources = {class ID: depset[File]})`), load label
(`//quality:sources.bzl`), field name and representation, registry exports, and validation
placement are frozen in M03 under [O15](../open-decisions.md) and pinned by
`//quality:sources_registry`. The attribute
names in the example below are illustrative placeholders, not frozen upstream attribute
symbols; do not implement rule attributes against them:

```starlark
QualitySourcesInfo(
    direct_sources = {
        "javascript": depset([ctx.file.browser_js]),
        "typescript": depset([ctx.file.server_ts]),
        "json": depset([ctx.file.package_json]),
        "css": depset([ctx.file.stylesheet]),
    },
)
```

In that API, `direct_sources` maps stable semantic file-class IDs to depsets of directly owned repository
source artifacts. A target may expose any number of classes. The map itself supplies source
classification; no positive capability or language tag is required or interpreted.

The provider contains no capabilities, selected tools, native configs, dependencies, generated
context, transitive sources, or execution metadata. Those facts come from workspace policy,
typed config providers, and authoritative language-rule providers.

Project wrappers emit `QualitySourcesInfo`. Custom source-owning rules participate by returning
it directly. Tested internal adapters may derive the same facts from authoritative public
providers of supported upstream rule kinds. Generic aspects never infer custom-rule sources or
classes from rule names, arbitrary attributes, `DefaultInfo.files`, extensions, or filesystem
inspection.

## Semantic File Classes

A semantic file class is the narrowest stable category needed to make tool applicability and
policy unambiguous. Classes are split when two source kinds can have different adapter support,
configuration semantics, or compiler behavior. They are not split merely because two equivalent
filename extensions exist.

For example, JavaScript, JSX, TypeScript, TSX, JSON, JSONC, CSS, SCSS, and Markdown remain
distinct classes because tools such as Prettier, ESLint, Biome, TypeScript, and Stylelint support
different overlapping subsets. Python may remain one class when all relevant adapters treat `.py`
files alike; the paired `.pyi` source kind may receive its own class when provider or adapter behavior
differs. That classification does not give the stub an independent runtime target or dependency
owner.

The ruleset owns one versioned registry of canonical lowercase IDs and each class's admissible
source shapes. Admissibility may use basename, extension, and authoritative provider semantics.
It is not equivalent to comparing the class ID with a file extension. This supports cases such
as extensionless scripts, Starlark `BUILD` files, and ambiguous C-family headers.

Once approved and exported, class IDs are public compatibility surface:

- IDs have one canonical spelling and no implicit aliases.
- Renaming, removing, merging, or semantically narrowing an ID is breaking.
- Adding a class or broadening an existing class requires adapter and policy compatibility tests.
- Human-facing coverage families in the tool baseline are not automatically registry IDs.

The frozen first-release registry contains these IDs:

| Ecosystem or family | Canonical semantic file-class IDs |
| --- | --- |
| Language-independent | `text` |
| C family | `c`, `cpp`, `cuda` |
| .NET | `csharp`, `fsharp`, `powershell` |
| Web styles | `css`, `less`, `scss` |
| JavaScript, TypeScript, and frameworks | `javascript`, `jsx`, `typescript`, `tsx`, `vue`, `svelte`, `astro`, `mdx` |
| Data and markup | `graphql`, `html`, `html_template`, `json`, `json5`, `jsonc`, `markdown`, `toml`, `xml`, `yaml` |
| JVM | `java`, `kotlin`, `scala` |
| Python | `python`, `python_stub` |
| Other source languages | `cue`, `gherkin`, `go`, `jsonnet`, `pkl`, `protobuf`, `qml`, `ruby`, `rust`, `shell`, `sql`, `starlark`, `terraform` |
| Ecosystem manifests | `go_module` |

The framework IDs classify each authoritative physical container source for quality applicability;
they do not predetermine the still-open transport for exact embedded regions. A paired `python_stub`
artifact belongs to its `.py` target's provider, while an orphan `.pyi` emits no provider and remains
inert.

`text` is for directly owned text that has no more specific registered class. It is not a
fallback assigned by generic aspects. A broad adapter such as keep-sorted may explicitly support
`text` and other registered text classes. More specific classes remain preferred because one
path belongs to one class.

The classification principles are accepted. The provider API and initial IDs above are
frozen in M03 under [O15](../open-decisions.md); the class-to-policy-family assignment and
admissibility mappings remain pending the registry review and must not be published as stable
API before that decision.

Every semantic class has exactly one quality policy family. Families keep unrelated source
kinds independently configurable even when one adapter supports both. In particular,
JavaScript policy owns `javascript` and `jsx`, TypeScript policy owns `typescript` and `tsx`,
and JSON policy owns the JSON classes. Selecting Prettier in JavaScript policy does not format
TypeScript, JSON, CSS, Markdown, or any other family. The candidate registry review must assign
every class to one family and may group classes only when they should always share tool
selection. A class is split further when the source context genuinely requires independent
policy; providers do not attach a second ecosystem field to each source.

Quality policy families are independent of application-foundation activation. A custom rule may
expose `typescript` and use a standalone formatter or linter without enabling TypeScript build
rules, Node dependencies, `tsc`, or a TypeScript developer environment. A target-coupled adapter
still requires its authoritative application provider/toolchain context; for example, `tsc`
cannot become applicable from a `typescript` class alone. Enabling quality does not implicitly
activate an application foundation, and activating an application foundation does not bypass normal
class and policy applicability.

Every built-in quality policy family has lazy curated defaults. Users do not maintain a family
enable list: matching direct source classes make the relevant default adapters applicable.
Families appear in workspace configuration only to add or remove supported tools or explicitly
disable a capability. If no selected target exposes an effective class, the family creates no
stage, action, environment contribution caused solely by applicability, or artifact fetch.
The `secrets` policy family is its own semantic class for Gitleaks source audit, with SARIF and secret-value redaction;
qualify the registry amendment, redaction, and findings-versus-error evidence before implementation.

The registry has one source of truth from which exported Starlark constants, provider
validation, adapter metadata, tests, and Rust diagnostic names are generated or checked. The
admissibility table maps each ID to typical basenames/extensions and, where filenames are
ambiguous, required authoritative provider semantics. For example, `.h` alone cannot decide
`c` versus `cpp`, extensionless files do not become `shell` without rule/provider evidence, and
`BUILD`/`BUILD.bazel` are admissible as `starlark` by basename. Adapters consume canonical IDs;
they do not carry independent extension tables.

## Validation

Validation is split per [O15](../open-decisions.md). Provider construction validates
field shape and known IDs: every key is a known canonical semantic file-class ID and every
value is a depset of Bazel `File` source artifacts. Consuming-aspect analysis validates the
rest before registering quality actions:

- Every file is directly owned, belongs to the main workspace, and is not generated or a
  directory.
- Every file is admissible for its declared class according to the central registry and any
  authoritative provider semantics.
- One path appears under exactly one class in one provider.

Class membership is owner-contextual rather than repository-global. If one physical path has
multiple direct Bazel owners, each owner may classify it differently according to authoritative
rule/provider semantics. For example, a shared `.h` file may be `c` for one owner and `cpp` for
another. Every applicable owner pipeline runs. The normal shared-path consensus rule still
requires every mutating owner context to produce identical final bytes before `dx` writes the
file, because one path holds one byte sequence. Disagreement rejects only that path while
valid unrelated paths still apply; the rejected path still fails the command. A user may apply `no-lint`, `no-typecheck`, `no-format`, or `no-audit` to an owner target
that should not participate in that capability.

An empty map is a valid no-op for a configurable rule with no direct quality sources. Empty
depsets are omitted rather than retained as class entries. Deterministic consumers sort class IDs
and normalized paths after flattening depsets; provider declaration order has no semantics.

"Repository source artifact" is a Bazel graph property, not a Git query. Commands operate on
the current workspace bytes and never inspect tracked status. Repository policy and CI determine
whether declared source artifacts are committed.

## Adapter Applicability

For one target, adapter, and capability, analysis computes:

```text
effective classes =
    target provider classes
    intersect adapter-supported classes for the capability
    intersect classes allowed by workspace policy

effective sources =
    union of the target's direct sources for the effective classes
```

Capability support is part of adapter metadata: ESLint may support lint for JavaScript-family
classes, while Prettier may support format for those classes plus JSON, CSS, Markdown, and
others. Curated defaults and user overrides select stable tool IDs through policy-family
configuration. One family's selection
authorizes only the semantic classes assigned to that family; selecting Prettier for
JavaScript does not implicitly enable it for Markdown or CSS. The ruleset expands all ecosystem
selections and curated defaults through adapter manifests into an aggregate
`capability -> semantic class -> ordered adapters` workspace policy. Users do not repeat class
or extension lists.

If several policy-family configs select the same adapter for classes present in one target, analysis
deduplicates the adapter into one target/capability stage over the union of those authorized
effective subsets. Policy families select only stable tool IDs; they cannot attach flags, presets,
versions, executables, runtimes, native-config labels, or separately named adapter instances.
One ID resolves to one ruleset-owned adapter and acquisition identity. Checked-in native tool
configuration is the sole behavioral policy and resolves normally for each effective source.
An adapter with no effective sources creates no stage and causes no tool fetch.

Tool-native include and exclusion behavior runs only within the effective sources already
declared for that stage. Every adapter receives explicit source paths and must disable Git/VCS
ignore discovery. Native configuration cannot add undeclared files or make an adapter applicable
to an unsupported class.

Generated files and transitive sources may be declared as read-only analysis context by a typed
language adapter, for example for type resolution. They never enter `effective sources` and are
never mutation candidates merely because a tool consumes them.

## Pipeline Construction

Under the initial [ADR 0003](../decisions/0003-action-granularity.md) granularity, lint, typecheck, and format use one convergence pipeline
per applicable target and capability. Semantic file classes never subdivide the chosen Bazel
action boundary. Each selected adapter appears at most once in the pipeline and receives the
union of all effective sources authorized by selecting policy-family configs. Future evidence-based target batching may change which targets
share an action, but not this class-subset model.

For a target exposing JavaScript, TypeScript, JSON, and CSS, a format pipeline might contain:

```text
Biome:    JavaScript, TypeScript, JSON sources
Prettier: JavaScript, TypeScript, JSON, CSS sources
```

The runner uses the versioned capability stage order and executes each stage against only that
stage's fixed source subset. Class membership and stage subsets are analysis-time facts and do
not change as virtual bytes change. A complete convergence round runs every nonempty stage;
success requires each stage to leave its own subset unchanged. Common fixed-point requirements
therefore apply naturally to files in overlapping subsets, while disjoint files are not passed
to unrelated tools.

Audit may retain independent target/tool actions because it is non-mutating, but it uses the
same effective-class and effective-source calculation.

The target/capability action key includes only that capability's effective projection: selected
adapter identities, effective classes, exact per-stage source subsets, relevant supported-class
metadata and workspace policy, configs, tools, runner, stage order, and convergence limit. It
does not include provider classes unsupported by every selected adapter for that capability.
Changing such an irrelevant class or an unselected adapter does not invalidate the pipeline.

## Tags

The only target tags interpreted by quality workflows are `no-lint`, `no-typecheck`,
`no-format`, and `no-audit`. Each suppresses its corresponding target capability without changing
source ownership or native-config bindings. There is no combined opt-out tag.

Tags such as `lint-python` or `format-javascript` have no `rules_dx` meaning. They neither add nor
remove classes, select tools, or change applicability; unrelated Bazel tags remain available to
other systems.
