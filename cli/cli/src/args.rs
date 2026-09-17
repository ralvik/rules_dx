//! Invocation parsing for the `dx` quality, workflow, run, clean, and
//! managed environment/codegen/setup commands (M07 WP1+WP3, M08 WP1+WP4,
//! M25 WP5).
//!
//! Contract: `docs/cli/cli-contract.md#invocation-shape`. Scope positionals
//! accept explicit Bazel labels and patterns (`//...`, `//pkg:target`,
//! `@repo//pkg/...`) as well as workspace-relative file and directory
//! paths. Package-relative labels (`:target`) and empty scopes fail with
//! [`ArgsError::RelativeLabel`] and [`ArgsError::EmptyScope`];
//! external-repository scopes parse but
//! fail during resolution, and file ownership resolves through Bazel
//! query per `docs/cli/target-resolution.md`. With no scope the
//! repository operation (`//...`) runs.
//!
//! `dx clean` takes no scopes: it prunes validated unselected managed
//! state per `docs/cli/commands/check-fix-clean.md#dx-clean`, with
//! `--dry-run` listing without deleting and `--bazel` additionally
//! forwarding `bazel clean`.
//!
//! Domain split (issue #236): the command vocabulary lives in the
//! `command` module, the `clap` grammar (`Cli`, `VALUE_OPTIONS`,
//! `cli_command`) in the `grammar` module, shell-completion rendering in
//! the `completion` module, help rendering in the `help` module, typo
//! suggestions in the `suggest` module, build-profile vocabulary in the
//! `profile` module, shared invocation types in the `invocation` module,
//! error vocabulary in the `error` module, and invocation parsing in the
//! `parser` module (the `parse` domain; named `parser` so the module
//! and the `parse` function coexist), with the small value helpers
//! (`scope_error`, `parse_report`, `parse_min_coverage`) in the
//! `values` module and the Bazel-verbatim tokenizer
//! and `clap`-error mapping in the `tokenizer` module. This facade keeps the re-exports;
//! the public paths stay `crate::args::Command`,
//! `crate::args::{COMPLETION_SHELLS, render_completion}`,
//! `crate::args::{Invocation, ReportRequest}`,
//! `crate::args::ArgsError`, and
//! `crate::args::{parse, cli_command}` via the re-exports below.

pub mod command;
pub mod completion;
pub mod error;
pub mod grammar;
pub mod help;
pub mod invocation;
pub mod parser;
pub mod profile;
pub mod suggest;
pub mod tokenizer;
pub mod values;

pub use command::Command;
pub use completion::{render_completion, COMPLETION_SHELLS};
pub use error::ArgsError;
pub use grammar::cli_command;
pub use invocation::{Invocation, ReportRequest};
pub use parser::parse;
pub use profile::{resolve_profile, Profile, DX_PROFILE_ENV};
