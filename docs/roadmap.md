# Roadmap

Cleanup first; expand and improve without haste for v1. Domain docs under
`docs/` describe current behavior.

## Cleanup — completed

* Domain docs de-milestoning.
* `//env:default_config` membership conflict.
* Facade reorganization (`dx/` Rust to `cli/`, documentation IR to `docs/ir`, mixed/hello fixture to `examples/mixed`; issue #76).

## Improve

* Tag hygiene and release-input gaps, no publication pressure.
* Toolchain, provider, and adapter qualification backlogs: Rust providers plus Gazelle plus integration (issues #470-#475), runners plus locks plus defaults (issues #476-#489), adapter wiring (issues #490-#493), native qualification (issues #494-#505), env plus codegen (issue #506), coverage plus consumer plus quality (issues #507-#512).
* Docs-pipeline execution gaps stay open under issue #421 (live successor to closed #310; adapter runs with pins
  and mappings, renderer and site execution, byte-identical rebuild proof, link and
  reference completeness, guide prose with guide-step CI wiring, first-hour timing proof,
  and per-release pin-bump plus drift process; no working site claimed).
* Verification stages: hello smoke as test delivered (issue #464,
  11 binary hellos with `hello_output_test` under `bazel test //...`),
  parser-sample backfill delivered (issue #465,
  per-adapter pass plus fail samples with `parser_sample_qualification`),
  E2E via
  e2e suite, close-out battery + docs delivered (issue #467,
  battery commands plus docs gate with `closeout_battery_qualification`),
  rustfmt with crate edition delivered (issue #468, crate edition flows to
  rustfmt with fixtures plus `rustfmt_edition_qualification`),
  CC opt-out linker delivered (issue #471, kept opt-out executes for
  pure-Rust scripts with fixtures plus `cc_optout_qualification`).
* Robustness and hygiene: CI hygiene,
  warnings-as-errors, visibility hardening, cli-contract claims, human-run
  release path (signing-first, issue #458), signing stack + distribution
  (issue #459), GHCR prebuilt
  images (separate workflow, issue #460), native bump loop as sole updater
  (decided, issue #461), automation policy (native-only).
* V1 scope: `dx migrate` syntax + manifest selection delivered (issue #462, parsed CLI
  plus major-release-only gate plus one manifest per major hop with fail-closed execution).
  `dx run` multirun delivered (issue #463, sequential local-only multirun for explicit
  labels/patterns). No post-v1 bucket.
* Rust library extraction decided internal-only under issue #469 (see ADR 0023; 34 internal crates, binaries-only boundary, no consumer migration).
* CLI and deploy follow-ups: `--debug`/`--release` flags plus `DX_PROFILE` forwarding plus deploy provider/CLI (issues #457-#458); `dx docs` reintroduction with real extraction/validation (see ADR 0020).
