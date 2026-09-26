use std::path::Path;

use super::{ArtifactReader, BepError};

pub const UNWIRED_REMOTE_FLAGS: [&str; 3] = ["remote_cache", "remote_executor", "bes_backend"];

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RemoteConfig {
    remote_cache: Option<String>,
    remote_executor: Option<String>,
    bes_backend: Option<String>,
}

impl RemoteConfig {
    pub fn local() -> Self {
        RemoteConfig {
            remote_cache: None,
            remote_executor: None,
            bes_backend: None,
        }
    }

    pub fn is_local_only(&self) -> bool {
        self.remote_cache.is_none() && self.remote_executor.is_none() && self.bes_backend.is_none()
    }

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

pub trait Downloader {
    fn fetch_local(&self, path: &Path) -> std::io::Result<Vec<u8>>;
}

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
        assert!(matches!(
            super::super::file_uri_to_path("bytestream://remote/cache/a.pb"),
            Err(BepError::UnsupportedUri { .. })
        ));
    }
}
