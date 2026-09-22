//! Local-only remote/cache abstraction for BEP collection.
//!
//! Owning contract: `docs/quality/action-model.md#outputs-remote-cache-and-execution`.
//! See: `docs/environments/codegen.md`, `docs/product/scope.md`.
//!
//! Why a separate module: enabling remote/cache later flips one
//! [`RemoteConfig`] plus one downloader impl, never every call site.
//! Bazel stays authoritative for remote materialization; the CLI never
//! implements a second remote-cache downloader, so remote URIs fail
//! instead of triggering a network fetch.

use std::path::Path;

use super::{ArtifactReader, BepError};

/// Bazel remote flags the CLI never emits (local-only until qualified).
/// Single owner for the unwired-flag inventory checked by
/// `action_execution_cache_qualification`.
pub const UNWIRED_REMOTE_FLAGS: [&str; 3] = ["remote_cache", "remote_executor", "bes_backend"];

/// Remote/cache selection for one Bazel invocation. Today always
/// [`RemoteConfig::local`]; qualifying remote fills these plus the
/// matching toolchain inputs instead of editing call sites.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RemoteConfig {
    remote_cache: Option<String>,
    remote_executor: Option<String>,
    bes_backend: Option<String>,
}

impl RemoteConfig {
    /// Local-only selection: no remote cache, executor, or BES backend.
    pub fn local() -> Self {
        RemoteConfig {
            remote_cache: None,
            remote_executor: None,
            bes_backend: None,
        }
    }

    /// Reports whether the selection stays local-only.
    pub fn is_local_only(&self) -> bool {
        self.remote_cache.is_none() && self.remote_executor.is_none() && self.bes_backend.is_none()
    }

    /// Fails when any remote endpoint is set (local-only gate).
    /// Future qualification replaces this gate with endpoint wiring.
    pub fn validate_local_only(&self) -> Result<(), BepError> {
        if self.is_local_only() {
            return Ok(());
        }
        let uri = self
            .remote_cache
            .clone()
            .or(self.remote_executor.clone())
            .or(self.bes_backend.clone())
            .unwrap_or_default();
        Err(BepError::UnsupportedUri { uri })
    }
}

/// Downloader seam for BEP-reported artifacts. The local-only impl
/// ([`LocalDownloader`]) reads Bazel-materialized `file://` paths;
/// a future remote impl downloads `bytestream://` URIs behind a
/// qualified [`RemoteConfig`]. Call sites depend on this trait, never
/// on a concrete reader, so enabling remote adds one impl.
pub trait Downloader {
    /// Reads one Bazel-materialized local artifact.
    fn fetch_local(&self, path: &Path) -> std::io::Result<Vec<u8>>;
}

/// Local-only downloader: the single [`Downloader`] plus
/// [`ArtifactReader`] impl used by collection. Performs no network
/// fetch; unmaterialized remote URIs already fail at URI parsing
/// before this reader is reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LocalDownloader;

impl Downloader for LocalDownloader {
    fn fetch_local(&self, path: &Path) -> std::io::Result<Vec<u8>> {
        std::fs::read(path)
    }
}

impl ArtifactReader for LocalDownloader {
    fn read_artifact(&self, path: &Path) -> std::io::Result<Vec<u8>> {
        self.fetch_local(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_config_stays_local_only() {
        let config = RemoteConfig::local();
        assert!(config.is_local_only());
        assert_eq!(config.validate_local_only(), Ok(()));
    }

    #[test]
    fn any_endpoint_breaks_local_only() {
        for config in [
            RemoteConfig {
                remote_cache: Some("https://cache.example".to_owned()),
                ..RemoteConfig::local()
            },
            RemoteConfig {
                remote_executor: Some("https://exec.example".to_owned()),
                ..RemoteConfig::local()
            },
            RemoteConfig {
                bes_backend: Some("https://bes.example".to_owned()),
                ..RemoteConfig::local()
            },
        ] {
            assert!(!config.is_local_only());
            assert!(matches!(
                config.validate_local_only(),
                Err(BepError::UnsupportedUri { .. })
            ));
        }
    }

    #[test]
    fn unwired_flags_name_the_remote_boundary() {
        assert_eq!(
            UNWIRED_REMOTE_FLAGS,
            ["remote_cache", "remote_executor", "bes_backend"]
        );
    }

    #[test]
    fn local_downloader_reads_files_without_network() {
        let dir = tempfile::TempDir::new().expect("scratch");
        let path = dir.path().join("a.pb");
        std::fs::write(&path, b"bytes").expect("write");
        let reader = LocalDownloader;
        assert_eq!(
            reader.read_artifact(&path).expect("read"),
            b"bytes".to_vec()
        );
        assert_eq!(reader.fetch_local(&path).expect("fetch"), b"bytes".to_vec());
        assert!(reader
            .read_artifact(&dir.path().join("missing.pb"))
            .is_err());
        // No network path exists: a remote URI never reaches the reader
        // because `file_uri_to_path` rejects it first.
        assert!(matches!(
            super::super::file_uri_to_path("bytestream://remote/cache/a.pb"),
            Err(BepError::UnsupportedUri { .. })
        ));
    }
}
