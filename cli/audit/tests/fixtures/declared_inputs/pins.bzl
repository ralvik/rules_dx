"""Audit curator declared-Bazel-inputs pins.

Contract: `docs/cli/commands/audit-update-bazel.md#dx-audit`.
Fixture: `cli/audit/tests/fixtures/declared_inputs/` via
`bazel run //tools/ci:audit_declared_inputs_qualification`.
"""

# Declared Bazel label for the committed curator file (same bytes the
# CLI reads from the workspace; Bazel-owned analysis declares it as an
# input so cache identity reflects the data actually analyzed).
CURATOR_LABEL = "//:audit_curator"

# Workspace-relative curator file (CLI read path; filegroup src).
CURATOR_REL = "licenses.toml"

# Advisory sets owning identified snapshots as declared analysis inputs.
CURATOR_ADVISORY_SETS = ["cargo", "npm", "maven", "nuget", "go"]

# Per-set snapshot plus identity rels (acquired, not committed; missing,
# invalid, or stale fails with advisory_refresh_failed, never clean).
CURATOR_SNAPSHOT_RELS = [
    ".dx/advisory/cargo.json",
    ".dx/advisory/cargo.meta.json",
    ".dx/advisory/npm.json",
    ".dx/advisory/npm.meta.json",
    ".dx/advisory/maven.json",
    ".dx/advisory/maven.meta.json",
    ".dx/advisory/nuget.json",
    ".dx/advisory/nuget.meta.json",
    ".dx/advisory/go.json",
    ".dx/advisory/go.meta.json",
]

# Per-package inventory shape is unchanged so NOTICE aggregation stays
# hermetic and cached (package plus set plus license plus versions plus
# text_present; see dx_audit::license_policy::LicenseInventory).
INVENTORY_SHAPE = ["package", "set", "license", "versions", "text_present"]

# NOTICE aggregation rides declared per-package inputs (hermetic
# notice_gen, byte-identical rebuilds, missing-notice-text fails).
NOTICE_HERMETIC = "deploy/release/notice.bzl notice_bundle hermetic"

# Semantics preserved: same policy evaluation, same advisory freshness
# plus fail-closed mapping, same SPDX/SARIF reporting, same exit codes.
SEMANTICS_PRESERVED = "same policy plus freshness plus reporting"

# Rejected routes (never silent fallbacks here).
REJECTED_STALE_FALLBACK = "stale snapshot fallback rejected"
REJECTED_UNTRACKED_DB = "untracked live database rejected"
REJECTED_SHAPE_DRIFT = "per-package shape drift rejected"

SEED_ONLY = "qualified seed-only under issue #812"
NO_SUPPORTED = "no Supported claim"
