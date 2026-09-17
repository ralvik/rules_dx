//! Delivered adoption behavior (M30b: `dx init` scaffolding, hermetic hook
//! runner, devcontainer admission, `dx status` diagnostics, single-version
//! `dx version` with rollback, local watch loop, thin inspect forwarding,
//! and single-source completion generation).
//!
//! Planning predicates below own the adoption shape; the I/O helpers after
//! them deliver it: absent-only scaffolding, unmanaged refusal, pin files
//! that equal the `rules_dx` module version, hermetic-only hook Git,
//! two-layer hook configuration, pinned Bazel-delegated devcontainers, the
//! consolidated status surface (never `doctor`), local-only re-resolved
//! watch iterations, thin `query`/`cquery` forwarding, and completion
//! scripts generated from the single command table. Helpers operate on
//! injected paths only and touch no network.
//!
//! Domain split (issue #236): single-source completion vocabulary lives in
//! the `completion` module, major-release migration planning lives in
//! the `migrate` module, the single-version pin lives in the `version`
//! module, thin inspect forwarding lives in the `inspect` module, the
//! local watch loop lives in the `watch` module, absent-only `dx init`
//! scaffolding lives in the `scaffold` module, the hermetic hook
//! runner lives in the `hooks` module, the typed failure vocabulary
//! lives in the `error` module, and the admissibility policy lives in
//! the `policy` module. This facade keeps
//! the re-exports; the public path stays stable via the re-exports below.

// Issue #238: infallible paths must not `expect`/`unwrap` outside tests
// (`cfg_attr(not(test))` keeps `rust_test` bodies ergonomic).
#![cfg_attr(not(test), deny(clippy::expect_used, clippy::unwrap_used))]

pub mod completion;
pub mod error;
pub mod hooks;
pub mod inspect;
pub mod migrate;
pub mod policy;
pub mod scaffold;
pub mod status;
pub mod version;
pub mod watch;

pub use completion::{completion_source_is_single, ALL_COMMANDS, SUPPORTED_SHELLS};
pub use error::AdoptError;
pub use hooks::{
    hook_git_is_hermetic, hook_shim_overwrite_allowed, hook_status_shows_merged, install_hooks,
    render_hook_shim, render_hooks_status, uninstall_hooks, HOOK_BUDGET_SECS, HOOK_MANAGED_MARKER,
};
pub use inspect::{inspect_scope_allowed, plan_inspect, plan_somepath, InspectPlan};
pub use migrate::{migrate_is_major_bump, migrate_manifest_name, plan_migrate, MigratePlan};
pub use policy::{devcontainer_is_admissible, diagnostics_command_allowed};
pub use scaffold::{
    absent_only_write_allowed, apply_init, init_must_refuse, plan_init_files, ScaffoldFile,
    DEVCONTAINER_JSON, RENOVATE_JSON,
};
pub use status::{default_status_checks, render_status_json, render_status_text, StatusCheck};
pub use version::{
    read_version_pin, rollback_re_pins_previous, version_pin_matches_module, write_version_pin,
    DX_VERSION, MODULE_VERSION, PREVIOUS_VERSION,
};
pub use watch::{
    coalesce_watch_paths, plan_watch, watch_for_change, watch_iteration_accepts,
    WATCHABLE_COMMANDS, WATCH_DEBOUNCE_MS,
};

// Hook per-check budget seconds (O49 freeze: blocking timeout) lives in
// the `hooks` module (issue #236); the re-exports above keep
// `HOOK_BUDGET_SECS` on the `dx_adopt` facade.

// Typed adoption failure lives in the `error` module (issue #236);
// the re-export above keeps `AdoptError` on the `dx_adopt` facade.

// Admissibility policy lives in the `policy` module (issue #236);
// the re-exports above keep `devcontainer_is_admissible` and
// `diagnostics_command_allowed` on the `dx_adopt` facade.

// Commands watchable under ADR 0017/0018 live in the `watch` module
// (issue #236); the re-exports above keep `WATCHABLE_COMMANDS` and
// `WATCH_DEBOUNCE_MS` on the `dx_adopt` facade.

// Absent-only `dx init` scaffolding lives in the `scaffold` module
// (issue #236); the re-exports above keep `absent_only_write_allowed`,
// `init_must_refuse`, `ScaffoldFile`, `DEVCONTAINER_JSON`,
// `RENOVATE_JSON`, `plan_init_files`, and `apply_init` on the
// `dx_adopt` facade.

// Hermetic hook runner (`hook_git_is_hermetic`,
// `hook_shim_overwrite_allowed`, `hook_status_shows_merged`) lives in the
// `hooks` module (issue #236); the re-exports above keep those paths on
// the `dx_adopt` facade.

// ---------------------------------------------------------------------------
// Delivered I/O (M30b WPs 2-4, 6-7 + O61).
// ---------------------------------------------------------------------------

// Absent-only `dx init` scaffolding (`ScaffoldFile`, `DEVCONTAINER_JSON`,
// `RENOVATE_JSON`, `plan_init_files`, `apply_init`) lives in the
// `scaffold` module (issue #236); the re-exports above keep those paths
// on the `dx_adopt` facade.

// Hermetic hook I/O (`HOOK_MANAGED_MARKER`, `render_hook_shim`,
// `install_hooks`, `uninstall_hooks`, `render_hooks_status`) lives in the
// `hooks` module (issue #236); the re-exports above keep those paths on
// the `dx_adopt` facade.

// The consolidated `dx status` surface lives in the `status` module
// (issue #236); the re-exports above keep `StatusCheck`,
// `render_status_text`, `render_status_json`, and
// `default_status_checks` on the `dx_adopt` facade.

// Completion vocabulary lives in the `completion` module (issue #236):
// production rendering uses the `Cli` grammar, the tables there remain
// the O61 frozen vocabulary reference only.
