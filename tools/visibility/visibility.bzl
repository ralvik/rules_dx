"""Package-boundary visibility contract (single source).

Contract: `docs/contributing/build-conventions.md#visibility`.
"""

# Scope spellings (buildifier-sorted; BUILD files load these by name, never
# re-spell the literals, so the allowlists below stay the only copy).

VIS_PUBLIC = ["//visibility:public"]

VIS_PRIVATE = ["//visibility:private"]

VIS_INTERNAL = ["//:__subpackages__"]

# Rust implementation leaves: visible inside //cli only. Consumers use the
# //dx facade plus the //cli/cli:dx and //cli/env:env binaries, never the
# libraries directly (see docs/architecture/facade.md).
VIS_CLI = [
    "//cli:__pkg__",
    "//cli:__subpackages__",
]

# Shared leaves: VIS_CLI plus exactly the observed cross-group consumers
# (each grant has at least one in-repo dependent; see the guard
# //tools/ci:visibility_guards). No new entries without a dependent.
VIS_CLI_ENV_QUALITY = VIS_CLI + [
    "//env:__subpackages__",
    "//quality:__subpackages__",
]

VIS_CLI_GENERATION_QUALITY = VIS_CLI + [
    "//generation:__subpackages__",
    "//quality:__subpackages__",
]

VIS_CLI_DEPLOY_QUALITY = VIS_CLI + [
    "//deploy:__subpackages__",
    "//quality:__subpackages__",
]

VIS_CLI_TOOLS = VIS_CLI + [
    "//tools:__subpackages__",
]

VIS_CLI_ENV_GENERATION_QUALITY = VIS_CLI + [
    "//env:__subpackages__",
    "//generation:__subpackages__",
    "//quality:__subpackages__",
]

VIS_CLI_WIDE = VIS_CLI + [
    "//docs:__subpackages__",
    "//env:__subpackages__",
    "//generation:__subpackages__",
    "//quality:__subpackages__",
]

VIS_CLI_DEPLOY = VIS_CLI + [
    "//deploy:__subpackages__",
]

VIS_CLI_DOCS_GENERATION_QUALITY = VIS_CLI + [
    "//docs:__subpackages__",
    "//generation:__subpackages__",
    "//quality:__subpackages__",
]

# Plan-shard libraries: env_shard serves the //env registry root (the
# rust_env_shard target attribute) plus //cli; codegen_shard also serves
# //generation (the dx_codegen_shard/prost_codegen_shard attr defaults
# declared in generation/codegen.bzl).
VIS_ENV_SHARD = VIS_CLI + [
    "//env:__pkg__",
    "//env:__subpackages__",
]

VIS_CLI_GENERATION = VIS_CLI + [
    "//generation:__pkg__",
]

# Quality leaves: visible inside //quality only, except markdown (serves
# the //env registry tool) and result (serves //cli).
VIS_QUALITY = [
    "//quality:__pkg__",
    "//quality:__subpackages__",
]

VIS_QUALITY_ENV = [
    "//env:__pkg__",
] + VIS_QUALITY

VIS_QUALITY_CLI = VIS_CLI + VIS_QUALITY

# External API: packages whose default is public. Language rules wrappers
# (loaded from //<lang>/rules:defs.bzl), the //config, //env, //generation,
# //quality roots, plus the //deploy and //modules entry points.
PUBLIC_PACKAGES = [
    "astro/rules",
    "cc/rules",
    "config",
    "csharp/rules",
    "deploy/offline",
    "deploy/rules",
    "env",
    "fsharp/rules",
    "generation",
    "go/rules",
    "java/rules",
    "javascript/rules",
    "kotlin/rules",
    "mdx/rules",
    "modules",
    "powershell/rules",
    "python/rules",
    "quality",
    "ruby/rules",
    "rust/rules",
    "scala/rules",
    "svelte/rules",
    "typescript/rules",
    "vue/rules",
]

# Focused per-language environment plans: no cross-package consumers, so
# the default stays private.
PRIVATE_ENV_PACKAGES = [
    "astro/env",
    "cc/env",
    "csharp/env",
    "fsharp/env",
    "go/env",
    "java/env",
    "javascript/env",
    "kotlin/env",
    "mdx/env",
    "powershell/env",
    "python/env",
    "ruby/env",
    "rust/env",
    "scala/env",
    "svelte/env",
    "typescript/env",
    "vue/env",
]

# Generated-code infrastructure: private with narrow explicit grants to
# //gazelle/dispatch and //dx (see each package).
PRIVATE_GAZELLE_PACKAGES = [
    "gazelle/astro",
    "gazelle/cc",
    "gazelle/csharp",
    "gazelle/dispatch",
    "gazelle/fsharp",
    "gazelle/go",
    "gazelle/java",
    "gazelle/javascript",
    "gazelle/kotlin",
    "gazelle/mdx",
    "gazelle/mixed",
    "gazelle/python",
    "gazelle/ruby",
    "gazelle/rust",
    "gazelle/scala",
    "gazelle/svelte",
    "gazelle/typescript",
    "gazelle/vue",
]

