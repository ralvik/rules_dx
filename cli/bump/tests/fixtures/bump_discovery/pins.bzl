"""Bump discovery outdated enumeration pins.

Contract: `docs/cli/commands/audit-update-bazel.md#dx-bump`.
Fixture: `cli/bump/tests/fixtures/bump_discovery/` via
`bazel run //tools/ci:bump_discovery_qualification`.
"""

# Disposition: scheduled discovery enumerates outdated via upstream
BUMP_DISCOVERY = "enumerate"

# Per-set upstream registry clients (never custom HTTP).
BUMP_DISCOVERY_BAZEL = "BCR"
BUMP_DISCOVERY_CARGO = "crates.io"
BUMP_DISCOVERY_NPM = "npm registry"
BUMP_DISCOVERY_GO = "Go proxy"
BUMP_DISCOVERY_MAVEN = "Maven Central"
BUMP_DISCOVERY_NUGET = "NuGet"
BUMP_DISCOVERY_GHA = "GitHub releases"

# Discovery policy (planned in `dx_bump::discovery`).
BUMP_DISCOVERY_STABLE = "stable only"
BUMP_DISCOVERY_PRERELEASE = "prerelease follows upstream resolver and project config"
BUMP_DISCOVERY_ORDER = "upstream semver comparison"
BUMP_DISCOVERY_TRANSITIVES = "transitives stay resolver-governed"
BUMP_DISCOVERY_NEXT = "first in selector order (never batch)"

# Selector shapes (declared plus available snapshot, semver sets only).
BUMP_DISCOVERY_CARGO_SELECTOR = "cargo:anyhow outdated when crates.io reports stable above current"
BUMP_DISCOVERY_NPM_SELECTOR = "npm:jest outdated when npm registry reports stable above current"
BUMP_DISCOVERY_GO_SELECTOR = "go:example.com/mod outdated when Go proxy reports stable above current"
BUMP_DISCOVERY_MAVEN_SELECTOR = "maven:junit:junit outdated when Maven Central reports stable above current"
BUMP_DISCOVERY_NUGET_SELECTOR = "nuget:FSharp.Core outdated when NuGet reports stable above current"
BUMP_DISCOVERY_BAZEL_SELECTOR = "bazel:rules_rust outdated when BCR reports stable above current"

# GitHub Actions tags enumerate via GitHub releases but need SHA
BUMP_DISCOVERY_GHA_NOTE = "github-actions tags need SHA resolution via the upstream GitHub releases client"

# Up-to-date means no candidate (nothing widened for the selector).
BUMP_DISCOVERY_UP_TO_DATE = "up-to-date has no candidate"

# Rejected routes (never pinned as supported here).
REJECTED_MANUAL_SELECTOR_ONLY = "manual selector only rejected"
REJECTED_CUSTOM_HTTP = "custom HTTP rejected"
REJECTED_PRIVATE_RESOLVER = "private resolver rejected"

# Honesty lines (never pinned as supported here).
NO_SUPPORTED = "no Supported claim"
SEED_ONLY = "qualified seed-only under issue #639"
