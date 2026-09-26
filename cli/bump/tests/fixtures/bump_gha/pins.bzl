"""Bump GHA tag-to-SHA auto resolution pins.

Contract: `docs/cli/commands/audit-update-bazel.md#dx-bump`.
Fixture: `cli/bump/tests/fixtures/bump_gha/` via
`bazel run //tools/ci:bump_gha_qualification`.
"""

# Disposition: tags auto-resolve to SHA via the upstream GitHub releases
BUMP_GHA = "auto"

# Upstream registry client owning tag enumeration plus SHA resolution
# (never custom HTTP).
BUMP_GHA_CLIENT = "GitHub releases"

# Tag snapshots (injected owner/repo plus tag plus SHA, fetched via gh api
# in the scheduled bump.yml runner; planned in `dx_bump::gha`).
BUMP_GHA_CHECKOUT_SNAPSHOT = "actions/checkout v5 auto-resolves to SHA via upstream snapshot"
BUMP_GHA_CHECKOUT_SHA = "3d3c42e5aac5ba805825da76410c181273ba90b1"
BUMP_GHA_CACHE_SNAPSHOT = "actions/cache v4 auto-resolves to SHA via upstream snapshot"

# Shapes validate through version::parse (never custom version code).
BUMP_GHA_TAG_SHAPE = "tag via version::parse GitTag (e.g. v5)"
BUMP_GHA_SHA_SHAPE = "SHA via version::parse GitCommit 40/64-char hex"

# Resolution matches one snapshot before the file edit (one tag per run).
BUMP_GHA_RESOLVE = "resolve_tag matches owner/repo plus tag (one tag per run)"
BUMP_GHA_NEXT = "one tag per run (never batch)"

# Unknown tags fail closed (nothing widened, never invent a SHA).
BUMP_GHA_UNKNOWN = "unknown tag fails closed (nothing widened; never invent a SHA)"

# Direct tags need SHA resolution before the file edit (NeedsSha in request).
BUMP_GHA_NEEDS_SHA = "direct tag needs SHA resolution via upstream client before file edit"

# Rejected routes (never pinned as supported here).
REJECTED_MANUAL_SHA_ONLY = "manual SHA only rejected"
REJECTED_CUSTOM_HTTP = "custom HTTP rejected"
REJECTED_INVENTED_SHA = "invented SHA rejected"
REJECTED_PRIVATE_RESOLVER = "private resolver rejected"

# Honesty lines (never pinned as supported here).
NO_SUPPORTED = "no Supported claim"
SEED_ONLY = "qualified seed-only under issue #640"
