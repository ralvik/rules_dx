"""SBOM + provenance generation for releases.

Contract: `docs/tools/tool-acquisition.md`, `docs/deploy/release-runbook.md`.
"""

def sbom_filenames(name):
    """Returns the deterministic (spdx, provenance) output names."""
    return (name + ".spdx.json", name + ".provenance.json")

def sbom_spdx_error(spdx_version):
    """Validates the SPDX document version."""
    if spdx_version != "SPDX-2.3":
        return ("sbom_release: invalid spdx_version '" + str(spdx_version) +
                "': want 'SPDX-2.3' (selected wire profile per issue #311)")
    return ""

def sbom_predicate_error(predicate):
    """Validates one in-toto predicate type."""
    if predicate != "https://slsa.dev/provenance/v1":
        return ("sbom_release: invalid predicate '" + str(predicate) +
                "': want 'https://slsa.dev/provenance/v1' (selected wire profile per issue #311)")
    return ""

# of these workflow identities, never an arbitrary string. The dry-run id
# serves the `sbom_demo` shape check only; real releases pass the release
# id explicitly. Anything else fails closed at analysis time.
SBOM_BUILDER_DRY_RUN = "https://github.com/ralvik/rules_dx/.github/workflows/publish-dry-run.yml"
SBOM_BUILDER_RELEASE = "https://github.com/ralvik/rules_dx/.github/workflows/release.yml"

def sbom_builder_error(builder_id):
    """Validates one SLSA builder id against the allowlist."""
    if builder_id == SBOM_BUILDER_DRY_RUN or builder_id == SBOM_BUILDER_RELEASE:
        return ""
    return ("sbom_release: invalid builder '" + str(builder_id) +
            "': want '" + SBOM_BUILDER_DRY_RUN + "' (dry-run demo only) or '" +
            SBOM_BUILDER_RELEASE + "' (owner-approved release)")

def sbom_release(name, artifact, package_name = "dx", supplier = "rules_dx", builder_id = "https://github.com/ralvik/rules_dx/.github/workflows/publish-dry-run.yml"):
    """Generates SPDX 2.3 JSON + SLSA provenance for one release artifact.

    Creates `<name>.spdx.json` (SPDX 2.3 document describing the artifact
    bytes + sha256) and `<name>.provenance.json` (in-toto Statement v1
    with the SLSA v1 predicate, subject digest = artifact sha256). Both
    are deterministic given the artifact bytes: the hermetic Rust generator
    records the sha256 at build time with no host toolchain.
    """
    spdx_err = sbom_spdx_error("SPDX-2.3")
    if spdx_err != "":
        fail(spdx_err + " (in " + native.package_name() + ":" + name + ")")
    pred_err = sbom_predicate_error("https://slsa.dev/provenance/v1")
    if pred_err != "":
        fail(pred_err + " (in " + native.package_name() + ":" + name + ")")
    builder_err = sbom_builder_error(builder_id)
    if builder_err != "":
        fail(builder_err + " (in " + native.package_name() + ":" + name + ")")
    (spdx, provenance) = sbom_filenames(name)
    src_target = artifact

    # SPDX: hermetic digest + deterministic JSON via Rust
    # (no host sha256sum/shasum/python3, no network, no Syft
    # dependency; Syft/CycloneDX output remains compatible input to the
    # same verify path when owners adopt it per the runbook).
    native.genrule(
        name = name + "_spdx",
        srcs = [src_target],
        outs = [spdx],
        tools = ["//deploy/release:sbom_spdx_gen"],
        cmd = "$(location //deploy/release:sbom_spdx_gen) $(location " + src_target + ") $(OUTS) \"" + package_name + "\" \"" + supplier + "\"",
    )

    # Provenance: in-toto Statement v1 + SLSA v1 predicate, subject =
    # artifact digest. Builder id is the validated allowlist selection
    # above: the dry-run default serves the `sbom_demo` shape check only;
    # real releases pass SBOM_BUILDER_RELEASE explicitly per runbook.
    native.genrule(
        name = name + "_provenance",
        srcs = [src_target],
        outs = [provenance],
        tools = ["//deploy/release:sbom_prov_gen"],
        cmd = "$(location //deploy/release:sbom_prov_gen) $(location " + src_target + ") $(OUTS) \"" + builder_id + "\"",
    )

    native.filegroup(
        name = name,
        srcs = [":" + name + "_spdx", ":" + name + "_provenance"],
    )
