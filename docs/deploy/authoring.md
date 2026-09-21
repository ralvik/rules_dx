# Deploy Authoring

Two paths produce a deployable target for `dx deploy`.

## Path A: raw executable (accepted)

Path A is a raw executable: any `*_binary` or executable target (aliases
included) is deployable with no provider. Bazel owns executability.
Use this path for a single program with no separate app identity and no
non-default profile.

## Path B: `dx_deployment` wrapper (accepted)

Path B wraps the program in `dx_deployment`
(`deploy/rules/defs.bzl`):

```starlark
load("@rules_dx//deploy/rules:defs.bzl", "dx_deployment")

dx_deployment(
    name = "production",
    deploy = ":release_script",
    app = ":server",
    profile = "release",
)
```

`deploy` is the executable program. `app` is the deployed app target
when distinct from the program; omit it to deploy the program itself.
`profile` is the default only (`debug`, `dev`, or `release`, default
`release`); an explicit `--debug`/`--release` flag always wins.
Invalid profiles fail at analysis.

`DxDeployInfo(app, profile)` is the dispatch boundary `dx deploy`
reads. Custom rules participate by returning it directly with an
executable `DefaultInfo`; they do not depend on the wrapper. There are
no `kind`, `environment`, or veto fields: deploy identity lives in
target names (`//deploy:production`) until a concrete consumer proves
the need.

