# M11 Completion Report: PATH Tools And Environment Bootstrap

Seed host: local Linux x86_64 glibc (same host class as the M00 seed report).
Local execution only, no remote. Non-Linux platforms were unavailable and are
recorded as gaps, not claimed.

WP1 (validated transitive PATH-tool records and collision detection), WP2
(staged symlink trees with versioned ownership metadata plus the managed
`.dx/bin` installer), and WP3 (CLI and initial quality tools on the same
public contribution path) are complete per the
[M11 milestone](M11-path-tools-environment-bootstrap.md). No silent
deferrals. No capability-term transitions are claimed: the environment
machinery is first-party implementation under test, not a product support
claim.

## WP1: Tool Records, Composition, Collision Detection

`env/defs.bzl` implements the public registry:

- `environment_tool` validates one host-tool record (primary `bin_name`
  plus `aliases`) against its executable's files-to-run and exposes it as
  `EnvironmentInfo` (`runners` dict by owner label, `tools` depset of
  `{bin_name, aliases, owner}` structs).
- `environment_config` composes records transitively and fails closed on
  host-name collisions via `env_collision_error`.
- Name validation is conservative: collision keys are case-folded (a
  collision-free config here is collision-free on case-insensitive hosts
  too); Windows reserved stems (`CON`, `PRN`, `AUX`, `NUL`, `COM1-9`,
  `LPT1-9`, extension forms like `nul.txt`) and explicit executable
  suffixes (`.exe`, `.bat`, `.cmd`, `.com`, case-insensitive) are rejected
  because suffix materialization owns the platform suffix (WP2).
- Native filename rules (`env_host_filename`): logical name unchanged on
  POSIX, `.exe` appended on Windows; validation guarantees no doubling.
- Fixtures in `env/BUILD.bazel`: `tool_alpha` (+alias `a`) and `tool_beta`
  over `tool.sh`, with `config_under_test` reaching `tool_beta` only
  transitively through `config_inner`.

## WP2: Staged Trees, Managed Install, Bootstrap Binary

- `environment_tree` stages one symlink per host name plus versioned
  management metadata (`ENV_METADATA_SCHEMA_VERSION = 1`); the on-disk
  `.rules_dx_managed` binary Protobuf marker (frozen schema in
  `env/marker.proto`, `marker_proto`/`marker_proto_rs`) carries the schema
  version so replacement refuses metadata it cannot validate.
- `dx/env` (`dx_env` library + `env` binary, `dx/env/BUILD.bazel`) installs
  the runfiles-located default tree into `<workspace>/.dx/bin`: fresh
  install, second-run noop (`already current`), replacement with stale-entry
  removal, restore, unmanaged-tree refusal, marker presence, and doctor
  execution via the installed link. Swap atomicity (staging
  `.dx/bin.next`, commit rename, `.dx/bin.prev` rollback surface, no
  leftovers) and crash recovery are unit-tested in `src/lib.rs`; the
  `bootstrap_test` sh_test proves the installed surface under Bazel,
  including a workspace path containing spaces.
- Windows fails before mutation: the installer pre-checks symlink
  capability and returns actionable `symlink_guidance` (no
  junction/copy/launcher fallback); `os` is selectable so the guidance is
  unit-tested without a Windows host. Physical Windows evidence remains a
  gap (no host available).

## WP3: Default Config On The Public Path

`env/BUILD.bazel` `default_config` contributes `doctor` (curated probe),
module-matched `dx` (`//dx/cli:dx`), and user-facing `quality_markdown`
(`//quality/markdown:quality_markdown`) through the same public
`environment_tool` path. Internal pipeline binaries (`quality_runner`,
`quality_evaluator`) are intentionally not exposed, per the managed
environment tool-exposure contract (O62). Fresh bootstrap installs exactly
3 tools, asserted by `bootstrap_test.sh`.

## Public Surface And Load Labels

