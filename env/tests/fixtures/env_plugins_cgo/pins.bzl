"""Third-party env plugin model plus Go cgo exception pins (issue #587).

Contract: `docs/environments/environment.md#public-tool-api`,
`docs/environments/environment.md#ownership-and-refresh`,
`docs/environments/environment.md#test-requirements`.
Fixture: `env/tests/fixtures/env_plugins_cgo/` via
`bazel run //tools/ci:env_plugins_cgo_qualification`.

Decides the two #587 slices that issue #506 explicitly does not cover:
third-party language-integration (persistent-environment) plugins get a
deferred-past-v1 design decision with owner plus acceptance criteria
instead of an owner-less deferral, and Go cgo completion gets an explicit
exception boundary instead of a bare out-of-scope line. Env only; no
PATH-tool collision rule change. Seed only: no Supported claim.
"""

# Plugin model stays deferred past v1 with an explicit design owner.
PLUGIN_DISPOSITION = "deferred past v1"
PLUGIN_DEFERRED_NOTE = "third-party language-integration plugins are deferred past v1"
PLUGIN_OWNER = "//env plus //cli/env"
PLUGIN_NO_MODEL = "no v1 persistent-environment plugin model"
PLUGIN_NO_EXTENSION_POINT = "no EnvironmentInfo extension point"

# Repurposing EnvironmentInfo as a persistent-environment plugin API is wont-fix in v1.
PLUGIN_ENV_INFO_WONT_FIX = "EnvironmentInfo stays PATH-tool-only"
PLUGIN_ENV_INFO_NOT_PLUGIN_API = "is not a persistent-environment plugin API"
PLUGIN_NO_PRIVATE_PATH = "no private first-party contribution path"

# Acceptance criteria for any post-v1 plugin proposal.
PLUGIN_CRITERIA_PROVIDER_DERIVED = "provider-derived symlink-only plans"
PLUGIN_CRITERIA_SELECTION = "managed identity/commit/reuse selection"
PLUGIN_CRITERIA_BOUNDARY = "PATH-tools-only boundary"
PLUGIN_CRITERIA_NO_PRIVATE = "no private path"
PLUGIN_CRITERIA_QUALIFICATION = "fixture-pinned qualification with no Supported claim"

# Go IDE boundary stays upstream GOPACKAGESDRIVER on the pinned rules_go.
GO_DRIVER = "GOPACKAGESDRIVER"
GO_DRIVER_UPSTREAM = "rules_go 0.63.0"
GO_DRIVER_DOC = "https://github.com/bazel-contrib/rules_go/blob/v0.63.0/docs/editors.md"
GO_SUPPORTED = "pure-Go packages"
GO_SUPPORTED_CONSTRAINTS = "declared build constraints and platform source selection"
GO_SEPARATE_BASE = "Use a separate IDE output base"
GO_NO_MUTATION = "do not mutate the selected environment"
GO_NO_SNAPSHOT = "rather than creating a static package snapshot"
GO_NO_REPLACEMENT_GRAPH = "or replacement package graph"
GO_PROPAGATE_FAILURES = "propagate failures rather than reporting a successful incomplete package graph"

# cgo completion plus cgo diagnostics stay the explicit out-of-scope exception.
CGO_OUT_OF_SCOPE = "cgo completion is out of scope"
CGO_EXCEPTION_NOTE = "this exception does not admit the complete Go foundation"
CGO_UPSTREAM_NON_GUARANTEE = "upstream does not guarantee cgo completion"
CGO_RECORD_GAPS = "record cgo and platform gaps rather than claiming generic IDE parity"

# Rejected substitutes (never accepted as the #587 resolution).
REJECTED_PLUGIN_CLAIM = "third-party plugin claim"
REJECTED_PRIVATE_PATH = "private first-party contribution path"
REJECTED_STATIC_SNAPSHOT = "static package snapshot"
REJECTED_REPLACEMENT_GRAPH = "replacement package graph"
REJECTED_AMBIENT_FALLBACK = "ambient Go tools fallback"
REJECTED_GENERIC_PARITY = "generic IDE parity"
REJECTED_CGO_COMPLETION_CLAIM = "cgo completion claim"

# Honesty lines (never pinned as supported here).
NO_SUPPORTED = "no Supported claim"
SEED_ONLY = "qualified seed-only under issue #587"
OWNED_GAPS_NOTE = "platform plus consumer plus release evidence stays owned gap"
