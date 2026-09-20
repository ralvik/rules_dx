"""Full release matrix for `dx` standalone binaries.

Contract: `docs/deploy/release-runbook.md`.
"""

# (name, os, cpu, status, notes). Status is one of
# `qualified-seed-built-here` or `unqualified-per-issue-311`.
RELEASE_MATRIX = [
    ("dx-linux-x86_64", "linux", "x86_64", "qualified-seed-built-here", "Seed host; built + verified in publish dry-run"),
    ("dx-linux-arm64", "linux", "arm64", "unqualified-per-issue-311", "Required (ADR 0014); needs Linux arm64 host + toolchain evidence"),
    ("dx-macos-arm64", "macos", "arm64", "unqualified-per-issue-311", "Required (ADR 0014); needs macOS arm64 host + SDK evidence"),
    ("dx-macos-x86_64", "macos", "x86_64", "unqualified-per-issue-311", "Best-effort (ADR 0014); qualifies when a host is available"),
    ("dx-windows-x86_64", "windows", "x86_64", "unqualified-per-issue-311", "Required, backend blocked (ADR 0014); needs hermetic MSVC backend"),
]

def release_matrix_names():
    """Returns the ordered release artifact names."""
    return [entry[0] for entry in RELEASE_MATRIX]

def release_matrix_status(name):
    """Returns the qualification status for one matrix cell."""
    for entry in RELEASE_MATRIX:
        if entry[0] == name:
            return entry[3]
    return ""

def release_matrix_error(name):
    """Validates one matrix cell name."""
    if release_matrix_status(name) == "":
        return ("release_matrix: unknown artifact '" + str(name) +
                "': want one of " + ", ".join(release_matrix_names()))
    return ""

def release_matrix_unqualified():
    """Returns the names of cells still awaiting qualification."""
    return [entry[0] for entry in RELEASE_MATRIX if entry[3] != "qualified-seed-built-here"]
