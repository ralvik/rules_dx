# Roadmap

Cleanup first; expand and improve without haste for v1. Domain docs under
`docs/` describe current behavior.

## Cleanup — completed

* Domain docs de-milestoning.
* `//env:default_config` membership conflict.
* Facade reorganization (`dx/` Rust to `cli/`, documentation IR to `docs/ir`, mixed/hello fixture to `examples/mixed`; issue #76).
* Verification stages Seed-host-delivered (closed #464, #465, #467, #468, #471, #472, #474; qualified seed-only via the harnesses below; platform plus consumer plus release evidence stays owned gap; no Supported claim; verification `Delivered` is seed-host layer evidence, not support-matrix promotion):
  * hello smoke as test Seed-host-delivered (closed #464,
  11 binary hellos with `hello_output_test` under `bazel test //...`),
  * parser-sample backfill Seed-host-delivered (closed #465,
  per-adapter pass plus fail samples with `parser_sample_qualification`),
  * close-out battery + docs Seed-host-delivered (closed #467,
  battery commands plus docs gate with `closeout_battery_qualification`),
  * rustfmt with crate edition Seed-host-delivered (closed #468, crate edition flows to
  rustfmt with fixtures plus `rustfmt_edition_qualification`),
  * CC opt-out linker Seed-host-delivered (closed #471, kept opt-out executes for
  pure-Rust scripts with fixtures plus `cc_optout_qualification`),
  * Shell-env default vs annotation extension Seed-host-delivered (closed #472, global
  `False` with zero per-crate opt-ins plus `shell_env_qualification`),
  * CXX graph identity decided Seed-host-delivered (closed #474, single `crates` graph with
  `cxx == cxxbridge-cmd == 1.0.200` plus fixtures plus
  `cxx_identity_qualification`).

## Improve

* Tag hygiene and release-input gaps, no publication pressure.
* Release promotion checklist to Supported owned under #808 (successor to closed #611;
  `docs/product/promotion-checklist.md` with fixture evidence via
  `bazel run //tools/ci:promotion_checklist_qualification`; tag hygiene,
  versioning, and platform plus consumer plus release evidence per cell;
  ad-hoc release rejected; no Supported claim).
* SBOM plus provenance upload on CI owned under closed #612
  (`sbom` job in `.github/workflows/ci.yml` plus fixture evidence via
  `bazel run //tools/ci:sbom_upload_qualification`; SPDX-2.3 plus SLSA v1
  build plus verify plus `sbom-provenance` upload on every push/PR, attestation
  stays owner-gated human-run; dry-run-only rejected; no Supported claim;
  per-host release evidence #803-#807).
* Toolchain, provider, and adapter qualification backlogs: Rust providers plus Gazelle plus integration (issues #470-#475), runners plus locks plus defaults (issues #476-#489), adapter wiring (issues #490-#493), native qualification (issues #494-#505), env plus codegen (#787, #788; successors to closed #506), coverage plus consumer plus quality (#802; successor to closed #512).
* Docs-pipeline execution gaps stay open under #779-#785 (successors to closed #581, live successor to closed #421; adapter runs with pins
  and mappings, renderer and site execution, byte-identical rebuild proof, link and
  reference completeness, guide prose with guide-step CI wiring, first-hour timing proof,
  and per-release pin-bump plus drift process; no working site claimed).
* Robustness and hygiene: CI hygiene,
  warnings-as-errors, visibility hardening, cli-contract claims, human-run
  release path (signing-first, issue #458), signing stack + distribution
  (issue #459), GHCR prebuilt
  images (separate workflow, issue #460), native bump loop as sole updater
  (decided, issue #461), automation policy (native-only).
* V1 scope: `dx migrate` syntax + manifest selection delivered (issue #462, parsed CLI
  plus upgrade-only gate plus one manifest per major hop and one per full version pair for minor/patch
  with fail-closed execution; upgrade scope issue #671 per ADR 0025).
  `dx run` multirun delivered (issue #463, sequential local-only multirun for explicit
  labels/patterns). No post-v1 bucket.
* Rust library extraction decided internal-only under issue #469 (see ADR 0023; 34 internal crates, binaries-only boundary, no consumer migration).
* CLI and deploy follow-ups: `--debug`/`--release` flags plus `DX_PROFILE` forwarding plus deploy provider/CLI (#814; successor to closed #457-#458); `dx docs` reintroduction with real extraction/validation (see ADR 0020; open under #786, successor to closed #581, live successor to closed #421).