- Public Starlark: `environment_tool`, `environment_config`,
  `environment_tree`, `EnvironmentInfo`, `env_host_filename`,
  `env_name_error`, `env_collision_error`, `ENV_METADATA_SCHEMA_VERSION`
  from `//env:defs.bzl`; `//env:default_tree` is the canonical staged
  default tree; `//dx:env` resolves per the milestone outcome.
- Host surface: `.dx/bin/<tool>` symlinks (`.exe` on Windows) plus the
  regular `.rules_dx_managed` marker file. No other dotfiles are written.

## Evidence

Exact commands on this host (2026-09-11; all Bazel clients run with
`LD_PRELOAD` unset — the ambient `libtorsocks.so` preload breaks the
client↔server localhost channel, so every prior default-environment
connect attempt timed out at 120s and the server was declared
non-responsive; unsetting it connects instantly):

- `bazel --output_base=/tmp/dx-verify-ob test //env/... //dx/env/...`:
  10/10 pass.
- `bazel --output_base=/tmp/dx-verify-ob test --nocache_test_results
  --test_output=errors //env/... //dx/env/...`: 10/10 pass fresh
  (`bootstrap_test`, `dx_env_bin_test`, `dx_env_clippy_test`,
  `dx_env_fmt_test`, `dx_env_test`, `env_config_analysis`,
  `env_default_config_analysis`, `env_default_tree_analysis`,
  `env_defs_unit`, `env_tree_analysis`).
- `bazel --output_base=/tmp/dx-verify-ob coverage //...
  --combined_report=lcov --test_tag_filters=-no-coverage
  --test_output=errors`: 69/69 pass (60 executed).
- Coverage gate (`//tools/coverage:check` over the LCOV report, full
  Bazel-declared `*.rs`/`*.go` sources vs `tools/coverage/inventory.txt`):
  **18848/18848** executable lines covered, including M11's
  `dx/env/src/lib.rs: 804/804 (22 ignored with reasons)` and
  `dx/env/src/main.rs: 0/0 (107 ignored; thin shim, zero eligible
  lines)`. The tool's overall verdict is FAIL solely on eight
  not-yet-inventoried files from the same-day M10 WP3 landing
  (`gazelle/rust/native_config{,_test}.go`, four
  `testdata/native_config` fixture sources,
  `quality/testdata/generated_shape.rs`) — sibling follow-up, zero
  M11-attributable gaps.
- Collision/name matrix (`env/defs_tests.bzl`, in the passing
  `env_defs_unit`/`env_config_analysis` suites): suffix rejection,
  reserved stems, empty/dot/separator names, disjoint acceptance,
  same-owner repeats, primary/alias cross-tool collisions, case folding,
  multi-claimant errors.
- `bootstrap_test.sh` asserts under Bazel: `installed 3 tool(s)`, three
  managed symlinks, regular marker, no `.next`/`.prev` leftovers, doctor
  output identity (`//env:doctor`), noop, 2-tool replacement with stale
  removal, 3-tool restore, foreign-tree refusal without modification.
- `dx_env_fmt_test` / `dx_env_clippy_test`: rustfmt via the pinned
  toolchain config, Clippy with warnings as errors.

## Changed Components

- `env/` (`defs.bzl`, `defs_tests.bzl`, `BUILD.bazel`, `marker.proto`,
  `tool.sh`, `doctor.sh`): registry, tests, fixtures, default config.
- `dx/env/` (`Cargo.toml`, `BUILD.bazel`, `src/lib.rs`, `src/main.rs`,
  `bootstrap_test.sh`): installer crate, binary, e2e proof.
- `dx/BUILD.bazel`: canonical `//dx:env` resolution.
- This report; milestone index status flip.

## Deviations, Gaps, Exclusions

No deviations from the milestone scope. Out of scope respected:
language-native environments, `dx env` planning, generated-source
projection, and `dx setup` untouched. Gaps: physical Windows host evidence
(unit-covered pre-check only), remote/cache qualification, and
external-consumer evidence (M27+). The M10-WP3 inventory follow-up is
owned by that work package, not this milestone.
