"""Laziness analysis pins: zero-delta fetches/actions plus analysis budget.

Contract: `docs/testing/tools.md#laziness`.
Fixture: `tools/ci/tests/fixtures/laziness_analysis/` via
`bazel run //tools/ci:laziness_analysis_guard`.
"""

# M1 configured-target counts: `bazel cquery "deps(//examples/<ex>/...)"`
# `--output=label | LC_ALL=C sort -u | wc -l` on the seed host. Delta must
# be zero: adding a bazel_dep+extension must not add configured targets to
# single-foundation adopt-* consumers. Each corpus_starlark also binds
# `//:buildifier_config`, costing the same two configured targets the
# Markdown corpus's Vale binding costs (config target plus its source).
# The C locale is required: UTF-8 collation merges distinct labels and
# undercounts by 2-4 targets per consumer, which is the count CI reports.
CQUERY_EXPECTED = {
    "adopt-cpp": 242,
    "adopt-csharp": 4813,
    "adopt-fsharp": 4827,
    "adopt-go": 6132,
    "adopt-java": 2592,
    "adopt-js-ts": 6124,
    "adopt-kotlin": 2854,
    "adopt-python": 2407,
    "adopt-rust": 487,
    "adopt-scala": 4290,
}

# M2 action counts: `bazel aquery //examples/<ex>/... --output=text |
# grep -c ActionKey:` on the seed host after one `bazel shutdown` plus one
# warming cquery (guard order: shutdown once, then per-consumer
# cquery/aquery/build). Count gate atop the slice-4 marker gate: delta
# must be zero. Rust warms 87 cold to 92 warm; the guard pins warm.
AQUERY_EXPECTED = {
    "adopt-cpp": 39,
    "adopt-csharp": 32,
    "adopt-fsharp": 32,
    "adopt-go": 34,
    "adopt-java": 40,
    "adopt-js-ts": 62,
    "adopt-kotlin": 44,
    "adopt-python": 126,
    "adopt-rust": 92,
    "adopt-scala": 46,
}

# M4 analysis budgets (milliseconds): `bazel build --nobuild
# --profile=<scratch>/profile.json` runAnalysisPhase duration. Warm seed
# measures 11-823ms, cold ~1.5s; budgets hold generous headroom
# (pin + max(30%, 5s), rounded to 30s uniform) per ADR 0022: counts hard
# fail, time is informational-regression (noisy-neighbor rerun legitimate).
ANALYSIS_BUDGET_MS = {
    "adopt-cpp": 30000,
    "adopt-csharp": 30000,
    "adopt-fsharp": 30000,
    "adopt-go": 30000,
    "adopt-java": 30000,
    "adopt-js-ts": 30000,
    "adopt-kotlin": 30000,
    "adopt-python": 30000,
    "adopt-rust": 30000,
    "adopt-scala": 30000,
}

# M3 fetch allowlists: base modules that may appear in the configured
# `cquery deps()` repo set for each single-foundation consumer. Every repo
# base must be a member; anything else (e.g. npm in rust, pypi in go,
# maven in python, paket/dotnet in rust) fails. Base toolchains
# (bazel_tools/platforms/rules_shell plus cc/java where they are base)
# are allowlisted explicitly, never asserted as ownership markers.
# Derived from `bazel cquery deps() --output=label` repo prefixes on the
# seed host (Bzlmod has no resolved file on Bazel 9.2.0; the cquery repo
# set is the portable configured-fetch equivalent, analysis-only).
# Own-foundation acquisition repos arrive with the consumer lock fixtures
# (pom, paket, go.mod, gtest pins) and stay foundation-owned; a foreign
# ecosystem in a single-foundation closure still fails.
FETCH_ALLOWLIST = {
    "adopt-cpp": ["bazel_tools", "googletest", "platforms", "rules_cc", "rules_shell"],
    "adopt-csharp": ["bazel_tools", "main", "paket.main", "platforms", "rules_dotnet", "rules_shell"],
    "adopt-fsharp": ["bazel_tools", "main", "paket.main", "platforms", "rules_dotnet", "rules_shell"],
    "adopt-go": ["bazel_tools", "com_github_google_go_cmp", "platforms", "rules_cc", "rules_go", "rules_shell"],
    "adopt-java": ["aspect_rules_py", "bazel_skylib", "bazel_tools", "maven", "package_metadata", "platforms", "rules_cc", "rules_java", "rules_jvm_external", "rules_python", "rules_shell"],
    "adopt-js-ts": ["aspect_bazel_lib", "aspect_rules_jest", "aspect_rules_js", "aspect_rules_ts", "bazel_lib", "bazel_skylib", "bazel_tools", "npm_typescript", "platforms", "rules_nodejs", "rules_shell", "tar.bzl"],
    "adopt-kotlin": ["aspect_rules_py", "bazel_skylib", "bazel_tools", "maven", "package_metadata", "platforms", "rules_cc", "rules_java", "rules_jvm_external", "rules_kotlin", "rules_python", "rules_shell"],
    "adopt-python": ["aspect_rules_py", "bazel_tools", "hermetic_launcher", "platforms", "pypi", "rules_python", "rules_shell"],
    "adopt-rust": ["bazel_skylib", "bazel_tools", "platforms", "rules_cc", "rules_rust", "rules_shell"],
    "adopt-scala": ["abseil-cpp", "aspect_rules_py", "bazel_skylib", "bazel_tools", "bazel_worker_api", "maven", "package_metadata", "platforms", "protobuf", "rules_cc", "rules_java", "rules_jvm_external", "rules_license", "rules_python", "rules_scala", "rules_scala_config", "rules_shell", "zlib"],
}

# Forbidden ecosystem markers per consumer family (aquery action-graph
# only): never assert rules_cc/rules_java/bazel_tools/skylib/platforms
# (base toolchains spanning closures, same rule as slices 3/4). Fetch uses
# the allowlist subset above, not this list: configured cquery legitimately
# resolves base-toolchain pulls (e.g. aspect_rules_py/rules_python into
# java/kotlin/scala closures) that must stay allowlisted, so forbidding
# them as fetches would false-fail.
FORBIDDEN_RUST = ["aspect_rules_py", "aspect_rules_js", "rules_go", "rules_dotnet", "rules_kotlin", "rules_scala"]
FORBIDDEN_PYTHON = ["rules_rust", "aspect_rules_js", "rules_go", "rules_dotnet", "rules_kotlin", "rules_scala"]
FORBIDDEN_JS = ["rules_rust", "aspect_rules_py", "rules_go", "rules_dotnet", "rules_kotlin", "rules_scala"]
FORBIDDEN_GO = ["rules_rust", "aspect_rules_py", "aspect_rules_js", "rules_dotnet", "rules_kotlin", "rules_scala"]
FORBIDDEN_DOTNET = ["rules_rust", "aspect_rules_py", "aspect_rules_js", "rules_go", "rules_kotlin", "rules_scala"]
FORBIDDEN_KOTLIN = ["rules_rust", "aspect_rules_py", "aspect_rules_js", "rules_go", "rules_dotnet", "rules_scala"]
FORBIDDEN_SCALA = ["rules_rust", "aspect_rules_py", "aspect_rules_js", "rules_go", "rules_dotnet", "rules_kotlin"]
FORBIDDEN_NEGATIVE = ["rules_rust", "aspect_rules_py", "aspect_rules_js", "rules_go", "rules_dotnet", "rules_kotlin", "rules_scala"]
