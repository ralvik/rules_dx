"""Quality family taxonomy execution pins.

"""

TAXONOMY_CLASSES = "47 classes each with exactly one owning family"
TAXONOMY_FAMILIES = "39 owning families"
TAXONOMY_SINGLE_SOURCED = "single-sourced in quality/adapters.bzl REAL_CLASS_TO_FAMILY"

GROUP_CSS = "css plus less plus scss in the css family"
GROUP_JSON = "json plus json5 plus jsonc in the json family"
GROUP_PYTHON = "python plus python_stub in the python family"
GROUP_TYPESCRIPT = "typescript plus tsx in the typescript family"
GROUP_JAVASCRIPT = "javascript plus jsx in the javascript family"
GROUP_CC = "c plus cpp in the cc family"

CURATED_FAMILIES = "10 curated families: java plus javascript plus json plus kotlin plus markdown plus python plus rust plus starlark plus toml plus typescript"
CURATED_JAVA = "java family format google_java_format plus lint checkstyle plus pmd plus spotbugs"
CURATED_KOTLIN = "kotlin family format ktfmt plus lint ktlint"
CURATED_JAVASCRIPT = "javascript family lint biome plus format biome"
CURATED_JSON = "json family lint biome plus format prettier"
CURATED_MARKDOWN = "markdown family lint markdown_check plus vale"
CURATED_PYTHON = "python family lint pydoclint plus ruff plus format ruff plus typecheck ty"
CURATED_RUST = "rust family lint clippy plus format rustfmt plus typecheck rustc"
CURATED_STARLARK = "starlark family lint buildifier plus format buildifier"
CURATED_TOML = "toml family lint taplo plus format taplo"
CURATED_TYPESCRIPT = "typescript family lint biome plus format biome plus typecheck tsc"

BACKED_CLASSES = "39 adapter-backed classes: c plus cpp plus cuda plus csharp plus fsharp plus go plus java plus javascript plus json plus jsx plus kotlin plus markdown plus protobuf plus python plus python_stub plus qml plus rust plus scala plus starlark plus toml plus tsx plus typescript plus css plus cue plus gherkin plus go_module plus html_template plus jsonnet plus less plus pkl plus powershell plus ruby plus scss plus shell plus sql plus terraform plus text plus xml plus yaml"
BACKED_TOOLS = "53 real adapters: biome plus buf plus buildifier plus checkstyle plus clang_format plus clang_tidy plus clippy plus cppcheck plus csharpier plus cue plus djlint plus errcheck plus eslint plus fantomas plus flake8 plus fsharplint plus gofumpt plus google_java_format plus govet plus jsonnetfmt plus keep_sorted plus ktfmt plus ktlint plus markdown_check plus modfmt plus pkl plus pmd plus prettier plus psscriptanalyzer plus pydoclint plus pylint plus qmlformat plus qmllint plus roslyn plus rubocop plus ruff plus rustc plus rustfmt plus scalafix plus scalafmt plus shellcheck plus shfmt plus spotbugs plus standardrb plus staticcheck plus stylelint plus taplo plus terraform plus tsc plus ty plus vale plus yamlfmt plus yamllint"

DEFERRED_COUNT = "8 deferred classes with owner plus frozen route"
DEFERRED_OWNER = "every deferral names ADR 0019 plus frozen delivery route"
DEFERRED_NO_DOUBLE_CLAIM = "no class is both adapter-backed and deferred"
DEFERRED_NO_UNDISPOSITIONED = "no classified class lacks a disposition"

AUDIT_EMPTY = "curated audit stays empty with explicit disablement"
AUDIT_BANDIT_EXCLUDED = "Bandit excluded from v1 by ADR 0019"
AUDIT_SECRETS_SEPARATE = "secrets family rides Gitleaks detect with redact plus SARIF"

APPLICABILITY_INTERSECTION = "effective classes intersect provider plus adapter plus policy"
APPLICABILITY_NO_SUFFIX = "suffix inference rejected with registry-owned applicability"
APPLICABILITY_CROSS_FAMILY = "cross-family union into one stage with one acquisition identity"
APPLICABILITY_LAZY = "lazy curated defaults with no stage plus no fetch on empty subset"
APPLICABILITY_NO_PRESET = "no hidden preset with native-configuration sole policy"

ADMISSIBILITY_BUILD = "BUILD files admissible by basename as starlark"
ADMISSIBILITY_H = "dot-h alone cannot decide c versus cpp"
ADMISSIBILITY_EXTENSIONLESS = "Extensionless requires shell rule plus provider evidence"
ADMISSIBILITY_TEMPLATE = "Requires template provider with bare html stays html"
ADMISSIBILITY_TEXT = "text never inferred as fallback"

REGISTRY_SINGLE_SOURCE = "single-sourced registry with no parallel allowlist"
REGISTRY_SCHEMA_V1 = "SOURCES plus ADAPTER plus CURATED plus PARITY schemas stay v1"
REGISTRY_QUERIES = "consumers query via registry plus curated plus parity queries"

EXECUTION_POLICY = "real_fixture_policy executes the 10 curated families"
EXECUTION_MATRIX = "runner matrix pass plus fail per backed class plus capability"
EXECUTION_PARSERS = "every backed tool keeps a parser with pass plus fail samples"
EXECUTION_NATIVE = "native bindings for biome plus buf plus buildifier plus checkstyle plus clang_format plus clang_tidy plus cppcheck plus csharpier plus djlint plus eslint plus fsharplint plus qmlformat plus qmllint plus ruff plus rustfmt plus scalafix plus scalafmt plus staticcheck plus stylelint plus taplo plus vale plus yamllint"
EXECUTION_ASPECTS = "real aspects wire target-coupled plus upstream-delegated plus check-only"
EXECUTION_PARITY_GATE = "parity gate fails closed on unclassified plus undispositioned plus double-claim"

REJECTED_ALTERNATIVES = [
    "taxonomy doc only",
    "report-not-gate shape only",
    "unexecuted taxonomy",
]

OWNED_DEFERRED = "deferred adapters delivered under 796 plus 797 plus 798 plus 799 plus 800 with remaining owned under ADR 0019 plus 307"
OWNED_DIGESTS = "digests plus rule-sets stay owned by their cohort qualifications"
OWNED_DIGEST_POLICY = "digest policy with JVM digests pinned in quality/tools/jvm/repos.bzl plus standalone per-host digests in quality/artifacts plus rule-sets qualified under 485 through 489"
OWNED_PLATFORM = "platform evidence with per-host artifacts plus coverage cells plus CI matrix plus unsupported_platform refusal"
OWNED_CONSUMER = "consumer evidence with adopt workspaces plus reusable-consumer workflow"
OWNED_RELEASE = "release evidence with promotion-checklist plus SBOM plus signing plus supported_evidence_gate linkage"
OWNED_PROMOTION = "platform plus consumer plus release evidence stays owned gap with support-matrix linkage under issue 802"
OWNED_GAPS_NOTE = "platform plus consumer plus release evidence stays owned gap"
NO_SUPPORTED_CLAIM = "no Supported claim"
BACKENDS_PROVISIONAL = "backends stay provisional"

LIVE_QUALITY_ALL = "//quality:all"
LIVE_RUNNER_MATRIX = "//quality/testdata:runner_matrix"
LIVE_FIXTURE_CORPUS = "//quality/tests/fixtures/quality_taxonomy:corpus_starlark"
