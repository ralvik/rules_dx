"""Deterministic synthetic adapter registry (M03 WP2 fixtures).

Provisional M03-only registry proving target/capability pipeline
construction without real tools. Tool IDs are synthetic; the real family
taxonomy and curated defaults belong to the tool baseline (M04+).

Contract: `docs/quality/tool-integrations.md`, `docs/quality/quality-sources.md`
(adapter applicability), `docs/decisions/0003-action-granularity.md`
(provisional stage order).

Stage order note (O19): WP2 uses lexical tool-ID order as the stable
ruleset order for the synthetic set. Real capability orders are selected
from convergence/interaction/performance fixtures, never from user list
order; this lexical pick is flagged for review when real adapters land.
"""

# Synthetic tool ID to capability to supported semantic file classes.
# Mirrors the `//quality:fixture_policy` shape: `lint-a` is selected by two
# families (proves cross-family union into one stage), `lint-b` and `fmt-a`
# are rust-only (prove exact subsets and python exclusion from format).
SYNTHETIC_ADAPTERS = {
    "fmt-a": {"format": ["rust"]},
    "lint-a": {"lint": ["python", "rust"]},
    "lint-b": {"lint": ["rust"]},
}

# WP2 fixture class-to-family assignment. The full registry assignment stays
# pending the O15 review; this minimal map only authorizes the fixture
# families above.
SYNTHETIC_CLASS_TO_FAMILY = {
    "python": "python",
    "rust": "rust",
}

def adapter_supported_classes(tool_id, capability):
    """Returns the sorted supported classes for one synthetic tool/capability.

    Fails on an unknown tool ID (matches `applicability.selected_adapters`
    failing "during configuration or analysis"). An unsupported capability
    returns [], so the stage is omitted and no empty action is created.
    """
    if tool_id not in SYNTHETIC_ADAPTERS:
        fail("adapters: unknown tool '" + tool_id +
             "': not in the synthetic adapter registry")
    return sorted(SYNTHETIC_ADAPTERS[tool_id].get(capability, []))

# Real initial-adapter capability manifests (M04 WP2-WP3, O20; M12 WP3 adds
# the rustc typecheck stage; M15 WP2 adds the curated Python adapters
# ruff/ty/pydoclint; M15 WP3 adds the flake8/pylint lint opt-ins; M17 WP2
# adds the curated JavaScript/TypeScript/JSON adapters biome/eslint/prettier
# and the target-coupled tsc typecheck adapter).
#
# Tool IDs are the stable built-in identifiers users select in policy
# families. Every class has exactly one tool per capability except
# Markdown lint, where the repo-owned `markdown_check` (link/structure)
# and Vale (prose style) run as two ordered stages over the same files;
# no virtual convergence across tools runs yet. The pipeline formula
# orders stages by sorted tool ID (the provisional O19 rule) when several
# apply to one target. Rust typechecking is the toolchain `rustc` itself
# (`rust_toolchain_rustc`), invoked as a lib-root metadata check with no
# config discovery; it is check-only and never applies suggestions.
# Python lint runs flake8, pydoclint, pylint, then ruff in lexical stage
# order when selected; ruff is also the default and only active Python
# formatter and ty the typechecker, per the tool-baseline curated
# defaults. flake8 and pylint are baseline opt-ins: selectable in policy
# families but absent from the curated defaults, always pinned upstream
# defaults with no native-config rule, check-only, never rewriting.
# JavaScript/TypeScript/JSON (M17 WP2): biome is the default linter and
# formatter for the javascript, jsx, typescript, and tsx classes and the
# default linter for json; prettier is the default json formatter and the
# formatter alternative for javascript/jsx/typescript/tsx (selecting both
# formatters runs them in stable pipeline order per the tool baseline);
# eslint is a lint opt-in for javascript/jsx only (no TypeScript parser is
# installed, so typescript/tsx files match no eslint configuration and stay
# out of eslint stages). tsc typechecks typescript/tsx but is
# target-coupled: it never applies from the class alone and requires the
# authoritative typescript_project context (TsConfigInfo).
REAL_ADAPTERS = {
    "biome": {
        "format": ["javascript", "json", "jsx", "typescript", "tsx"],
        "lint": ["javascript", "json", "jsx", "typescript", "tsx"],
    },
    "buildifier": {"format": ["starlark"], "lint": ["starlark"]},
    "clippy": {"lint": ["rust"]},
    "eslint": {"lint": ["javascript", "jsx"]},
    "flake8": {"lint": ["python", "python_stub"]},
    "markdown_check": {"lint": ["markdown"]},
    "prettier": {"format": ["javascript", "json", "jsx", "typescript", "tsx"]},
    "pydoclint": {"lint": ["python", "python_stub"]},
    "pylint": {"lint": ["python", "python_stub"]},
    "ruff": {"format": ["python", "python_stub"], "lint": ["python", "python_stub"]},
    "rustc": {"typecheck": ["rust"]},
    "rustfmt": {"format": ["rust"]},
    "taplo": {"format": ["toml"], "lint": ["toml"]},
    "tsc": {"typecheck": ["typescript", "tsx"]},
    "ty": {"typecheck": ["python", "python_stub"]},
    "vale": {"lint": ["markdown"]},
}

