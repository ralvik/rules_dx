# ADR 0028: Deferred CI Drivers Plus Artifacts Update Stance

## Status

Accepted.

## Context

[ADR 0026](./0026-rust-product-code.md) scopes the Rust product-code migration
with a phase index: archiver/hasher plus preset plus depcheck (per
[ADR 0027](./0027-depcheck-rust.md)) plus SBOM/BCR generators delivered Rust,
deploy launchers provisional, and the CI-driver plus `update.py` slice
deferred. Each phase lands only with an accepted successor plus its guard
update.

After the four delivered phases, residue remains: 133 `tools/ci/*.sh`
qualification drivers (`sh_binary`, Linux-only bash harness) and
`quality/artifacts:update` (`update.py`, 643 lines, maintainer-only network
plus `readelf`/`objdump`/`tar`). The drivers are IO-bound Bazel-invoking
maintainer CI, not consumer product; the updater needs network plus ELF
tooling that is awkward in a sandboxed `rust_binary`. The harness strategy
is owned by #667, guard maintenance by #653, bootstrap dedupe by #654; the
shell contract lives in
[Tool And Platform Test Matrix](../testing/tools.md#shell-and-host-tool-contract)
and the artifact metadata contract in
[Tool Acquisition](../tools/tool-acquisition.md). Without a recorded stance
this residue generates repeat "why is there still shell/python?" issues.

## Decision

`tools/ci/*.sh` stay bash and `quality/artifacts:update` stays Python.
Harness-wide Rust-ify is out of scope here; harness strategy lives under
#667 and guard work under #653. Either slice migrates opportunistically
only: when a driver becomes consumer-facing or blocks a platform claim, or
when #667 decides a portable/POSIX driver direction, that work lands there
with its own accepted record plus guard update, not here.

## Consequences

- `//tools/ci:product_runtime_guards` pins the deferral (`update`
  `py_binary` present, `tools/ci` driver count floor) instead of migrating
  it.
- The umbrella phase index closes except the provisional deploy-launcher
  slice (#764), which still needs its own accepted successor.
- No consumer API change; docs/stance only.

## Rejected Alternatives

- Rewrite all `tools/ci` drivers in Rust: rejected on ROI unless a driver
  becomes consumer-facing or blocks a platform claim; IO-bound
  Bazel-invoking maintainer CI gains negligibly while re-opening
  #667/#653/#654 scope.
