"""SBOM + provenance generation for releases (issue #311).

`sbom_release` is the SBOM/provenance macro: it derives deterministic
SPDX 2.3 JSON plus SLSA Build Provenance v1 (in-toto Statement v1) from
one release artifact with host tools only (`sha256sum`/`shasum`,
`python3`), no new module dependencies, no registry, no credentials.

Wire profile follows docs/tools/tool-acquisition.md (provisional
candidates, now selected for releases): SPDX predicate
`https://spdx.dev/Document/v2.3`, SLSA predicate
`https://slsa.dev/provenance/v1`, statement
`https://in-toto.io/Statement/v1` with the exact published artifact
digest as subject. The embedded-vs-detached boundary from Artifact
Identity And Metadata holds: the SBOM describes the artifact bytes;
final-archive attestations over those exact bytes are produced by the
signing macro (`signing.bzl`), never here. Changing the artifact
invalidates the SBOM and its attestations.

Contract: `docs/deploy/release-runbook.md`. SBOM bundles verify through
`//deploy/install:dx_verify --sbom/--sbom-bundle` (same cosign path as
the binary).
"""

def sbom_filenames(name):
    """Returns the deterministic (spdx, provenance) output names.

    Args:
      name: the `sbom_release` instance name.

    Returns:
      A `(spdx, provenance)` string tuple, for example
      `("release.spdx.json", "release.provenance.json")`.
    """
    return (name + ".spdx.json", name + ".provenance.json")

def sbom_spdx_error(spdx_version):
    """Validates the SPDX document version.

    Args:
      spdx_version: candidate SPDX version string.

    Returns:
      "" when valid, else the failure reason.
    """
    if spdx_version != "SPDX-2.3":
        return ("sbom_release: invalid spdx_version '" + str(spdx_version) +
                "': want 'SPDX-2.3' (selected wire profile per issue #311)")
    return ""

def sbom_predicate_error(predicate):
    """Validates one in-toto predicate type.

    Args:
      predicate: candidate predicate URI.

    Returns:
      "" when valid, else the failure reason.
    """
    if predicate != "https://slsa.dev/provenance/v1":
        return ("sbom_release: invalid predicate '" + str(predicate) +
                "': want 'https://slsa.dev/provenance/v1' (selected wire profile per issue #311)")
    return ""

