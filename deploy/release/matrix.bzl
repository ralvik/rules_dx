
RELEASE_MATRIX = [
    ("dx-linux-x86_64", "linux", "x86_64", "qualified-seed-built-here", "Seed host; built + verified in publish dry-run"),
    ("dx-linux-arm64", "linux", "arm64", "qualified-host-evidence", "Required; Platform-qualified plus per-host sbom-provenance (
    ("dx-macos-arm64", "macos", "arm64", "qualified-host-evidence", "Required; Platform-qualified plus per-host sbom-provenance (
    ("dx-windows-x86_64", "windows", "x86_64", "qualified-host-evidence", "Required; Platform-qualified plus per-host sbom-provenance (
]

def release_matrix_names():
    return [entry[0] for entry in RELEASE_MATRIX]

def release_matrix_status(name):
    for entry in RELEASE_MATRIX:
        if entry[0] == name:
            return entry[3]
    return ""

def release_matrix_error(name):
    if release_matrix_status(name) == "":
        return ("release_matrix: unknown artifact '" + str(name) +
                "': want one of " + ", ".join(release_matrix_names()))
    return ""

def release_matrix_unqualified():
    return [entry[0] for entry in RELEASE_MATRIX if not entry[3].startswith("qualified-")]
