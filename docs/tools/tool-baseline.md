# First-Release Tool Baseline

## Source

The frozen `aspect_rules_lint` v2.8.0 tool list is the minimum first-release quality baseline,
not a scope ceiling. Curated quality-tool expansion across languages is mandatory where upstream
implementations or Bazel rules permit hermetic thin integration under
[First-Release Admission](../product/scope.md#first-release-admission) and O46 in the
[open-decision register](../open-decisions.md). This does not authorize building replacement
language, toolchain, package-management, or framework stacks.

Equivalent tool coverage is required for the first release, subject to explicit feasibility
assessment, but exact upstream APIs, output names, bundled versions, and implementation details
are not. The matrix is an inventory of the frozen list, not the sole definition of scope.
Neither the matrix nor a curated addition claims that an integration is currently implemented,
dogfooded, adapter-tested, or supported.

## Matrix

The row labels below are human-readable coverage families, not normative
`QualitySourcesInfo` keys. Adapter manifests map each integration to canonical semantic
file-class IDs from the registry defined by
[Quality Sources and Applicability](../quality/quality-sources.md). Grouped rows such as JSON/JSON5/JSONC
remain distinct classes when tool applicability or policy differs.

| Language or file class | Formatter choices | Lint, typecheck, or audit integrations |
| --- | --- | --- |
| Language-independent | | keep-sorted |
| C and C++ | clang-format | clang-tidy, cppcheck |
| C# | CSharpier | |
| CUE | cue fmt | |
| CUDA | clang-format | |
| CSS, Less, SCSS | Prettier | Stylelint |
| F# | Fantomas | |
| Go | gofmt, gofumpt | |
| Go modules | modfmt | |
| Gherkin | prettier-plugin-gherkin | |
| GraphQL | Prettier | |
| HTML | Prettier | |
| HTML templates | djlint | |
| JSON, JSON5, JSONC | Prettier | |
| Java | google-java-format | PMD, Checkstyle, SpotBugs |
| JavaScript | Prettier | ESLint |
| Jsonnet | jsonnetfmt | |
| Kotlin | ktfmt | ktlint |
| Markdown | Prettier | Vale |
| Pkl | pkl | |
| PowerShell | | PSScriptAnalyzer |
| Protocol Buffer | buf format | buf lint |
| Python | Ruff formatter | flake8, pydoclint, pylint, Ruff, Ty |
| QML | qmlformat | qmllint |
| Ruby | | RuboCop, StandardRB |
| Rust | rustfmt | Clippy |
| SQL | prettier-plugin-sql | |
| Scala | scalafmt | scalafix |
| Shell | shfmt | ShellCheck |
| Starlark | Buildifier | Buildifier |
| Terraform | terraform fmt | |
| TOML | Taplo | Taplo |
| TSX | Prettier | ESLint |
| TypeScript | Prettier | ESLint |
| Vue | Prettier | |
| YAML | yamlfmt | yamllint |
| XML | prettier-plugin-xml | |

## Curated Differences

Curated default membership and explicit opt-ins are distinct from baseline integration coverage.
Naming a tool in the matrix does not enable it by default. Omitted workspace selections use
curated defaults; supported alternatives remain explicit opt-ins. Enabled tools use pinned native
behavioral defaults unless an applicable checked-in native config supplies policy, as defined in
[Native Configuration](../quality/native-configuration.md#authority). Tool selection does not
authorize hidden behavioral presets. Unselected opt-ins create no actions or tool/runtime fetches.

The planned initial curated Python defaults are Ruff, Ty, and pydoclint. Ruff
and pydoclint run under lint and Ty runs under typecheck; initial audit tool selection is
owned by O11 (Bandit excluded from v1 by
[ADR 0019](../decisions/0019-first-release-additional-foundations.md)).
Ruff is planned as the default and only active Python formatter. Flake8 and pylint remain
baseline opt-ins.

Biome is a planned `rules_dx` integration beyond the frozen v2.8.0 baseline and is the
curated default linter and formatter for JavaScript, TypeScript, and TSX. Prettier remains
in the baseline as a formatter alternative, while ESLint is an opt-in linter. The planned
policy permits selecting both Biome and Prettier; they run against the current virtual source
in stable pipeline order. Formatting succeeds only after a complete round in which both leave
the source unchanged; cycles and iteration limits fail closed.

Planned TypeScript compiler diagnostics are emitted by `dx typecheck` actions derived
from `typescript_project`, not by lint or test adapter sets. The initial `tsc` adapter is
diagnostic-only.

Framework-container quality integrations remain gated by the applicable
[framework adapter contract](../generation/framework-adapters.md) and O29/O40-O43 in the
[open-decision register](../open-decisions.md). A tool's presence in the frozen list or curated
scope does not settle an exact framework parser, provider, region, or typecheck integration.

Adapter architecture, result normalization, mutation behavior, acquisition, and
update policy are defined in [Tool Integrations](../quality/tool-integrations.md),
[Quality Result Protocol](../quality/quality-result-protocol.md), and
[Tool Acquisition](tool-acquisition.md).

## Default Lifecycle Direction

Curated defaults are supported until keeping them no longer makes sense (upstream death,
unfixable hermeticity or platform failure, superseding default). Removing a default, changing
the default formatter set, or otherwise breaking consumer workflows requires a major release.
Only additions to curated lint/audit membership are eligible for a minor release, and only after
compatibility qualification, explicit release notes, and a tested workspace override preserving
the prior lint/audit set. An override does not make a removal, formatter-set change, or otherwise
breaking workflow eligible for a minor release. Required evidence is defined in
[Quality Workflow Testing](../quality/quality-testing.md#configured-policy).

Normal tool-version updates within the selected set follow the
[exact-pin currency policy](tool-acquisition.md), including qualification of native diagnostic
or formatting drift; they do not authorize default membership changes.

## Swift Feasibility

Swift, including SwiftFormat, is excluded from v1 by
[ADR 0019](../decisions/0019-first-release-additional-foundations.md).
SwiftFormat has no proven compliant upstream integration route here, and the v2.8.0
SwiftFormat integration requires a host Swift toolchain; that route is forbidden. This
exclusion is an evidence-backed v1 scope decision, not a feasibility assessment awaiting
O46, and SwiftFormat is not a v1 release blocker. Reconsidering Swift after v1 requires
a new scope decision.
