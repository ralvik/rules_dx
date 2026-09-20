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
  parser-sample backfill, E2E via
  integration test, close-out battery + docs, rustfmt with crate edition.
* Robustness and hygiene: CI hygiene,
  warnings-as-errors, visibility hardening, cli-contract claims, human-run
  release path (signing-first, issue #458), signing stack + distribution
  (issue #459), GHCR prebuilt
  images (separate workflow, issue #460), native bump loop as sole updater
  (decided, issue #461), automation policy (native-only).
* V1 scope: `dx migrate` syntax + manifest selection (issue #462). `dx run` multirun
  delivered (issue #463, sequential local-only multirun for explicit labels/patterns).
  No post-v1 bucket.
* Rust library extraction.
* CLI and deploy follow-ups: `--debug`/`--release` flags plus `DX_PROFILE` forwarding plus deploy provider/CLI (issues #457-#458); `dx docs` reintroduction with real extraction/validation (see ADR 0020).
