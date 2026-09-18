# Roadmap

Cleanup first; expand and improve without haste for v1. Domain docs under
`docs/` describe current behavior.

## Cleanup — completed

* Domain docs de-milestoning.
* `//env:default_config` membership conflict.

## Improve

* Tag hygiene and release-input gaps, no publication pressure.
* Toolchain, provider, and adapter qualification backlogs.
* Docs-pipeline execution gaps (adapter runs, site build, guide-step CI,
  first-hour timing proof).
* Verification stages: hello smoke as test, parser-sample backfill, E2E via
  integration test, close-out battery + docs, rustfmt with crate edition.
* Robustness and hygiene: repo reorg, CI hygiene, split exec.rs,
  warnings-as-errors, visibility hardening, cli-contract claims, human-run
  release path (signing-first), signing stack + distribution, GHCR prebuilt
  images (separate workflow), Renovate out-of-the-box (native bot deferred),
  automation policy (Renovate allowed).
* V1 scope: `dx migrate` syntax + manifest selection, `dx run` multirun.
  No post-v1 bucket.
* Rust library extraction.
