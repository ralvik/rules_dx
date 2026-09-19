"""Unit tests for SBOM + provenance generation (issue #311).

Pins the selected wire profile (SPDX 2.3, SLSA v1) and the deterministic
output naming. Content binding (subject digest == artifact sha256) is
proven by the `sbom_verify` sh_tests in BUILD, not here.
"""

load("//libs/starlark:defs.bzl", "expect_equal", "starlark_test")
load(":sbom.bzl", "sbom_filenames", "sbom_predicate_error", "sbom_spdx_error")

def sbom_unit_tests(name):
    starlark_test(
        name = name,
        mode = "unit",
        checks = [
            expect_equal(
                "sbom_filenames pins spdx and provenance names",
                sbom_filenames("release"),
                ("release.spdx.json", "release.provenance.json"),
            ),
            expect_equal(
                "sbom_filenames derives names per instance",
                sbom_filenames("dx_standalone"),
                ("dx_standalone.spdx.json", "dx_standalone.provenance.json"),
            ),
            expect_equal(
                "sbom_spdx_error accepts SPDX-2.3",
                sbom_spdx_error("SPDX-2.3"),
                "",
            ),
            expect_equal(
                "sbom_spdx_error rejects SPDX-3",
                sbom_spdx_error("SPDX-3.0"),
                "sbom_release: invalid spdx_version 'SPDX-3.0': want 'SPDX-2.3' (selected wire profile per issue #311)",
            ),
            expect_equal(
                "sbom_predicate_error accepts SLSA v1",
                sbom_predicate_error("https://slsa.dev/provenance/v1"),
                "",
            ),
            expect_equal(
                "sbom_predicate_error rejects bare SLSA",
                sbom_predicate_error("https://slsa.dev/provenance"),
                "sbom_release: invalid predicate 'https://slsa.dev/provenance': want 'https://slsa.dev/provenance/v1' (selected wire profile per issue #311)",
            ),
        ],
    )