def sbom_release(name, artifact, package_name = "dx", supplier = "rules_dx", builder_id = "https://github.com/ralvik/rules_dx/.github/workflows/publish-dry-run.yml"):
    """Generates SPDX 2.3 JSON + SLSA provenance for one release artifact.

    Creates `<name>.spdx.json` (SPDX 2.3 document describing the artifact
    bytes + sha256) and `<name>.provenance.json` (in-toto Statement v1
    with the SLSA v1 predicate, subject digest = artifact sha256). Both
    are deterministic given the artifact bytes: the genrule records the
    sha256 at build time with host tools only.

    Args:
      name: instance name.
      artifact: label of the single release artifact file (tarball/binary).
      package_name: SPDX package name (default `dx`).
      supplier: SPDX supplier string.
      builder_id: SLSA `runDetails.builder.id`.
    """
    spdx_err = sbom_spdx_error("SPDX-2.3")
    if spdx_err != "":
        fail(spdx_err + " (in " + native.package_name() + ":" + name + ")")
    pred_err = sbom_predicate_error("https://slsa.dev/provenance/v1")
    if pred_err != "":
        fail(pred_err + " (in " + native.package_name() + ":" + name + ")")
    (spdx, provenance) = sbom_filenames(name)
    src_target = artifact

    # SPDX: host sha256 + python3 emit deterministic JSON (no network,
    # no Syft dependency; Syft/CycloneDX output remains compatible input
    # to the same verify path when owners adopt it per the runbook).
    # genrule `cmd` uses Make expansion: `$(location ...)` stays
    # single-`$`, shell `$` is escaped as `$$`.
    native.genrule(
        name = name + "_spdx",
        srcs = [src_target],
        outs = [spdx],
        cmd = "set -euo pipefail; " +
              "src=\"$(location " + src_target + ")\"; " +
              "out=\"$(OUTS)\"; " +
              "if command -v sha256sum >/dev/null 2>&1; then d=\"$$(sha256sum \"$$src\" | cut -d' ' -f1)\"; " +
              "else d=\"$$(shasum -a 256 \"$$src\" | cut -d' ' -f1)\"; fi; " +
              "b=\"$$(basename \"$$src\")\"; " +
              "python3 - \"$${src}\" \"$${out}\" \"$${d}\" \"$${b}\" \"" + package_name + "\" \"" + supplier + "\" <<'EOF'\n" +
              "import json,sys\n" +
              "src,out,digest,base,pkg,sup = sys.argv[1:7]\n" +
              "doc={\"spdxVersion\":\"SPDX-2.3\",\"dataLicense\":\"CC0-1.0\",\"SPDXID\":\"SPDXRef-DOCUMENT\",\"name\":pkg+\"-\"+base,\"documentNamespace\":\"https://github.com/ralvik/rules_dx/releases/\"+base+\"-\"+digest,\"creationInfo\":{\"created\":\"1970-01-01T00:00:00Z\",\"creators\":[\"Tool: rules_dx-sbom-1.0\"]},\"packages\":[{\"SPDXID\":\"SPDXRef-Package\",\"name\":pkg,\"supplier\":\"Organization: \"+sup,\"downloadLocation\":\"NOASSERTION\",\"filesAnalyzed\":False,\"verificationCode\":{\"packageVerificationCodeValue\":digest},\"checksums\":[{\"algorithm\":\"SHA256\",\"checksumValue\":digest}],\"externalRefs\":[{\"referenceCategory\":\"PACKAGE-MANAGER\",\"referenceType\":\"purl\",\"referenceLocator\":\"pkg:generic/\"+pkg+\"@\"+digest}]}],\"files\":[{\"SPDXID\":\"SPDXRef-File\",\"fileName\":base,\"checksums\":[{\"algorithm\":\"SHA256\",\"checksumValue\":digest}]}]}\n" +
              "open(out,\"w\",encoding=\"utf-8\").write(json.dumps(doc,indent=2,sort_keys=True)+\"\\n\")\n" +
              "EOF",
    )

    # Provenance: in-toto Statement v1 + SLSA v1 predicate, subject =
    # artifact digest. Builder id is the dry-run workflow; real releases
    # replace it with the owner-approved release workflow id per runbook.
    native.genrule(
        name = name + "_provenance",
        srcs = [src_target],
        outs = [provenance],
        cmd = "set -euo pipefail; " +
              "src=\"$(location " + src_target + ")\"; " +
              "out=\"$(OUTS)\"; " +
              "if command -v sha256sum >/dev/null 2>&1; then d=\"$$(sha256sum \"$$src\" | cut -d' ' -f1)\"; " +
              "else d=\"$$(shasum -a 256 \"$$src\" | cut -d' ' -f1)\"; fi; " +
              "b=\"$$(basename \"$$src\")\"; " +
              "python3 - \"$${out}\" \"$${d}\" \"$${b}\" \"" + builder_id + "\" <<'EOF'\n" +
              "import json,sys\n" +
              "out,digest,base,builder = sys.argv[1:5]\n" +
              "stmt={\"_type\":\"https://in-toto.io/Statement/v1\",\"subject\":[{\"name\":base,\"digest\":{\"sha256\":digest}}],\"predicateType\":\"https://slsa.dev/provenance/v1\",\"predicate\":{\"buildDefinition\":{\"buildType\":\"https://github.com/ralvik/rules_dx/release@v1\",\"externalParameters\":{\"artifact\":base}},\"runDetails\":{\"builder\":{\"id\":builder},\"metadata\":{\"invocationId\":\"dry-run\"}}}}\n" +
              "open(out,\"w\",encoding=\"utf-8\").write(json.dumps(stmt,indent=2,sort_keys=True)+\"\\n\")\n" +
              "EOF",
    )

    native.filegroup(
        name = name,
        srcs = [":" + name + "_spdx", ":" + name + "_provenance"],
    )
