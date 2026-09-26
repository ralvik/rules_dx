"""dx_tools use_repo inventory. Contract: docs/tools/tool-acquisition.md."""

# Checked-in inventory of the `dx_tools` extension repos re-exported by the
# root MODULE.bazel `use_repo` block. Single source stays the artifact
# metadata loaded by `extension.bzl`; `//quality/artifacts:metadata`
# fails when this list drifts from the derived `dx_<tool>_<os>_<cpu>`
# names, and MODULE.bazel must match this list (drift breaks acquisition).
# Add a tool here only with its four platform
# metadata files plus extension coverage (macOS x86_64 removed per #976).
DX_TOOL_REPOS = [
    "dx_biome_linux_arm64",
    "dx_biome_linux_x86_64",
    "dx_biome_macos_arm64",
    "dx_biome_windows_x86_64",
    "dx_buildifier_linux_arm64",
    "dx_buildifier_linux_x86_64",
    "dx_buildifier_macos_arm64",
    "dx_buildifier_windows_x86_64",
    "dx_gitleaks_linux_arm64",
    "dx_gitleaks_linux_x86_64",
    "dx_gitleaks_macos_arm64",
    "dx_gitleaks_windows_x86_64",
    "dx_ruff_linux_arm64",
    "dx_ruff_linux_x86_64",
    "dx_ruff_macos_arm64",
    "dx_ruff_windows_x86_64",
    "dx_taplo_linux_arm64",
    "dx_taplo_linux_x86_64",
    "dx_taplo_macos_arm64",
    "dx_taplo_windows_x86_64",
    "dx_ty_linux_arm64",
    "dx_ty_linux_x86_64",
    "dx_ty_macos_arm64",
    "dx_ty_windows_x86_64",
    "dx_vale_linux_arm64",
    "dx_vale_linux_x86_64",
    "dx_vale_macos_arm64",
    "dx_vale_windows_x86_64",
]

# Execution-platform hub re-exported alongside the tool repos.
DX_TOOL_HUB = "dx_tools"
