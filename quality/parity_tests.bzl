"""Fail-closed v1 parity gate (WP1/WP3, ADR 0019).
"""

load("//libs/starlark:defs.bzl", "expect_equal", "starlark_test")
load(":adapters.bzl", "REAL_ADAPTERS", "REAL_CLASS_TO_FAMILY")

# Versioned parity-gate schema. Consumers query via
# `parity_schema_error`, `adapter_backed_classes`, and `deferred_classes`
# instead of duplicating the deferral inventory, so adding a deferred
# class edits this one data map plus its owning decision, never a parallel
# allowlist.
PARITY_SCHEMA_VERSION = 1

# Class -> [owning decision, frozen acquisition route] for every classified
# class no adapter claims yet. Owners are open-decision IDs (ADR 0019 owns the
# deferred adapter backlog; ADR 0019 owns the frozen Ruby/PowerShell tool routes;
# /// own the framework quality regions) or ADR 0019 for
# exclusions. Routes name the frozen delivery class from tool acquisition;
# exact versions, digests, rule sets, and adapter mappings stay pending
# under the owning decision.
PARITY_DEFERRED = {
    "astro": ["ADR 0019", "framework adapter region; Prettier/ESLint plugin closure pending"],
    "c": ["ADR 0019", "authoritative hermetic-llvm toolchain (clang-format, clang-tidy); cppcheck standalone artifact -- Native cohort owned by issue #418"],
    "cpp": ["ADR 0019", "authoritative hermetic-llvm toolchain (clang-format, clang-tidy); cppcheck standalone artifact -- Native cohort owned by issue #418"],
    "csharp": ["ADR 0019", "exact upstream package plus shared .NET runtime (CSharpier); Roslyn CA analyzers SDK-coupled -- Scala/.NET cohort owned by issue #417"],
    "css": ["ADR 0019", "private Node graph (Prettier); standalone artifact (Stylelint) -- Interpreted/file-family cohort owned by issue #420"],
    "cuda": ["ADR 0019", "authoritative toolchain (clang-format); CUDA scope fails closed until qualified"],
    "cue": ["ADR 0019", "checksummed standalone artifact (cue fmt) -- Interpreted/file-family cohort owned by issue #420"],
    "fsharp": ["ADR 0019", "exact upstream package plus shared .NET runtime (Fantomas); FSharpLint pending -- Scala/.NET cohort owned by issue #417"],
    "gherkin": ["ADR 0019", "private Node graph (Prettier prettier-plugin-gherkin closure) -- Interpreted/file-family cohort owned by issue #420"],
    "go": ["ADR 0019", "authoritative Go toolchain (gofmt/gofumpt); staticcheck/govet standalone artifacts -- Native cohort owned by issue #418"],
    "go_module": ["ADR 0019", "checksummed standalone artifact (modfmt) -- Interpreted/file-family cohort owned by issue #420"],
    "graphql": ["ADR 0019", "private Node graph (Prettier GraphQL parser)"],
    "html": ["ADR 0019", "private Node graph (Prettier HTML parser)"],
    "html_template": ["ADR 0019", "checksummed standalone artifact (djlint); shared managed Python runtime -- Interpreted/file-family cohort owned by issue #420"],
    "java": ["ADR 0019", "complete upstream artifact plus shared JDK (google-java-format, Checkstyle, PMD, SpotBugs) -- JVM cohort owned by issue #416"],
    "json5": ["ADR 0019", "private Node graph (Prettier JSON5 parser); Biome JSON-family extension pending"],
    "jsonc": ["ADR 0019", "private Node graph (Prettier JSONC parser); Biome JSON-family extension pending"],
    "jsonnet": ["ADR 0019", "checksummed standalone artifact (jsonnetfmt) -- Interpreted/file-family cohort owned by issue #420"],
    "kotlin": ["ADR 0019", "complete upstream artifact plus shared JDK (ktfmt, ktlint); detekt pending -- JVM cohort owned by issue #416"],
    "less": ["ADR 0019", "private Node graph (Prettier); standalone artifact (Stylelint)"],
    "mdx": ["ADR 0019", "framework adapter region; prose-is-not-dependency boundary per composition"],
    "pkl": ["ADR 0019", "checksummed standalone artifact (pkl) -- Interpreted/file-family cohort owned by issue #420"],
    "powershell": ["ADR 0019", "exact module plus portable PowerShell runtime (PSScriptAnalyzer); foundation deferred by ADR 0019 -- Interpreted/file-family cohort owned by issue #420"],
    "protobuf": ["ADR 0019", "checksummed standalone artifact (buf format+lint) -- Structured cohort owned by issue #419"],
    "qml": ["ADR 0019", "authoritative Qt distribution toolchain (qmlformat, qmllint) -- Structured cohort owned by issue #419"],
    "ruby": ["ADR 0019", "release-assembled Ruby closure (RuboCop, StandardRB); foundation deferred by ADR 0019 -- Interpreted/file-family cohort owned by issue #420"],
    "scala": ["ADR 0019", "managed JVM route: compatible JVM artifact (scalafmt); semantic-rule artifacts over shared JDK (Scalafix) -- Scala/.NET cohort owned by issue #417"],
    "scss": ["ADR 0019", "private Node graph (Prettier); standalone artifact (Stylelint)"],
    "shell": ["ADR 0019", "checksummed standalone artifacts (shfmt, ShellCheck)"],
    "sql": ["ADR 0019", "private Node graph (Prettier prettier-plugin-sql closure) -- Interpreted/file-family cohort owned by issue #420"],
    "svelte": ["ADR 0019", "framework adapter region; Prettier/ESLint plugin closure pending"],
    "terraform": ["ADR 0019", "checksummed standalone artifact (terraform fmt) -- Interpreted/file-family cohort owned by issue #420"],
    "text": ["ADR 0019", "checksummed standalone artifact (keep-sorted) -- Interpreted/file-family cohort owned by issue #420"],
    "vue": ["ADR 0019", "framework adapter region; Prettier/ESLint plugin closure pending"],
    "xml": ["ADR 0019", "private Node graph (Prettier prettier-plugin-xml closure) -- Interpreted/file-family cohort owned by issue #420"],
    "yaml": ["ADR 0019", "checksummed standalone artifacts (yamlfmt, yamllint); shared managed Python runtime for yamllint -- Interpreted/file-family cohort owned by issue #420"],
}

