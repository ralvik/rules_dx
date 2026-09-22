# Helper Upstream Re-evaluation Policy

Accepted. Owns the trigger, adopt-vs-keep criteria, and per-item verdicts for the hand-rolled helper decisions qualified under #315/#395.

The gap previously claimed as untracked re-evaluation in the [CLI contract](cli-contract.md#implementation-hygiene) and in [remaining reds](../testing/verification-matrix-remaining.md) now points here. Enforcement is `bazel run //tools/ci:helper_qualification`.

## Triggers

A re-check is forced by any of these events. The watcher is the native bump loop, the sole updater per [automation](../contributing/automation.md#native-bump-loop-delivered). Discovery enumerates outdated helpers via upstream registry clients under `dx_bump::discovery`.

- New stable major of any adopted helper (`hex`, `blake3`, `sha2`, `similar`, `spdx`, `chrono`, `tempfile`, `walkdir`, `ignore`, `globset`, `lcov`); for pre-`1.0` crates a minor counts as major. The bump PR must re-run `helper_qualification` and revisit the verdicts below before any update-only auto-merge.
- Advisory touching a helper or its transitive closure (`RUSTSEC`/`GHSA`/`OSV`). Re-check is immediate, even with no version bump.
- MSRV or toolchain drift. A helper MSRV above the pinned Rust toolchain blocks the bump fail-closed; a toolchain bump re-opens previously rejected alternatives. See the [stable stack](../native-toolchains.md) and [dependency currency](../decisions/0008-dependency-currency.md).
- Explicit migration proposal. Anyone proposing a previously rejected crate must meet the criteria below and re-qualify with lock updates. No silent drift.

## Adopt-vs-keep criteria

Generalizes the `cli/atomic_fs` precedent (stable `std::fs::File::try_lock` already owns the flock, so `fs2`/`fslock` add review plus lockfile plus manifest cost for zero gain). Per-helper reasons stay in source; the gate is now shared. Adopt requires all four; otherwise the helper stays hand-rolled with an owned reason.

- Behavior gain: upstream must supply needed behavior, not API parity. Repo contracts (digest lowercase policy, canonical diff headers, ladder order, LCOV 100% verdict, SPDX lattice, shape gate) never count as gain.
- Supply-chain proportionality: review plus lockfile churn plus `MODULE.bazel` manifests plus transitive tree must be proportionate. A stable standard-library equivalent wins.
- Semantic fit: upstream must not change classifier, verdict, or fail-closed semantics. Normalization versus classification and generic type versus policy lattice are mismatches.
- Contract preservation: typed path errors, byte-identical outputs, and fail-closed handling must be preserved or re-qualified.

## Per-item verdicts

All stays-hand-rolled items are final. No migration spike is scheduled beyond the owned `jiff 1.0` re-evaluation under #398.

| Item | Rejected upstream | Verdict |
| --- | --- | --- |
| Atomic write plus lock (`cli/atomic_fs`) | `fs2`, `fslock`, `atomic-write-file`, `fs-err` | Final: `try_lock` plus `tempfile` owns the path |
| Path ladder (`cli/path`) | `path-clean`, `camino` | Final: shape classifier, not normalization or UTF-8 type |
| LCOV scanner plus inventory plus verdict (`cli/lcov`) | `cargo-llvm-cov` | Final: tool binary, not a marker or verdict library; parser stays `lcov` |
| SPDX lattice (`cli/audit` license) | `license-exprs` | Final: repo policy lattice; parser stays `spdx` |
| Date shape gate (`cli/audit` exception) | `time`-family second engine | Final: hand-rolled shape gate, calendar stays `chrono` |
| Scratch wrappers (`cli/test_scratch` plus prod scratch discipline) | No upstream wrapper candidate | Final: creation stays `tempfile`, wrappers stay repo discipline |
| Semver scopes (`npm` hand parser plus Go normalization) | `node-semver` second engine | Final: `semver`-normalized parsing already owned |

The date engine re-evaluation on `jiff 1.0` stays owned under #398. Any other trigger above re-opens only the affected row.

## Migration rule

Process plus spikes only. A pin change that alters behavior needs a green `helper_qualification`, updated Cargo plus Bazel locks, and the affected Rust tests. No CLI flag or output break is authorized by this policy.
