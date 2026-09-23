"""Full release matrix for `dx` standalone binaries.

Contract: `docs/deploy/release-runbook.md`.
"""

# (name, os, cpu, status, notes). Status is one of
# `qualified-seed-built-here` or `qualified-host-evidence`.
# macOS x86_64 is Not planned, never planned for support (issue #976);
# it carries no matrix cell.
RELEASE_MATRIX = [
    ("dx-linux-x86_64", "linux", "x86_64", "qualified-seed-built-here", "Seed host; built + verified in publish dry-run"),
    ("dx-linux-arm64", "linux", "arm64", "qualified-host-evidence", "Required (ADR 0014); Platform-qualified plus per-host sbom-provenance (See: docs/product/support-matrix.md)"),
    ("dx-macos-arm64", "macos", "arm64", "qualified-host-evidence", "Required (ADR 0014); Platform-qualified plus per-host sbom-provenance (See: docs/product/support-matrix.md)"),
    ("dx-windows-x86_64", "windows", "x86_64", "qualified-host-evidence", "Required (ADR 0014); Platform-qualified plus per-host sbom-provenance (See: docs/product/support-matrix.md)"),
]

def release_matrix_names():
    """Returns the ordered release artifact names."""
    return [entry[0] for entry in RELEASE_MATRIX]

def release_matrix_status(name):
    """Returns the qualification status for one matrix cell.

    Args:
      name: Matrix artifact name (for example `dx-linux-x86_64`).

    Returns:
      Status string for the cell, or empty when unknown.
    """
    for entry in RELEASE_MATRIX:
        if entry[0] == name:
            return entry[3]
    return ""

def release_matrix_error(name):
    """Validates one matrix cell name.

    Args:
      name: Candidate matrix artifact name.

    Returns:
      Empty string when known, else an actionable error message.
    """
    if release_matrix_status(name) == "":
        return ("release_matrix: unknown artifact '" + str(name) +
                "': want one of " + ", ".join(release_matrix_names()))
    return ""

def release_matrix_unqualified():
    """Returns the names of cells still awaiting qualification."""
    return [entry[0] for entry in RELEASE_MATRIX if not entry[3].startswith("qualified-")]
