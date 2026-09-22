"""Coverage cell registry (single source). Contract: docs/testing/README.md#coverage."""

# Seven qualified cells, same first-party scope. Each cell gates its own
# combined LCOV report through `coverage_bin` against its versioned
# inventory; no cross-cell union, no averaged percentages, no rounding up.
# Every non-seed inventory carries the seed eligible scope line-for-line
# (`//tools/ci:coverage_qualification` enforces the equality); only the
# inventory path differs per cell, so each cell gates its own report
# separately. cells.txt plus the inventories are the checked-in snapshot
# of this registry (same pattern as the bazelrc preset parity); never edit
# them without updating the registry first.
COVERAGE_CELLS = {
    "seed-linux_x86_64": "tools/coverage/seed-inventory.txt",
    "linux_arm64": "tools/coverage/arm64-inventory.txt",
    "linux_x86_64_musl": "tools/coverage/musl-x86_64-inventory.txt",
    "linux_arm64_musl": "tools/coverage/musl-arm64-inventory.txt",
    "macos_arm64": "tools/coverage/macos-arm64-inventory.txt",
    "macos_x86_64": "tools/coverage/macos-x86_64-inventory.txt",
    "windows_x86_64": "tools/coverage/windows-x86_64-inventory.txt",
}

# Seed cell owning the exact-gate scope every other cell follows.
COVERAGE_SEED_CELL = "seed-linux_x86_64"

# Checked-in snapshot files owned by this registry.
INVENTORY_FILES = [
    "arm64-inventory.txt",
    "cells.txt",
    "macos-arm64-inventory.txt",
    "macos-x86_64-inventory.txt",
    "musl-arm64-inventory.txt",
    "musl-x86_64-inventory.txt",
    "seed-inventory.txt",
    "windows-x86_64-inventory.txt",
]

def coverage_cells(name):
    """Declare the per-cell inventory filegroup from the registry."""
    native.filegroup(
        name = name,
        srcs = INVENTORY_FILES,
    )
