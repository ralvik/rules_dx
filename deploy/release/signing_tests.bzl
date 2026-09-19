"""Unit tests for signing selection (issue #311).

Pins the Sigstore keyless + GitHub attestations selection (trust root,
issuer, identity gate) and bundle naming. Would-run dispatch is proven
by the `signing_verify` sh_tests in BUILD via RELEASE_SIGN_DRY_RUN=1.
"""

load("//libs/starlark:defs.bzl", "expect_equal", "starlark_test")
load(":signing.bzl", "SIGNING_ISSUER", "SIGNING_TRUST_ROOT", "signing_bundle_names", "signing_identity_error")

def signing_unit_tests(name):
    starlark_test(
        name = name,
        mode = "unit",
        checks = [
            expect_equal(
                "signing trust root is Sigstore TUF",
                SIGNING_TRUST_ROOT,
                "https://tuf-repo-cdn.sigstore.dev",
            ),
            expect_equal(
                "signing issuer is GitHub OIDC",
                SIGNING_ISSUER,
                "https://token.actions.githubusercontent.com",
            ),
            expect_equal(
                "signing bundle names derive per instance",
                signing_bundle_names("release"),
                ("release.bundle", "release.attestation"),
            ),
            expect_equal(
                "signing identity accepts the release workflow identity",
                signing_identity_error("https://github.com/ralvik/rules_dx/.github/workflows/release.yml@refs/tags/v1.2.3", "https://token.actions.githubusercontent.com"),
                "",
            ),
            expect_equal(
                "signing identity rejects an empty identity",
                signing_identity_error("", "https://token.actions.githubusercontent.com"),
                "signing: invalid identity '': want the owner-approved release workflow identity",
            ),
            expect_equal(
                "signing identity rejects a foreign issuer",
                signing_identity_error("https://github.com/ralvik/rules_dx/.github/workflows/release.yml@refs/tags/v1.2.3", "https://evil.example"),
                "signing: invalid issuer 'https://evil.example': want 'https://token.actions.githubusercontent.com' (Sigstore keyless via GitHub OIDC)",
            ),
        ],
    )