# Consumer facade: private default with explicit public exports only (see
# //tools/ci:dx_facade_qualification).
DX_FACADE_PACKAGE = "dx"

# Scoped packages: package directory to the scope constant it must load.
# Every multi-entry default_visibility in the tree appears here; anywhere
# else a multi-entry literal is a guard failure.
SCOPED_PACKAGES = {
    "cli/adopt": "VIS_CLI",
    "cli/apply": "VIS_CLI",
    "cli/atomic_fs": "VIS_CLI_ENV_QUALITY",
    "cli/audit": "VIS_CLI",
    "cli/bep": "VIS_CLI",
    "cli/bump": "VIS_CLI",
    "cli/ci": "VIS_CLI",
    "cli/clean": "VIS_CLI",
    "cli/cli": "VIS_CLI",
    "cli/codegen": "VIS_CLI",
    "cli/diff": "VIS_CLI",
    "cli/digest": "VIS_CLI_GENERATION_QUALITY",
    "cli/docgen": "VIS_CLI",
    "cli/env": "VIS_CLI",
    "cli/env_plan": "VIS_CLI",
    "cli/fingerprint": "VIS_CLI_DEPLOY_QUALITY",
    "cli/lcov": "VIS_CLI_TOOLS",
    "cli/output": "VIS_CLI_ENV_GENERATION_QUALITY",
    "cli/path": "VIS_CLI_WIDE",
    "cli/process": "VIS_CLI_DEPLOY",
    "cli/proto_validate": "VIS_CLI_WIDE",
    "cli/qualification": "VIS_CLI",
    "cli/roots": "VIS_CLI",
    "cli/schema": "VIS_CLI_DOCS_GENERATION_QUALITY",
    "cli/setup": "VIS_CLI",
    "cli/test_scratch": "VIS_CLI",
    "cli/update": "VIS_CLI",
    "env/env_shard": "VIS_ENV_SHARD",
    "generation/codegen_shard": "VIS_CLI_GENERATION",
    "generation/result": "VIS_CLI",
    "quality/adapter": "VIS_QUALITY",
    "quality/artifacts": "VIS_QUALITY",
    "quality/evaluator": "VIS_QUALITY",
    "quality/markdown": "VIS_QUALITY_ENV",
    "quality/result": "VIS_QUALITY_CLI",
    "quality/runner": "VIS_QUALITY",
    "quality/testdata": "VIS_QUALITY",
}

# Attribute-level public targets on otherwise scoped packages, plus the
# public-package binaries that carry an explicit export for documentation:
# package directory to the target names allowed an explicit public
# visibility. Binaries (plus the man-page doc) are the only public Rust
# surface; the //dx entries are the Starlark facade exports; the
# //deploy/rules entries are the hermetic archive tools consumed as
# genrule tools by downstream macros.
EXPLICIT_PUBLIC_TARGETS = {
    "cli/cli": ["dx", "man_pages"],
    "cli/env": ["env"],
    "deploy/rules": ["archiver", "dx_deploy_tools", "hasher"],
    "dx": ["codegen", "config", "env", "generate", "generate_check"],
}

# Packages allowed an explicit public exports_files (Cargo.toml for the
# workspace build; visibility-exempt loads otherwise).
EXPLICIT_PUBLIC_EXPORTS = [
    "cli/cli",
    "cli/env",
]

# Target-level multi-entry grants: [package, rule kind, extra scope]. Only
# the CI-owned fixtures below may widen a target beyond its package
# default (the CI harnesses consume them as data); every other target
# visibility stays a singleton.
SCOPED_TARGET_GRANTS = [
    ["quality/artifacts", "exports_files", "//tools/ci:__pkg__"],
    ["quality/testdata", "exports_files", "//tools/ci:__pkg__"],
]

# Layering: [consumer package prefix, forbidden label prefix, exempt
# labels]. The //dx facade sits above the //cli implementation, which sits
# above the registries; //tools is a downstream consumer of //cli leaves,
# never a dependency of them. Exemptions are fixtures or shell-contract
# data with their own guards.
LAYER_FORBIDDEN_DEPS = [
    ["cli/", "//dx:", ["//dx:codegen", "//dx:env"]],
    ["cli/", "//docs:", []],
    ["cli/", "//tools/", ["//tools/sh:bootstrap", "//tools/sh:lib"]],
    ["dx/", "//tools/", []],
    ["env/", "//dx:", []],
    ["gazelle/", "//dx:", []],
    ["generation/", "//dx:", []],
    ["quality/", "//dx:", []],
    ["tools/", "//dx:", []],
]

def is_public_package(pkg):
    """Reports whether a package directory carries the public default."""
    return pkg in PUBLIC_PACKAGES

def is_private_package(pkg):
    """Reports whether a package directory carries the private default."""
    return pkg in PRIVATE_ENV_PACKAGES or pkg in PRIVATE_GAZELLE_PACKAGES or pkg == DX_FACADE_PACKAGE

def scoped_constant(pkg):
    """Returns the scope constant a scoped package must load ("" when none)."""
    return SCOPED_PACKAGES.get(pkg, "")