# WP2 fixture class-to-family assignment for the M04 source classes. Like
# the synthetic map, this is a fixture, not the frozen taxonomy: the full
# registry assignment and curated defaults stay pending the O15/O17
# reviews. M15 WP2 adds the python/python_stub classes to the python
# family for the curated Ruff/Ty/pydoclint adapters. M17 WP2 adds the
# javascript/jsx classes to the javascript family, typescript/tsx to the
# typescript family, and json to the json family, per the frozen
# class-to-policy-family assignment (JavaScript owns javascript/jsx,
# TypeScript owns typescript/tsx, JSON owns the JSON classes). M18 WP2
# adds the vue class to the vue family: the `.vue` container stays one
# physical owner (vue_library) with virtual script/template/style
# regions handed to execution-time integrations; no lint/format/typecheck
# adapter claims vue yet, so the family selects no stages (classification
# only, no supported claim). M19 WP2 adds the svelte class to the svelte
# family on the same terms: the `.svelte` container stays one physical
# owner (svelte_library) with virtual instance/module-script, markup,
# and style regions handed to execution-time integrations; no adapter
# claims svelte yet (classification only, no supported claim). M20 WP2 adds
# the astro class to the astro family on the same terms: the `.astro`
# container stays one physical owner (astro_library) with virtual
# frontmatter/server-script, template, client-script, and style regions
# handed to execution-time integrations; no adapter claims astro yet
# (classification only, no supported claim). M21 WP2 adds the mdx class
# to the mdx family on the same terms: the `.mdx` container stays one
# physical owner (mdx_library) with virtual ESM/prose/JSX regions
# handed to execution-time integrations; no adapter claims mdx yet
# (classification only, no supported claim). M22 WP1 adds the go class
# to the go family on the same terms: each `.go` source has one
# physical owner (go_library, go_binary, go_test) with
# package-level test semantics preserved per the generation contract Go
# exception; no adapter claims go yet (classification only, no
# supported claim). M22 WP1 adds the c and cpp classes to the cc family
# on the same terms: each `.c`/`.h`/`.cc`/`.cpp`/`.cxx` source has one
# physical owner (cc_library, cc_binary, cc_test) with header
# ownership per extension; no adapter claims c/cpp yet (classification
# only, no supported claim). M22 WP3 adds the remaining native/toolchain
# standalone classes, one family per class, on the same terms: each source
# family keeps its existing ownership (foundation wrappers where admitted,
# otherwise the file itself); no lint/format/typecheck adapter claims any
# of these classes yet (classification only, no supported claim). O30
# qualifies exact adapters later; Qt stays last in M22. M23 WP2 adds the
# java class to the java family on the same terms: each `.java` source has
# one physical owner (java_library, java_binary, java_test); no
# adapter claims java yet (classification only, no supported claim). O31
# qualifies exact adapters later. M23 WP2 adds the kotlin class to the
# kotlin family on the same terms: each `.kt` source has one physical
# owner (kotlin_library, kotlin_binary, kotlin_test) over the
# pinned rules_kotlin kt_jvm_* rules, with same-compilation-unit `.java`
# sources owned alongside and classified java; no adapter claims kotlin
# yet (classification only, no supported claim). O31 qualifies exact
# adapters later. M23 WP2 adds the scala class to the scala family on the
# same terms: each `.scala` source has one physical owner
# (scala_library, scala_binary, scala_test) over the pinned
# rules_scala scala_* rules on the managed route (M22 O30 decision), with
# same-compilation-unit `.java` sources owned alongside and classified
# java; no adapter claims scala yet (classification only, no supported
# claim). O31 qualifies exact adapters later. M23 WP2 adds the csharp class
# to the csharp family on the same terms: each `.cs` source has one physical
# owner (csharp_library, csharp_binary, csharp_test) over the
# pinned rules_dotnet csharp_* rules; no adapter claims csharp yet
# (classification only, no supported claim). O31 qualifies exact adapters
# later. M23 WP2 adds the fsharp class to the fsharp family on the same
# terms: each `.fs`/`.fsi` source has one physical owner
# (fsharp_library, fsharp_binary, fsharp_test) over the pinned
# rules_dotnet fsharp_* rules with the shared Paket `paket.main` lock
# (FSharp.Core runtime); no adapter claims fsharp yet (classification only,
# no supported claim). O31 qualifies exact adapters later. M23 WP2 adds the
# ruby class to the ruby family and the powershell class to the powershell
# family on the same terms: both application foundations stay deferred
# beyond v1 per ADR 0019, so each source keeps its existing ownership (the
# file itself; no foundation wrapper lands here) while the retained
# RuboCop/StandardRB and PSScriptAnalyzer tool cohorts stay in force; no
# adapter claims ruby or powershell yet (classification only, no supported
# claim). O31 qualifies exact adapters later.
REAL_CLASS_TO_FAMILY = {
    "astro": "astro",
    "c": "cc",
    "cpp": "cc",
    "csharp": "csharp",
    "cuda": "cuda",
    "cue": "cue",
    "fsharp": "fsharp",
    "go": "go",
    "go_module": "go_module",
    "java": "java",
    "javascript": "javascript",
    "json": "json",
    "jsx": "javascript",
    "kotlin": "kotlin",
    "markdown": "markdown",
    "mdx": "mdx",
    "pkl": "pkl",
    "protobuf": "protobuf",
    "powershell": "powershell",
    "python": "python",
    "python_stub": "python",
    "qml": "qml",
    "ruby": "ruby",
    "rust": "rust",
    "scala": "scala",
    "shell": "shell",
    "starlark": "starlark",
    "svelte": "svelte",
    "terraform": "terraform",
    "toml": "toml",
    "typescript": "typescript",
    "tsx": "typescript",
    "vue": "vue",
    "jsonnet": "jsonnet",
    "yaml": "yaml",
}

def real_supported_classes(tool_id, capability):
    """Returns the sorted supported classes for one real tool/capability.

    Fails on an unknown tool ID during configuration or analysis, matching
    the tool-integrations contract. An unsupported capability returns [],
    so the stage is omitted and no empty action is created.
    """
    if tool_id not in REAL_ADAPTERS:
        fail("adapters: unknown tool '" + tool_id +
             "': not in the real adapter registry")
    return sorted(REAL_ADAPTERS[tool_id].get(capability, []))
