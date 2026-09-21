"""Quality family taxonomy execution pins.

Contract: `docs/quality/quality-sources.md`,
`docs/quality/tool-integrations.md`, `docs/quality/quality-testing.md`,
`docs/quality/action-model.md`, `docs/product/support-matrix.md`.
"""

# Taxonomy shape: 47 classes each with exactly one owning family;
# 39 owning families single-sourced in quality/adapters.bzl.
TAXONOMY_CLASSES = "47 classes each with exactly one owning family"
TAXONOMY_FAMILIES = "39 owning families"
TAXONOMY_SINGLE_SOURCED = "single-sourced in quality/adapters.bzl REAL_CLASS_TO_FAMILY"

# Grouped families share tool selection by design; unrelated classes
# never share a family.
GROUP_CSS = "css plus less plus scss in the css family"
GROUP_JSON = "json plus json5 plus jsonc in the json family"
GROUP_PYTHON = "python plus python_stub in the python family"
GROUP_TYPESCRIPT = "typescript plus tsx in the typescript family"
GROUP_JAVASCRIPT = "javascript plus jsx in the javascript family"
GROUP_CC = "c plus cpp in the cc family"

# Curated execution: 8 families carry lazy curated defaults; families
# absent here are PARITY_DEFERRED with owner plus frozen route.
CURATED_FAMILIES = "8 curated families: javascript plus json plus markdown plus python plus rust plus starlark plus toml plus typescript"
CURATED_JAVASCRIPT = "javascript family lint biome plus format biome"
CURATED_JSON = "json family lint biome plus format prettier"
CURATED_MARKDOWN = "markdown family lint markdown_check plus vale"
CURATED_PYTHON = "python family lint pydoclint plus ruff plus format ruff plus typecheck ty"
CURATED_RUST = "rust family lint clippy plus format rustfmt plus typecheck rustc"
CURATED_STARLARK = "starlark family lint buildifier plus format buildifier"
CURATED_TOML = "toml family lint taplo plus format taplo"
CURATED_TYPESCRIPT = "typescript family lint biome plus format biome plus typecheck tsc"

# Adapter-backed execution: 17 classes ride 29 real adapters with
# runner-matrix pass plus fail plus parser plus native-config plus
# aspect evidence. Scala plus C# plus F# delivered under #797; C plus
# C++ plus Go delivered under #798.
BACKED_CLASSES = "17 adapter-backed classes: c plus cpp plus csharp plus fsharp plus go plus javascript plus json plus jsx plus markdown plus python plus python_stub plus rust plus scala plus starlark plus toml plus tsx plus typescript"
BACKED_TOOLS = "29 real adapters: biome plus buildifier plus clang_format plus clang_tidy plus clippy plus cppcheck plus csharpier plus errcheck plus eslint plus fantomas plus flake8 plus fsharplint plus gofumpt plus govet plus markdown_check plus prettier plus pydoclint plus pylint plus roslyn plus ruff plus rustc plus rustfmt plus scalafix plus scalafmt plus staticcheck plus taplo plus tsc plus ty plus vale"

# Deferred execution boundary: 30 classes stay deferred with owning
# decision plus frozen delivery route; classification exists, adapter
# claim does not.
DEFERRED_COUNT = "30 deferred classes with owner plus frozen route"
DEFERRED_OWNER = "every deferral names ADR 0019 plus frozen delivery route"
DEFERRED_NO_DOUBLE_CLAIM = "no class is both adapter-backed and deferred"
DEFERRED_NO_UNDISPOSITIONED = "no classified class lacks a disposition"

# Audit taxonomy: curated audit stays empty with explicit disablement;
# Bandit excluded from v1; secrets ride the separate Gitleaks family.
AUDIT_EMPTY = "curated audit stays empty with explicit disablement"
AUDIT_BANDIT_EXCLUDED = "Bandit excluded from v1 by ADR 0019"
AUDIT_SECRETS_SEPARATE = "secrets family rides Gitleaks detect with redact plus SARIF"

# Applicability execution: provider classes intersect adapter support
# intersect policy; never suffix-inferred; cross-family union into one
# stage; lazy with no fetch on empty intersection; no hidden preset.
APPLICABILITY_INTERSECTION = "effective classes intersect provider plus adapter plus policy"
APPLICABILITY_NO_SUFFIX = "suffix inference rejected with registry-owned applicability"
APPLICABILITY_CROSS_FAMILY = "cross-family union into one stage with one acquisition identity"
APPLICABILITY_LAZY = "lazy curated defaults with no stage plus no fetch on empty subset"
APPLICABILITY_NO_PRESET = "no hidden preset with native-configuration sole policy"

# Admissibility stays owner-contextual with ambiguous cases pinned.
ADMISSIBILITY_BUILD = "BUILD files admissible by basename as starlark"
ADMISSIBILITY_H = "dot-h alone cannot decide c versus cpp"
ADMISSIBILITY_EXTENSIONLESS = "Extensionless requires shell rule plus provider evidence"
ADMISSIBILITY_TEMPLATE = "Requires template provider with bare html stays html"
ADMISSIBILITY_TEXT = "text never inferred as fallback"

# Registry singularity: one source of truth with versioned schemas;
# consumers query instead of duplicating allowlists.
REGISTRY_SINGLE_SOURCE = "single-sourced registry with no parallel allowlist"
REGISTRY_SCHEMA_V1 = "SOURCES plus ADAPTER plus CURATED plus PARITY schemas stay v1"
REGISTRY_QUERIES = "consumers query via registry plus curated plus parity queries"

# Execution evidence: real fixture policy plus runner matrix plus
# parsers plus native bindings plus aspects plus parity gate.
EXECUTION_POLICY = "real_fixture_policy executes the 8 curated families"
EXECUTION_MATRIX = "runner matrix pass plus fail per backed class plus capability"
EXECUTION_PARSERS = "every backed tool keeps a parser with pass plus fail samples"
EXECUTION_NATIVE = "native bindings for biome plus buildifier plus clang_format plus clang_tidy plus cppcheck plus csharpier plus eslint plus fsharplint plus ruff plus rustfmt plus scalafix plus scalafmt plus staticcheck plus taplo plus vale"
EXECUTION_ASPECTS = "real aspects wire target-coupled plus upstream-delegated plus check-only"
EXECUTION_PARITY_GATE = "parity gate fails closed on unclassified plus undispositioned plus double-claim"

# Rejected substitutes per the issue alternatives: taxonomy doc only
# with no fixture execution stays rejected here.
REJECTED_ALTERNATIVES = [
    "taxonomy doc only",
    "report-not-gate shape only",
    "unexecuted taxonomy",
]

# Owned gaps stay explicit: deferred adapters owned under issues
# 416 through 420 plus 307; digests plus rule-sets owned by their
# cohorts; platform plus consumer plus release evidence stays owned
# gap; no Supported claim.
OWNED_DEFERRED = "deferred adapters stay owned under issues 416 through 420 plus 307"
OWNED_DIGESTS = "digests plus rule-sets stay owned by their cohort qualifications"
OWNED_GAPS_NOTE = "platform plus consumer plus release evidence stays owned gap"
NO_SUPPORTED_CLAIM = "no Supported claim"
BACKENDS_PROVISIONAL = "backends stay provisional"

# Live proof labels: quality unit suites plus the fixture build prove
# execution on the seed host.
LIVE_QUALITY_ALL = "//quality:all"
LIVE_RUNNER_MATRIX = "//quality/testdata:runner_matrix"
LIVE_FIXTURE_CORPUS = "//quality/tests/fixtures/quality_taxonomy:corpus_starlark"
