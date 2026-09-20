//! Unified digest identities for `rules_dx`.
//!
//! Split history: `quality/result` + `generation/result` + `cli/env`
//! (`identity_digest`) + `cli/setup` hashed with BLAKE3-256 while
//! `dx_apply::envelope` hashed with SHA-256, plus hand `hex_digest` /
//! `parse_digest` helpers in `dx_cli` and `dx_output` and a repeated
//! `DIGEST_LEN = 32` constant.
//!
//! This crate is the single owner of the digest algorithm surface:
//!
//! * [`DIGEST_LEN`] — 32-byte digest length shared by both algorithms.
//! * [`Digest`] — `[u8; 32]` wrapper with hex/parse helpers.
//! * [`blake3`] — canonical content identity (snapshots, setup, env).
//! * [`sha256_hex`] / [`is_sha256_hex`] — compat shim for the frozen
//!   `dx_apply` envelope contract (`original_sha256` stays SHA-256 hex).
//! * [`to_hex`] / [`parse_hex`] — lowercase-hex spelling shared by CLI,
//!   output, and clean/setup record names.
//!
//! Canonical algorithm is BLAKE3-256. The SHA-256 envelope bytes are a
//! frozen contract and are kept byte-identical through this shim; see the
//! migration note on [`sha256_hex`].
//!
//! Dependency evaluation (adopted): hex spelling and parsing use
//! the upstream `hex` crate (`hex::encode`/`hex::decode` plus the length and
//! lowercase re-encode check, no manual digit loop), content identity uses
//! upstream `blake3`, and the envelope shim uses upstream `sha2`. The
//! `[u8; 32]` wrapper plus lowercase-only policy stays hand-rolled because it
//! is the repo's digest-identity contract, not an upstream type.

// Infallible paths must not `expect`/`unwrap` outside tests
// (`cfg_attr(not(test))` keeps `rust_test` bodies ergonomic).
#![cfg_attr(not(test), deny(clippy::expect_used, clippy::unwrap_used))]

use sha2::{Digest as _, Sha256};

/// Digest length in bytes (BLAKE3-256 and SHA-256 are both 32 bytes).
pub const DIGEST_LEN: usize = 32;

/// Raw 32-byte digest.
pub type RawDigest = [u8; DIGEST_LEN];

/// Digest parse/validation failure.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DigestError {
    /// Text is not 64 lowercase hex digits.
    #[error("invalid digest {value:?}: want 64-character lowercase hex")]
    BadDigest { value: String },
}

/// Fixed 32-byte digest with hex helpers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Digest(pub RawDigest);

impl Digest {
    /// Wraps raw bytes.
    pub fn new(bytes: RawDigest) -> Self {
        Self(bytes)
    }

    /// Raw bytes.
    pub fn as_bytes(&self) -> &RawDigest {
        &self.0
    }

    /// Consumes into raw bytes.
    pub fn into_bytes(self) -> RawDigest {
        self.0
    }

    /// BLAKE3-256 over `bytes`: the canonical content identity.
    pub fn blake3(bytes: &[u8]) -> Self {
        Self(blake3(bytes))
    }

    /// Lowercase hex spelling.
    pub fn to_hex(&self) -> String {
        to_hex(&self.0)
    }

    /// Parses exactly 64 lowercase hex digits.
    pub fn parse_hex(text: &str) -> Result<Self, DigestError> {
        parse_hex(text).map(Self)
    }
}

impl From<RawDigest> for Digest {
    fn from(bytes: RawDigest) -> Self {
        Self(bytes)
    }
}

impl From<Digest> for RawDigest {
    fn from(digest: Digest) -> Self {
        digest.0
    }
}

/// BLAKE3-256 over exact bytes: the canonical snapshot/setup/env identity.
/// There is no algorithm negotiation.
pub fn blake3(bytes: &[u8]) -> RawDigest {
    *blake3::hash(bytes).as_bytes()
}

/// Lowercase hex SHA-256 of `bytes`.
///
/// Compat shim for the frozen `dx_apply` envelope contract
/// (`original_sha256`): envelope bytes stay SHA-256 while every new
/// identity uses [`blake3`]. Do not use for new identities.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

/// True for exactly 64 lowercase hex digits (the [`sha256_hex`] and
/// [`to_hex`] output form).
///
/// Decode round-trip: `hex` accepts any even-length hex (including
/// uppercase), so the re-encode comparison is what pins the lowercase-only,
/// 32-byte form instead of re-implementing the digit loop.
pub fn is_hex(text: &str) -> bool {
    match hex::decode(text) {
        Ok(bytes) => bytes.len() == DIGEST_LEN && hex::encode(&bytes) == text,
        Err(_) => false,
    }
}

/// Compat alias for the envelope spelling check.
pub fn is_sha256_hex(text: &str) -> bool {
    is_hex(text)
}

/// Lowercase hexadecimal over 32 raw digest bytes.
pub fn to_hex(bytes: &RawDigest) -> String {
    hex::encode(bytes)
}