def _adapter_backed_classes():
    backed = {}
    for tool in REAL_ADAPTERS:
        for capability in REAL_ADAPTERS[tool]:
            for class_id in REAL_ADAPTERS[tool][capability]:
                backed[class_id] = True
    return backed

def adapter_backed_classes():
    """Returns the sorted adapter-backed classes via registry query.

    Derived from `REAL_ADAPTERS` capabilities, never duplicated, so adding
 an adapter claim edits the adapter registry data only.
    """
    return sorted(_adapter_backed_classes().keys())

def deferred_classes():
    """Returns the sorted explicitly deferred classes via registry query.

    Derived from `PARITY_DEFERRED` keys, never duplicated, so adding a
 deferral edits this one data map only.
    """
    return sorted(PARITY_DEFERRED.keys())

def _is_canonical_token(text):
    if text == "":
        return False
    for c in text.elems():
        if c not in "abcdefghijklmnopqrstuvwxyz0123456789_":
            return False
    return True

def parity_schema_error():
    """Validates the versioned parity-gate schema.

    Checks data shape without pinning exact contents, so adding a deferred
    class edits the deferral data only: version is v1, every deferred ID
    is canonical, and every entry carries an owning decision plus a frozen
    route. Disposition coverage (unclassified/undispositioned/double-claim)
    stays checked by the registry queries below, not by an allowlist."""
    if PARITY_SCHEMA_VERSION != 1:
        return "parity gate: unsupported schema v" + str(PARITY_SCHEMA_VERSION) + " (want v1)"
    for class_id in PARITY_DEFERRED:
        if not _is_canonical_token(class_id):
            return "parity gate: non-canonical deferred class '" + str(class_id) + "'"
        entry = PARITY_DEFERRED[class_id]
        if len(entry) != 2 or entry[0] == "" or entry[1] == "":
            return "parity gate: deferred class '" + class_id + "' must name an owning decision and a frozen route"
        owner = entry[0]
        if not owner.startswith("O") and not owner.startswith("ADR"):
            return "parity gate: deferred class '" + class_id + "' owner '" + owner + "' must start with O or ADR"
    return ""

def _unclassified_adapter_classes():
    backed = _adapter_backed_classes()
    return sorted([c for c in backed if c not in REAL_CLASS_TO_FAMILY])

def _undispositioned_classes():
    backed = _adapter_backed_classes()
    return sorted([c for c in REAL_CLASS_TO_FAMILY if c not in backed and c not in PARITY_DEFERRED])

def _double_claimed_classes():
    backed = _adapter_backed_classes()
    return sorted([c for c in PARITY_DEFERRED if c in backed])

def _malformed_deferrals():
    bad = []
    for class_id in sorted(PARITY_DEFERRED):
        entry = PARITY_DEFERRED[class_id]
        if len(entry) != 2 or entry[0] == "" or entry[1] == "":
            bad.append(class_id)
            continue
        owner = entry[0]
        if not owner.startswith("O") and not owner.startswith("ADR"):
            bad.append(class_id)
    return bad

def parity_unit_tests(name):
    starlark_test(
        name = name,
        mode = "unit",
        checks = [
            expect_equal(
                "parity schema version stays v1",
                PARITY_SCHEMA_VERSION,
                1,
            ),
            expect_equal(
                "parity deferral schema validates",
                parity_schema_error(),
                "",
            ),
            expect_equal(
                "no adapter class is unclassified",
                _unclassified_adapter_classes(),
                [],
            ),
            expect_equal(
                "no classified class lacks a disposition",
                _undispositioned_classes(),
                [],
            ),
            expect_equal(
                "no class is both adapter-backed and deferred",
                _double_claimed_classes(),
                [],
            ),
            expect_equal(
                "every deferral names an owning decision and a frozen route",
                _malformed_deferrals(),
                [],
            ),
            expect_equal(
                "adapter-backed query matches the registry derivation",
                adapter_backed_classes(),
                sorted(_adapter_backed_classes().keys()),
            ),
            expect_equal(
                "deferred query matches the deferral registry",
                deferred_classes(),
                sorted(PARITY_DEFERRED.keys()),
            ),
        ],
    )
