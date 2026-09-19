# Roadmap

Cleanup first; expand and improve without haste for v1. Domain docs under
`docs/` describe current behavior.

## Cleanup — completed

* Domain docs de-milestoning.
* `//env:default_config` membership conflict.

## Improve

* Tag hygiene and release-input gaps, no publication pressure.
* Toolchain, provider, and adapter qualification backlogs.
* Docs-pipeline execution gaps stay open under issue #310 (adapter runs with pins
  and mappings, renderer and site execution, byte-identical rebuild proof, link and
  reference completeness, guide prose with guide-step CI wiring, first-hour timing proof,
  and per-release pin-bump plus drift process; no working site claimed).
* Verification stages: hello smoke as test, parser-sample backfill, E2E via
  integration test, close-out battery + docs, rustfmt with crate edition.
* Robustness and hygiene: repo reorg, CI hygiene, split exec.rs,
  warnings-as-errors, visibility hardening, cli-contract claims, human-run
  release path (signing-first), signing stack + distribution, GHCR prebuilt
  images (separate workflow), Renovate plus native bump loop complementary
  (decided, issue #326), automation policy (Renovate allowed).
* V1 scope: `dx migrate` syntax + manifest selection, `dx run` multirun.
  No post-v1 bucket.
* Rust library extraction.
