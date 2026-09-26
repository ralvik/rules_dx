"""Bump GHA tag-to-SHA auto resolution pins."""

BUMP_GHA = "auto"

BUMP_GHA_CLIENT = "GitHub releases"

BUMP_GHA_CHECKOUT_SNAPSHOT = "actions/checkout v5 auto-resolves to SHA via upstream snapshot"
BUMP_GHA_CHECKOUT_SHA = "3d3c42e5aac5ba805825da76410c181273ba90b1"
BUMP_GHA_CACHE_SNAPSHOT = "actions/cache v4 auto-resolves to SHA via upstream snapshot"

BUMP_GHA_TAG_SHAPE = "tag via version::parse GitTag (e.g. v5)"
BUMP_GHA_SHA_SHAPE = "SHA via version::parse GitCommit 40/64-char hex"

BUMP_GHA_RESOLVE = "resolve_tag matches owner/repo plus tag (one tag per run)"
BUMP_GHA_NEXT = "one tag per run (never batch)"

BUMP_GHA_UNKNOWN = "unknown tag fails closed (nothing widened; never invent a SHA)"

BUMP_GHA_NEEDS_SHA = "direct tag needs SHA resolution via upstream client before file edit"

REJECTED_MANUAL_SHA_ONLY = "manual SHA only rejected"
REJECTED_CUSTOM_HTTP = "custom HTTP rejected"
REJECTED_INVENTED_SHA = "invented SHA rejected"
REJECTED_PRIVATE_RESOLVER = "private resolver rejected"

NO_SUPPORTED = "no Supported claim"
SEED_ONLY = "qualified seed-only"
