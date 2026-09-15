# Roadmap

Cleanup first; expand and improve without haste for v1. No milestone system:
tracked work lives in [GitHub issues](https://github.com/ralvik/rules_dx/issues)
and `.github/ISSUE_TEMPLATE/` (external reports). Domain docs under `docs/`
describe current behavior.

## Cleanup — completed

* [#11](https://github.com/ralvik/rules_dx/issues/11) — domain docs de-milestoning (closed).
* [#2](https://github.com/ralvik/rules_dx/issues/2) — `//env:default_config` membership conflict (closed).

## Improve

* [#5](https://github.com/ralvik/rules_dx/issues/5) — tag hygiene and release-input gaps, no
  publication pressure.
* [#6](https://github.com/ralvik/rules_dx/issues/6), [#7](https://github.com/ralvik/rules_dx/issues/7),
  [#8](https://github.com/ralvik/rules_dx/issues/8), [#9](https://github.com/ralvik/rules_dx/issues/9) —
  toolchain, provider, and adapter qualification backlogs.
* [#10](https://github.com/ralvik/rules_dx/issues/10) — docs-pipeline execution gaps (adapter runs,
  site build, guide-step CI, first-hour timing proof).
* Verification stages: [#58](https://github.com/ralvik/rules_dx/issues/58) (hello smoke as test),
  [#56](https://github.com/ralvik/rules_dx/issues/56) (parser-sample backfill),
  [#55](https://github.com/ralvik/rules_dx/issues/55) (E2E via integration test),
  [#54](https://github.com/ralvik/rules_dx/issues/54) (close-out battery + docs),
  [#49](https://github.com/ralvik/rules_dx/issues/49) (rustfmt with crate edition).
* Robustness and hygiene: [#76](https://github.com/ralvik/rules_dx/issues/76) (repo reorg),
  [#80](https://github.com/ralvik/rules_dx/issues/80) (CI hygiene),
  [#81](https://github.com/ralvik/rules_dx/issues/81) (split exec.rs),
  [#82](https://github.com/ralvik/rules_dx/issues/82) (warnings-as-errors),
  [#83](https://github.com/ralvik/rules_dx/issues/83) (visibility hardening),
  [#77](https://github.com/ralvik/rules_dx/issues/77) (cli-contract claims),
  [#78](https://github.com/ralvik/rules_dx/issues/78) (publishing dry-run),
  [#79](https://github.com/ralvik/rules_dx/issues/79) (native updater).
* Rust library extraction: [#64](https://github.com/ralvik/rules_dx/issues/64)–[#74](https://github.com/ralvik/rules_dx/issues/74).

## Parked

* [#3](https://github.com/ralvik/rules_dx/issues/3) — native `dx update` bot stays manual-interim.
* [#4](https://github.com/ralvik/rules_dx/issues/4) — `dx migrate` stays post-v1 direction-only.