Profile vocabulary follows [ADR 0021](../decisions/0021-build-profiles.md).
Precedence (explicit flag over target `profile` over command default)
and the `DX_PROFILE` name are implemented as specified in the
[`dx deploy` command](../cli/commands/build-test-coverage.md#dx-deploy),
delivered under closed #178/#179/#180.

## Path C: `archive_deploy` (accepted)

The first deploy macro
(distribution artifact qualified under issue #459) packages
one executable as a tarball + sha256 checksum with the managed Python
3.12 toolchain only (deterministic `archiver` tar.gz plus `hasher`
sha256 as declared genrule `tools` in `deploy/rules/`), no host
`tar`/`sha256sum`/`shasum`, no new module dependencies, no
registry, no credentials:

```starlark
load("@rules_dx//deploy/rules:archive.bzl", "archive_deploy")

archive_deploy(
    name = "release",
    app = ":hello",
)
```

`archive_release` stays as a thin compat alias for one release cycle,
then it is removed.

`bazel run //rust/tests/fixtures/hello:release` (or `dx deploy //rust/tests/fixtures/hello:release`)
verifies the checksum and copies `release.tar.gz` +
`release.tar.gz.sha256` to the output directory (first arg after `--`,
else `$BUILD_WORKSPACE_DIRECTORY`, else the cwd). Build actions are
hermetic (toolchain archiver/hasher, deterministic bytes); deploy
runtime needs bash + python3 + POSIX coreutils only (hashing, realpath,
and tar listing via python3). Deploy targets live
next to the app they release.

## Path D: `github_deploy` (accepted)

The second deploy macro
(distribution artifact qualified under issue #459) publishes
pinned files as a draft-only GitHub Release via the host `gh` CLI, no
new module dependencies:

```starlark
load("@rules_dx//deploy/rules:github.bzl", "github_deploy")

github_deploy(
    name = "github_draft",
    artifacts = [":dx"],
)
```

`github_release` stays as a thin compat alias for one release cycle,
then it is removed.

`bazel run //cli/cli:github_draft` (or `dx deploy
//cli/cli:github_draft`) execs `gh release create <tag> <assets...>
--draft --verify-tag`. Draft-only by construction
(open work under issue #458): `draft`
must stay `True`, `--verify-tag` means the program never creates or
pushes tags itself, and the default tag is the `v0.0.0-dryrun`
placeholder. `GH_RELEASE_DRY_RUN=1` prints the would-run command and
publishes nothing; this is what CI exercises. A real draft needs the
tag pushed beforehand and explicit owner approval, then publishing
happens by editing the draft on GitHub.

Our own release runbook is the publish dry-run workflow
([`publish-dry-run.yml`](../../.github/workflows/publish-dry-run.yml),
accepted) plus the human-run path ([release runbook](release-runbook.md),
accepted): it builds the seed-host `dx` binary plus the `dx_standalone` archive,
exercises `//cli/cli:github_draft` in dry-run mode, generates SBOM and
provenance via `//deploy/release:sbom_demo`, exercises owner-gated
signing via `//deploy/release:signing_demo` in dry-run mode, checks the
BCR module shape via `//deploy/release:bcr_demo` without submitting,
and proves the install verifier refuses checksum-only inputs,
so workflow and macro stay consistent instead of duplicating logic.
The full matrix is frozen in `deploy/release/matrix.bzl` (seed Linux
x86_64 qualified; four follow-ups unqualified until their hosts
qualify). Draft-only ceiling enforced, owner approval required.

## Path E: `pypi_deploy` (accepted)

The third deploy macro publishes one wheel plus an optional sdist with a
local-first Python publisher, no shell, no `sh_binary`:

```starlark
load("@rules_dx//deploy/rules:pypi.bzl", "pypi_deploy")

pypi_deploy(
    name = "pypi",
    wheel = ":dist-0.0.0-py3-none-any.whl",
    sdist = ":dist-0.0.0.tar.gz",
)
```

`bazel run //python/tests/fixtures/hello:pypi` (or `dx deploy
//python/tests/fixtures/hello:pypi`) builds a local wheelhouse directory
(`<name>-wheelhouse/` with `simple-index/` plus the pinned `.whl`) and
verifies bytes via sha256, publishing nothing. Pass an output directory
after `--` to choose where the wheelhouse lands (default:
`$BUILD_WORKSPACE_DIRECTORY`, else the cwd). The deploy program is a
`py_binary` on the managed Python 3.12 toolchain only, with pinned `data`
plus the Python runfiles library. Live `twine upload --non-interactive
--repository-url` runs only with `PYPI_PUBLISH_LIVE=1`, `PYPI_API_TOKEN`,
and `PYPI_PUBLISH_APPROVED=1` after explicit owner approval, never by
default. Deploy targets live next to the distribution they publish.

## Path F: `crates_deploy` (accepted)

The fourth deploy macro publishes staged crate sources with a
local-first Python publisher, no shell, no `sh_binary`:

```starlark
load("@rules_dx//deploy/rules:crates.bzl", "crates_deploy")

crates_deploy(
    name = "crates_demo",
    crate = ":crates_demo_crate",
    version = "0.0.0",
)
```

`bazel run //deploy/rules:crates_demo` (or `dx deploy
//deploy/rules:crates_demo`) builds a local vendor directory
(`<crate>-vendor/` with `vendor/<crate>/` plus a file registry under
`registry/<crate>/<version>/`) and verifies bytes via sha256, publishing
nothing. Pass an output directory after `--` to choose where the vendor
tree lands (default: `$BUILD_WORKSPACE_DIRECTORY`, else the cwd). The
deploy program is a `py_binary` on the managed Python 3.12 toolchain
only, with pinned `data` plus the Python runfiles library. Live
`cargo publish` runs only with `CRATES_PUBLISH_LIVE=1`,
`CARGO_REGISTRY_TOKEN`, and `CRATES_PUBLISH_APPROVED=1` after explicit
owner approval, never by default, and never passes `--allow-dirty`.
Deploy targets live next to the crate sources they publish.

## Path G: standalone `dx` install verification (accepted)

Standalone `dx` binaries verify publisher identity at install time via
`//deploy/install:dx_verify` (`deploy/install/dx_verify.sh`). The verifier
requires a Sigstore keyless bundle (`cosign sign-blob --bundle`) plus the
expected certificate identity and issuer, or a GitHub attestation
(`gh attestation verify`), on the Sigstore TUF trust root
(`tuf-repo-cdn.sigstore.dev`); verifiers are `cosign verify-blob` /
`gh attestation verify` / `slsa-verifier` (documented, not self-hosted).
There is no checksum-only fallback: a sha256 alone is not proof of
publisher identity and is rejected. Verification runs before any install
or exec; on failure nothing is installed and the binary never executes.
SBOM (Syft/CycloneDX) bundles verify through the same cosign path when
passed as `--sbom`. BCR needs no signing (archive `source.json` +
integrity hash + `presubmit.yml` + PR review). Seed-host standalone
packaging is `//cli/cli:dx_standalone`; the wider matrix stays
unqualified per the [distribution policy](../environments/environment.md#distribution).
Distribution verification qualified under issue #459. Policy tests are
`bazel test //deploy/install:all` plus `bazel run
//tools/ci:signing_distribution_qualification`.

## Path H: release matrix, SBOM/provenance, signing, BCR, human-run (accepted)

The full release path (issue #311; human-run driver owned under issue
#458, live successor to closed #311 for the human-run path; signing stack
+ distribution qualified under issue #459, live successor to closed
#311/#26/#78 for signing + distribution) is owner-gated dry-run-first
tooling in `deploy/release/` with policy tests `bazel test
//deploy/release:all` plus `bazel run
//tools/ci:signing_distribution_qualification`:

- Matrix (`matrix.bzl`): five cells, seed `dx-linux-x86_64`
  qualified-built-here, four follow-ups unqualified per ADR 0014 until
  host plus toolchain evidence lands.
- SBOM/provenance (`sbom.bzl`): SPDX 2.3 JSON plus SLSA v1 in-toto
  Statement v1 from the managed Python toolchain only (digest + JSON
  via declared genrule `tools`), subject digest equals artifact
  sha256; verifies via `//deploy/install:dx_verify --sbom`. On every
  push/PR the `sbom` job in `.github/workflows/ci.yml` (issue #612) builds
  plus verifies `//deploy/release:sbom_demo` and uploads the
  `sbom-provenance` artifact (publishes nothing).
- Signing/attestation (`signing.bzl` plus `sign_deploy.sh`, qualified
  under issue #459): Sigstore keyless (`cosign sign-blob --bundle`
  v2.4.1 pinned, bundle media type
  `application/vnd.dev.sigstore.bundle.v0.3+json`) plus GitHub
  attestations on the TUF trust root; `RELEASE_SIGN_DRY_RUN=1` prints
  would-sign, publishes nothing.
- BCR (`bcr.bzl` plus `bcr_deploy.sh`): `source.json` plus integrity
  shape check; `BCR_DRY_RUN=1` prints would-submit, submits nothing;
  `0.0.0` never submits.
- Human-run driver (`release.sh`, issue #458): dry-run by default, requires
  `RELEASE_APPROVE=1` plus a pre-pushed tag plus clean tree; never
  creates tags, never runs on CI.
- GHCR stays the separate `.github/workflows/ghcr.yml` route (issue #460,
  image lifecycle per-scaffold-change); push plus `cosign sign <digest>`
  stay owner-gated with dry-run first.

## Path I: `npm_deploy` (accepted)

The fifth deploy macro (`deploy/rules/npm.bzl`) publishes packed files
to a local npm folder feed with the managed Python 3.12 toolchain only
(deterministic `npm_packer` tgz plus feed JSON as declared genrule
`tools`), no host `tar`/`npm`, no new shell, no `sh_binary`, no
registry, no credentials:

```starlark
load("@rules_dx//deploy/rules:npm.bzl", "npm_deploy")

npm_deploy(
    name = "npm_feed",
    package = "npm-demo",
    srcs = ["package.json"],
)
```

`bazel run //path/to:npm_feed` (or `dx deploy //path/to:npm_feed`)
verifies the sha256 plus tar members and copies `npm_feed.tgz` +
`npm_feed.feed.json` to the output directory (first arg after `--`,
else `$BUILD_WORKSPACE_DIRECTORY`, else the cwd), assembling an
`npm_feed-feed/` folder with both files. Build actions are hermetic
(toolchain packer, deterministic bytes); deploy runtime needs Python
3.12 plus POSIX coreutils only. Live `npm publish --access public
--provenance` runs only with `NPM_PUBLISH_LIVE=1` plus `NPM_TOKEN` and
explicit owner approval, never by default; `NPM_PUBLISH_DRY_RUN=1`
prints the would-publish command and publishes nothing. Deploy targets
live next to the package they release.

## Custom deployers (accepted)

User-defined rules join `dx deploy` by returning `DxDeployInfo` with an
executable `DefaultInfo`, without depending on the wrapper:

```starlark
load("@rules_dx//deploy/rules:defs.bzl", "DxDeployInfo")

def _my_deploy_impl(ctx):
    exe = ctx.attr.deploy[DefaultInfo].files_to_run.executable
    link = ctx.actions.declare_file(ctx.label.name)
    ctx.actions.symlink(output = link, target_file = exe)
    return [
        DefaultInfo(executable = link),
        DxDeployInfo(app = ctx.attr.app.label, profile = "release"),
    ]

my_deploy = rule(
    implementation = _my_deploy_impl,
    executable = True,
    attrs = {
        "app": attr.label(mandatory = True),
        "deploy": attr.label(executable = True, cfg = "target", mandatory = True),
    },
)
```