/// Lowercase hexadecimal over a byte slice (convenience for CLI helpers
/// that previously hex-encoded `&[u8]`).
pub fn to_hex_bytes(bytes: &[u8]) -> String {
    hex::encode(bytes)
}

/// Parses exactly 64 lowercase hex digits into 32 raw bytes.
///
/// Single `hex::decode` plus the length and lowercase re-encode check
/// (the same contract as [`is_hex`]); no manual digit loop.
pub fn parse_hex(text: &str) -> Result<RawDigest, DigestError> {
    let bad = || DigestError::BadDigest {
        value: text.to_owned(),
    };
    let bytes = hex::decode(text).map_err(|_| bad())?;
    if bytes.len() != DIGEST_LEN || hex::encode(&bytes) != text {
        return Err(bad());
    }
    bytes.try_into().map_err(|_| bad())
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        /// Property pilot: hex encoding round-trips every
        /// 32-byte input, and the wrapper agrees with the free functions.
        /// Fixed official vectors stay as plain asserts above; properties
        /// own the shape (`is_hex`, length, round-trip).
        #[test]
        fn hex_round_trip_any_bytes(bytes in prop::array::uniform32(any::<u8>())) {
            let hex = to_hex(&bytes);
            prop_assert_eq!(hex.len(), 64);
            prop_assert!(is_hex(&hex));
            prop_assert!(is_sha256_hex(&hex));
            prop_assert_eq!(parse_hex(&hex), Ok(bytes));
            let wrapped = Digest::new(bytes);
            prop_assert_eq!(wrapped.to_hex(), hex.clone());
            prop_assert_eq!(Digest::parse_hex(&hex).unwrap().into_bytes(), bytes);
        }

        /// Uppercase spellings stay rejected whenever the encoding
        /// actually contains a hex letter; digit-only encodings are
        /// case-neutral, so they are assumed away.
        #[test]
        fn uppercase_rejected_when_letters_present(
            bytes in prop::array::uniform32(any::<u8>()),
        ) {
            let hex = to_hex(&bytes);
            prop_assume!(hex.chars().any(|c| c.is_ascii_alphabetic()));
            prop_assert!(!is_hex(&hex.to_uppercase()));
            prop_assert!(parse_hex(&hex.to_uppercase()).is_err());
        }

        /// Length and alphabet mutations of a valid encoding never parse.
        #[test]
        fn mutated_encodings_rejected(
            bytes in prop::array::uniform32(any::<u8>()),
            idx in 0..64usize,
        ) {
            let hex = to_hex(&bytes);
            let extended = format!("{hex}00");
            prop_assert!(parse_hex(&extended).is_err());
            let mut chars: Vec<char> = hex.chars().collect();
            chars[idx] = 'z';
            let mutated: String = chars.into_iter().collect();
            prop_assert!(parse_hex(&mutated).is_err());
            prop_assert!(!is_hex(&mutated));
        }
    }

    #[test]
    fn blake3_empty_matches_official_vector() {
        // BLAKE3-team/BLAKE3 test_vectors.json, input_len 0, hash prefix.
        let expected = [
            0xaf, 0x13, 0x49, 0xb9, 0xf5, 0xf9, 0xa1, 0xa6, 0xa0, 0x40, 0x4d, 0xea, 0x36, 0xdc,
            0xc9, 0x49, 0x9b, 0xcb, 0x25, 0xc9, 0xad, 0xc1, 0x12, 0xb7, 0xcc, 0x9a, 0x93, 0xca,
            0xe4, 0x1f, 0x32, 0x62,
        ];
        assert_eq!(blake3(b""), expected);
        assert_eq!(Digest::blake3(b"").into_bytes(), expected);
    }

    #[test]
    fn sha256_known_vector() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn hex_round_trip() {
        let bytes = blake3(b"hello");
        let hex = to_hex(&bytes);
        assert_eq!(hex.len(), 64);
        assert_eq!(parse_hex(&hex).expect("round trip"), bytes);
        assert_eq!(
            Digest::parse_hex(&hex).expect("wrapper").into_bytes(),
            bytes
        );
        assert_eq!(Digest::new(bytes).to_hex(), hex);
    }

    #[test]
    fn hex_spellings_rejected() {
        let good = to_hex(&blake3(b"x"));
        assert!(is_hex(&good));
        assert!(is_sha256_hex(&good));
        assert!(parse_hex(&good).is_ok());
        assert!(parse_hex(&good.to_uppercase()).is_err());
        assert!(!is_hex(&good.to_uppercase()));
        assert!(parse_hex("0123").is_err());
        assert!(parse_hex(&format!("{good}00")).is_err());
        assert!(parse_hex(&"zz".repeat(32)).is_err());
        assert_eq!(
            parse_hex("xyz"),
            Err(DigestError::BadDigest {
                value: "xyz".to_owned()
            })
        );
    }

    #[test]
    fn display_is_human_readable() {
        let rendered = format!(
            "{}",
            DigestError::BadDigest {
                value: "xyz".to_owned()
            }
        );
        assert!(rendered.contains("64-character lowercase hex"));
    }
}
