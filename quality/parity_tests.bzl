"""Fail-closed v1 parity gate (M24 WP1/WP3, O32).

Reconciles the three parity dimensions in one machine-checked place: every
class named by any `REAL_ADAPTERS` capability must be classified in
`REAL_CLASS_TO_FAMILY`, and every classified class must have exactly one
disposition — adapter-backed (named by at least one tool capability) or
explicitly deferred in `PARITY_DEFERRED` with an owning decision and a
frozen acquisition route. A new class without a disposition, a new adapter
claim on a deferred class, a silently dropped adapter claim, or a
deferral without an owner/route fails here instead of weakening v1
silently. Deferred adapter implementation stays owned by O32; this gate
owns the inventory, not the adapters.
"""

load("//libs/starlark:defs.bzl", "expect_equal", "starlark_test")
load(":adapters.bzl", "REAL_ADAPTERS", "REAL_CLASS_TO_FAMILY")

# Class -> [owning decision, frozen acquisition route] for every classified
# class no adapter claims yet. Owners are open-decision IDs (O32 owns the
# deferred adapter backlog; O31 owns the frozen Ruby/PowerShell tool routes;
# O29/O40/O41/O42 own the framework quality regions) or ADR 0019 for
# exclusions. Routes name the frozen delivery class from tool acquisition;
# exact versions, digests, rule sets, and adapter mappings stay pending
# under the owning decision.
PARITY_DEFERRED = {
    "astro": ["O42/O32", "framework adapter region; Prettier/ESLint plugin closure pending"],
    "c": ["O32", "authoritative hermetic-llvm toolchain (clang-format, clang-tidy); cppcheck standalone artifact"],
    "cpp": ["O32", "authoritative hermetic-llvm toolchain (clang-format, clang-tidy); cppcheck standalone artifact"],
    "csharp": ["O32", "exact upstream package plus shared .NET runtime (CSharpier); Roslyn CA analyzers SDK-coupled"],
    "cuda": ["O32", "authoritative toolchain (clang-format); CUDA scope fails closed until qualified"],
    "cue": ["O32", "checksummed standalone artifact (cue fmt)"],
    "fsharp": ["O32", "exact upstream package plus shared .NET runtime (Fantomas); FSharpLint pending"],
    "go": ["O32", "authoritative Go toolchain (gofmt/gofumpt); staticcheck/govet standalone artifacts"],
    "go_module": ["O32", "checksummed standalone artifact (modfmt)"],
    "java": ["O32", "complete upstream artifact plus shared JDK (google-java-format, Checkstyle, PMD, SpotBugs)"],
    "jsonnet": ["O32", "checksummed standalone artifact (jsonnetfmt)"],
    "kotlin": ["O32", "complete upstream artifact plus shared JDK (ktfmt, ktlint); detekt pending"],
    "mdx": ["O32", "framework adapter region; prose-is-not-dependency boundary per O42 composition"],
    "pkl": ["O32", "checksummed standalone artifact (pkl)"],
    "powershell": ["O31/O32", "exact module plus portable PowerShell runtime (PSScriptAnalyzer); foundation deferred by ADR 0019"],
    "protobuf": ["O32", "checksummed standalone artifact (buf format+lint)"],
    "qml": ["O32", "authoritative Qt distribution toolchain (qmlformat, qmllint)"],
    "ruby": ["O31/O32", "release-assembled Ruby closure (RuboCop, StandardRB); foundation deferred by ADR 0019"],
    "scala": ["O32", "managed JVM route: compatible JVM artifact (scalafmt); semantic-rule artifacts over shared JDK (Scalafix)"],
    "shell": ["O32", "checksummed standalone artifacts (shfmt, ShellCheck)"],
    "svelte": ["O40/O32", "framework adapter region; Prettier/ESLint plugin closure pending"],
    "terraform": ["O32", "checksummed standalone artifact (terraform fmt)"],
    "vue": ["O29/O32", "framework adapter region; Prettier/ESLint plugin closure pending"],
    "yaml": ["O32", "checksummed standalone artifacts (yamlfmt, yamllint); shared managed Python runtime for yamllint"],
}

def _adapter_backed_classes():
    backed = {}
    for tool in REAL_ADAPTERS:
        for capability in REAL_ADAPTERS[tool]:
            for class_id in REAL_ADAPTERS[tool][capability]:
                backed[class_id] = True
    return backed

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
    backed = _adapter_backed_classes()
    starlark_test(
        name = name,
        mode = "unit",
        checks = [
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
                "adapter-backed classes",
                sorted(backed.keys()),
                [
                    "javascript",
                    "json",
                    "jsx",
                    "markdown",
                    "python",
                    "python_stub",
                    "rust",
                    "starlark",
                    "toml",
                    "tsx",
                    "typescript",
                ],
            ),
            expect_equal(
                "deferred classes",
                sorted(PARITY_DEFERRED.keys()),
                [
                    "astro",
                    "c",
                    "cpp",
                    "csharp",
                    "cuda",
                    "cue",
                    "fsharp",
                    "go",
                    "go_module",
                    "java",
                    "jsonnet",
                    "kotlin",
                    "mdx",
                    "pkl",
                    "powershell",
                    "protobuf",
                    "qml",
                    "ruby",
                    "scala",
                    "shell",
                    "svelte",
                    "terraform",
                    "vue",
                    "yaml",
                ],
            ),
        ],
    )
