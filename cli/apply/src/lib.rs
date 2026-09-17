// LCOV_EXCL_START - reason: module root holds only mod declarations and re-exports with no executable statements; every item is covered in its own module.
//! `dx_apply`: validated, consensus-gated file mutations for the `dx` CLI.
//!
//! Agents propose mutations as JSON [`envelope`]s; two agents must reach
//! [`consensus`] before anything applies; every operation passes
//! [`validators`] and is written atomically by the [`applier`].

pub mod applier;
pub mod consensus;
pub mod envelope;
pub mod validators;

pub use applier::{
    apply_envelope, AppliedFile, ApplyError, ApplyReport, FileSystem, HookRunner, NoHooks,
    RealFileSystem,
};
pub use consensus::{merge, ConsensusError};
pub use envelope::{
    emit_envelope, is_sha256_hex, parse_envelope, sha256_hex, Envelope, EnvelopeError,
    FileOperation, ENVELOPE_VERSION,
};
pub use validators::{validate, ValidationError, BLOCKED_EXTENSIONS, MAX_OPERATION_BYTES};
// LCOV_EXCL_STOP - reason: end of re-export-only module root exclusion.
