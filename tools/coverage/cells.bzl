"""Coverage cell registry (single source)."""

# Four qualified cells, same first-party scope. Each cell gates its own
COVERAGE_CELLS = {
    "seed-linux_x86_64": "tools/coverage/seed-inventory.txt",
    "linux_arm64": "tools/coverage/arm64-inventory.txt",
    "macos_arm64": "tools/coverage/macos-arm64-inventory.txt",
    "windows_x86_64": "tools/coverage/windows-x86_64-inventory.txt",
}

# Seed cell owning the exact-gate scope every other cell follows.
COVERAGE_SEED_CELL = "seed-linux_x86_64"

# Checked-in snapshot files owned by this registry.
INVENTORY_FILES = [
    "arm64-inventory.txt",
    "cells.txt",
    "macos-arm64-inventory.txt",
    "seed-inventory.txt",
    "windows-x86_64-inventory.txt",
]

def coverage_cells(name):
    """Declare the per-cell inventory filegroup from the registry."""
    native.filegroup(
        name = name,
        srcs = INVENTORY_FILES,
    )
