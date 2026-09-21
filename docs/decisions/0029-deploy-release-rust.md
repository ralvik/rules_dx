# ADR 0029: Deploy/Release Shell Rust Delivery

## Status

Accepted.

## Context

[ADR 0026](./0026-rust-product-code.md) scopes the Rust product-code migration
with a phase index: archiver/hasher delivered Rust, preset delivered Rust,
depcheck delivered Rust per [ADR 0027](./0027-depcheck-rust.md), SBOM/BCR
generators delivered Rust, plus provisional deploy launcher programs and the
accepted deferred CI-driver plus `update.py` stance per
[ADR 0028](./0028-deferred-ci-drivers-update.md). Each phase lands only with
an accepted successor plus its guard update.

`deploy/release/` ran the owner-gated BCR plus signing launchers as
`sh_binary` with `runfiles.bash` plus `bcr_deploy.sh`/`sign_deploy.sh`, the
human-run driver as `sh_binary release.sh`, and the install verifier as
`sh_binary dx_verify.sh`, all Linux-only with Windows `shell: bash`. Verify
harnesses (`bcr_verify.sh`, `signing_verify.sh`, `sbom_verify.sh`,
`release_verify.sh`, `dx_verify_test.sh`) were Linux-only `sh_test`. The
nine `deploy/rules/` per-instance launchers already run as hermetic
`py_binary` on the managed Python 3.12 toolchain with the `profile` attr
documented on all paths; they stay Python in this phase and migrate to Rust
as a follow-up.

## Decision

Deploy/release shell is Rust: `deploy/release` owns `dx_release_tools`
launch logic (`bcr_run`, `signing_run`, `release_run`, `sbom_verify_files`)
plus thin `release_driver` `rust_binary`, per-instance `rust_binary`
launchers expanded from `bcr.bzl`/`signing.bzl` via the Rust `runfiles`
library, and portable `rust_test` coverage over the existing goldens;
`deploy/install` owns `dx_install_tools` (`parse_args`, `verify` with a
fake-verifier seam) plus `rust_binary :dx_verify` and portable
`rust_test :dx_install_tools_test`. The shell launchers plus verify scripts
are removed. CLI args, exit codes, dry-run gates, offline behavior, and the
`DxDeployInfo` plus `dx_deployment` surface stay stable for CI callers.

## Consequences

- `//tools/ci:product_runtime_guards` pins the Rust delivery (no shell
  launchers/verifies, `rust_binary` present, portable `rust_test`).
- `//tools/ci:signing_distribution_qualification`,
  `//tools/ci:sbom_upload_qualification`, `//tools/ci:publish_trust`, and
  `//tools/ci:release_hygiene` pin the Rust launch plus verifier strings.
- Qualification callers invoke `bazel run //deploy/release:bcr_demo_program`,
  `//deploy/release:signing_demo_program`, `//deploy/release:release_driver`,
  and `bazel run //deploy/install:dx_verify`, never the deleted `.sh`.

## Rejected Alternatives

- Keep `sh_binary` plus `sh_test`: preserves Linux-only pins where Rust is
  cross-platform.
- Migrate the nine `py_binary` deploy launchers in this phase: correct
  direction but doubles the review; tracked as the immediate follow-up with
  its own successor record.
