//! Install-time publisher-identity verifier for standalone `dx`.
//!
//! Owning contract: `docs/deploy/authoring.md` (Path H).
//! There is no checksum-only fallback: a sha256 alone never proves identity.

// Infallible paths must not `expect`/`unwrap` outside tests
// (`cfg_attr(not(test))` keeps `rust_test` bodies ergonomic).
#![cfg_attr(not(test), deny(clippy::expect_used, clippy::unwrap_used))]

use std::io;
use std::path::{Path, PathBuf};

/// Sigstore TUF trust root.
/// See: `deploy/install/dx_verify.sh` (`TRUST_ROOT`).
pub const TRUST_ROOT: &str = "https://tuf-repo-cdn.sigstore.dev";
/// Default OIDC issuer.
pub const DEFAULT_ISSUER: &str = "https://token.actions.githubusercontent.com";

/// Parsed verifier arguments.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VerifyArgs {
    /// Standalone binary under verification.
    pub binary: String,
    /// Sigstore bundle binding the binary bytes.
    pub bundle: String,
    /// Expected certificate identity.
    pub identity: String,
    /// Expected OIDC issuer.
    pub issuer: String,
    /// Optional GitHub attestation path.
    pub attestation: String,
    /// Owner for the attestation path.
    pub owner: String,
    /// Optional SBOM file.
    pub sbom: String,
    /// Bundle binding the SBOM bytes.
    pub sbom_bundle: String,
    /// Optional install directory (copy only, never exec).
    pub install_dir: String,
}

/// Argument parsing failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArgsError(pub String);

impl std::fmt::Display for ArgsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ArgsError {}

fn help_text() -> String {
    "usage: dx_verify --binary PATH --bundle PATH --identity ID --issuer ISSUER [--attestation PATH] [--owner OWNER] [--sbom PATH --sbom-bundle PATH] [--install-dir DIR]".to_owned()
}

/// Parses verifier argv, rejecting checksum-only flags.
pub fn parse_args(argv: &[String]) -> Result<VerifyArgs, ArgsError> {
    let mut args = VerifyArgs::default();
    let mut i = 1;
    while i < argv.len() {
        match argv[i].as_str() {
            "--binary" => {
                args.binary = argv.get(i + 1).cloned().unwrap_or_default();
                i += 2;
            }
            "--bundle" => {
                args.bundle = argv.get(i + 1).cloned().unwrap_or_default();
                i += 2;
            }
            "--identity" => {
                args.identity = argv.get(i + 1).cloned().unwrap_or_default();
                i += 2;
            }
            "--issuer" => {
                args.issuer = argv.get(i + 1).cloned().unwrap_or_default();
                i += 2;
            }
            "--attestation" => {
                args.attestation = argv.get(i + 1).cloned().unwrap_or_default();
                i += 2;
            }
            "--owner" => {
                args.owner = argv.get(i + 1).cloned().unwrap_or_default();
                i += 2;
            }
            "--sbom" => {
                args.sbom = argv.get(i + 1).cloned().unwrap_or_default();
                i += 2;
            }
            "--sbom-bundle" => {
                args.sbom_bundle = argv.get(i + 1).cloned().unwrap_or_default();
                i += 2;
            }
            "--install-dir" => {
                args.install_dir = argv.get(i + 1).cloned().unwrap_or_default();
                i += 2;
            }
            "--sha256" | "--checksum" | "--sha256-file" | "--checksum-file" => {
                return Err(ArgsError(
                    "dx_verify: checksum-only verification is not publisher-identity proof; pass --bundle plus --identity/--issuer (issue #26)".to_owned(),
                ));
            }
            "-h" | "--help" => {
                return Err(ArgsError(help_text()));
            }
            other => {
                return Err(ArgsError(format!(
                    "dx_verify: unknown argument '{other}'\n{}",
                    help_text()
                )));
            }
        }
    }
    if args.binary.is_empty() {
        return Err(ArgsError(
            "dx_verify: missing --binary PATH (standalone dx binary to verify)".to_owned(),
        ));
    }
    if args.bundle.is_empty() {
        return Err(ArgsError(
            "dx_verify: missing --bundle PATH; checksum-only verification is not publisher-identity proof (issue #26)".to_owned(),
        ));
    }
    if args.identity.is_empty() {
        return Err(ArgsError(
            "dx_verify: missing --identity ID (expected certificate identity, e.g. GitHub Actions workflow identity)".to_owned(),
        ));
    }
    if args.issuer.is_empty() {
        return Err(ArgsError(
            "dx_verify: missing --issuer ISSUER (expected OIDC issuer, e.g. https://token.actions.githubusercontent.com)".to_owned(),
        ));
    }
    if !args.sbom.is_empty() && args.sbom_bundle.is_empty() {
        return Err(ArgsError(
            "dx_verify: --sbom needs --sbom-bundle (SBOM verifies through the same cosign path)"
                .to_owned(),
        ));
    }
    if args.sbom.is_empty() && !args.sbom_bundle.is_empty() {
        return Err(ArgsError(
            "dx_verify: --sbom-bundle needs --sbom".to_owned(),
        ));
    }
    Ok(args)
}

/// Filesystem plus subprocess seam for unit tests.
pub trait Verifier {
    /// Reports whether `path` is a non-empty regular file.
    fn is_nonempty_file(&self, path: &Path) -> bool;
    /// Reports whether `path` exists as a file.
    fn is_file(&self, path: &Path) -> bool;
    /// Runs `cosign verify-blob` for one subject.
    fn cosign_verify(
        &self,
        cosign: &str,
        bundle: &Path,
        identity: &str,
        issuer: &str,
        subject: &Path,
    ) -> bool;
    /// Runs `gh attestation verify` for one subject.
    fn gh_verify(&self, gh: &str, subject: &Path, owner: &str) -> bool;
    /// Resolves the cosign executable name.
    fn cosign_bin(&self) -> String;
    /// Resolves the gh executable name.
    fn gh_bin(&self) -> String;
    /// Reports whether an executable resolves on PATH.
    fn have(&self, bin: &str) -> bool;
    /// SHA-256 hex of one file.
    fn sha256_file(&self, path: &Path) -> io::Result<String>;
    /// Copies one file, preserving bytes.
    fn install_copy(&self, src: &Path, dst: &Path) -> io::Result<()>;
}

/// Real verifier using the host filesystem plus subprocesses.
pub struct SystemVerifier;

impl Verifier for SystemVerifier {
    fn is_nonempty_file(&self, path: &Path) -> bool {
        std::fs::metadata(path).is_ok_and(|meta| meta.is_file() && meta.len() > 0)
    }

    fn is_file(&self, path: &Path) -> bool {
        path.is_file()
    }

    fn cosign_verify(
        &self,
        cosign: &str,
        bundle: &Path,
        identity: &str,
        issuer: &str,
        subject: &Path,
    ) -> bool {
        std::process::Command::new(cosign)
            .args([
                "verify-blob",
                "--bundle",
                &bundle.to_string_lossy(),
                "--certificate-identity",
                identity,
                "--certificate-issuer",
                issuer,
                &subject.to_string_lossy(),
            ])
            .output()
            .is_ok_and(|out| out.status.success())
    }

    fn gh_verify(&self, gh: &str, subject: &Path, owner: &str) -> bool {
        std::process::Command::new(gh)
            .args([
                "attestation",
                "verify",
                &subject.to_string_lossy(),
                "--owner",
                owner,
            ])
            .output()
            .is_ok_and(|out| out.status.success())
    }

    fn cosign_bin(&self) -> String {
        std::env::var("DX_VERIFY_COSIGN").unwrap_or_else(|_| "cosign".to_owned())
    }

    fn gh_bin(&self) -> String {
        std::env::var("DX_VERIFY_GH").unwrap_or_else(|_| "gh".to_owned())
    }

    fn have(&self, bin: &str) -> bool {
        std::process::Command::new(bin)
            .arg("--help")
            .output()
            .is_ok_and(|out| out.status.success())
            || Path::new(bin).is_file()
            || {
                std::env::var_os("PATH").is_some_and(|paths| {
                    std::env::split_paths(&paths).any(|dir| {
                        dir.join(bin).is_file() || dir.join(format!("{bin}.exe")).is_file()
                    })
                })
            }
    }

    fn sha256_file(&self, path: &Path) -> io::Result<String> {
        use sha2::Digest as _;
        let mut hasher = sha2::Sha256::new();
        let mut file = std::fs::File::open(path)?;
        let mut buf = [0u8; 1 << 20];
        loop {
            use std::io::Read as _;
            let read = file.read(&mut buf)?;
            if read == 0 {
                break;
            }
            hasher.update(&buf[..read]);
        }
        Ok(hex::encode(hasher.finalize()))
    }

    fn install_copy(&self, src: &Path, dst: &Path) -> io::Result<()> {
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(src, dst)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            std::fs::set_permissions(dst, std::fs::Permissions::from_mode(0o755))?;
        }
        Ok(())
    }
}

/// Verifies one standalone binary, returning the success transcript.
pub fn verify(args: &VerifyArgs, verifier: &dyn Verifier) -> Result<String, String> {
    let mut out = String::new();
    let binary = PathBuf::from(&args.binary);
    let bundle = PathBuf::from(&args.bundle);
    if !verifier.is_file(&binary) {
        return Err(format!("dx_verify: binary not found: {}", args.binary));
    }
    if !verifier.is_file(&bundle) {
        return Err(format!(
            "dx_verify: bundle not found: {} (missing, invalid, or unavailable verification inputs fail closed)",
            args.bundle
        ));
    }
    if !args.attestation.is_empty() && !verifier.is_file(Path::new(&args.attestation)) {
        return Err(format!(
            "dx_verify: attestation not found: {}",
            args.attestation
        ));
    }
    if !args.sbom.is_empty() && !verifier.is_file(Path::new(&args.sbom)) {
        return Err(format!("dx_verify: sbom not found: {}", args.sbom));
    }
    if !args.sbom_bundle.is_empty() && !verifier.is_file(Path::new(&args.sbom_bundle)) {
        return Err(format!(
            "dx_verify: sbom bundle not found: {}",
            args.sbom_bundle
        ));
    }
    if !verifier.is_nonempty_file(&bundle) {
        return Err(format!("dx_verify: bundle is empty: {}", args.bundle));
    }
    out.push_str(&format!(
        "dx_verify: trust root {TRUST_ROOT} (Sigstore TUF public-good; verifiers bootstrapped from the trust root, never alongside the binary)\n"
    ));
    let cosign = verifier.cosign_bin();
    let gh = verifier.gh_bin();
    let verified: String;
    if verifier.have(&cosign) {
        out.push_str(&format!(
            "dx_verify: verifying {} against bundle via {cosign} verify-blob\n",
            binary.display()
        ));
        if verifier.cosign_verify(&cosign, &bundle, &args.identity, &args.issuer, &binary) {
            verified = "cosign".to_owned();
            out.push_str(&format!(
                "dx_verify: cosign publisher-identity OK (identity={} issuer={})\n",
                args.identity, args.issuer
            ));
        } else {
            return Err(
                "dx_verify: cosign verification failed (tampered bytes, replaced binary/bundle, or unapproved signer fail closed)".to_owned(),
            );
        }
    } else if !args.attestation.is_empty() && verifier.have(&gh) {
        if args.owner.is_empty() {
            return Err(
                "dx_verify: --owner OWNER is required with --attestation (GitHub attestation path)"
                    .to_owned(),
            );
        }
        out.push_str(&format!(
            "dx_verify: verifying {} against attestation via {gh} attestation verify\n",
            binary.display()
        ));
        if verifier.gh_verify(&gh, &binary, &args.owner) {
            verified = "gh-attestation".to_owned();
            out.push_str(&format!(
                "dx_verify: attestation publisher-identity OK (owner={})\n",
                args.owner
            ));
        } else {
            return Err(
                "dx_verify: attestation verification failed (fail closed before install)"
                    .to_owned(),
            );
        }
    } else {
        return Err(format!(
            "dx_verify: no verifier available (want {cosign} verify-blob or {gh} attestation verify); missing, invalid, or unavailable verification inputs fail closed"
        ));
    }
    if !args.sbom.is_empty() {
        let sbom = PathBuf::from(&args.sbom);
        let sbom_bundle = PathBuf::from(&args.sbom_bundle);
        if !verifier.have(&cosign) {
            return Err(format!(
                "dx_verify: SBOM verification needs {cosign} (unavailable)"
            ));
        }
        out.push_str(&format!(
            "dx_verify: verifying SBOM {} via {cosign} verify-blob\n",
            sbom.display()
        ));
        if verifier.cosign_verify(&cosign, &sbom_bundle, &args.identity, &args.issuer, &sbom) {
            out.push_str("dx_verify: SBOM publisher-identity OK\n");
        } else {
            return Err(
                "dx_verify: SBOM verification failed (fail closed before install)".to_owned(),
            );
        }
    }
    if !args.install_dir.is_empty() {
        let base = binary
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("dx");
        let digest = verifier
            .sha256_file(&binary)
            .map_err(|error| format!("dx_verify: cannot hash {}: {error}", binary.display()))?;
        let dst = Path::new(&args.install_dir).join(base);
        verifier
            .install_copy(&binary, &dst)
            .map_err(|error| format!("dx_verify: cannot install {}: {error}", dst.display()))?;
        out.push_str(&format!(
            "dx_verify: installed {base} to {} (sha256 {digest}, verified via {verified})\n",
            args.install_dir
        ));
    } else {
        out.push_str(&format!(
            "dx_verify: OK (verified via {verified}; nothing installed, binary never executed)\n"
        ));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct FakeVerifier {
        files: HashMap<PathBuf, Vec<u8>>,
        cosign_ok: bool,
        gh_ok: bool,
        have_cosign: bool,
        have_gh: bool,
    }

    impl FakeVerifier {
        fn new() -> Self {
            Self {
                files: HashMap::new(),
                cosign_ok: true,
                gh_ok: true,
                have_cosign: true,
                have_gh: true,
            }
        }

        fn write(&mut self, path: &str, data: &[u8]) {
            self.files.insert(PathBuf::from(path), data.to_vec());
        }
    }

    impl Verifier for FakeVerifier {
        fn is_nonempty_file(&self, path: &Path) -> bool {
            self.files.get(path).is_some_and(|data| !data.is_empty())
        }

        fn is_file(&self, path: &Path) -> bool {
            self.files.contains_key(path)
        }

        fn cosign_verify(
            &self,
            _cosign: &str,
            bundle: &Path,
            _identity: &str,
            _issuer: &str,
            subject: &Path,
        ) -> bool {
            if !self.cosign_ok {
                return false;
            }
            // Bundle binds exact subject bytes like the shell stub.
            let subject_data = self.files.get(subject);
            let bundle_data = self.files.get(bundle);
            match (subject_data, bundle_data) {
                (Some(subject_bytes), Some(bundle_bytes)) => {
                    use sha2::Digest as _;
                    let mut hasher = sha2::Sha256::new();
                    hasher.update(subject_bytes);
                    let digest = hex::encode(hasher.finalize());
                    bundle_bytes == format!("bundle-for-{digest}").as_bytes()
                }
                _ => false,
            }
        }

        fn gh_verify(&self, _gh: &str, _subject: &Path, _owner: &str) -> bool {
            self.gh_ok
        }

        fn cosign_bin(&self) -> String {
            "cosign".to_owned()
        }

        fn gh_bin(&self) -> String {
            "gh".to_owned()
        }

        fn have(&self, bin: &str) -> bool {
            if bin == "cosign" {
                self.have_cosign
            } else {
                self.have_gh
            }
        }

        fn sha256_file(&self, path: &Path) -> io::Result<String> {
            use sha2::Digest as _;
            let data = self
                .files
                .get(path)
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "missing file"))?;
            let mut hasher = sha2::Sha256::new();
            hasher.update(data);
            Ok(hex::encode(hasher.finalize()))
        }

        fn install_copy(&self, _src: &Path, _dst: &Path) -> io::Result<()> {
            Ok(())
        }
    }

    fn valid_args() -> VerifyArgs {
        VerifyArgs {
            binary: "/tmp/dx-fake".to_owned(),
            bundle: "/tmp/dx-fake.bundle".to_owned(),
            identity: "IDENT".to_owned(),
            issuer: "ISSUER".to_owned(),
            ..Default::default()
        }
    }

    fn valid_fake() -> FakeVerifier {
        let mut fake = FakeVerifier::new();
        let subject = b"standalone-dx-bytes-v1";
        fake.write("/tmp/dx-fake", subject);
        use sha2::Digest as _;
        let mut hasher = sha2::Sha256::new();
        hasher.update(subject);
        let digest = hex::encode(hasher.finalize());
        fake.write(
            "/tmp/dx-fake.bundle",
            format!("bundle-for-{digest}").as_bytes(),
        );
        fake
    }

    #[test]
    fn parses_required_flags() {
        let argv = [
            "dx_verify",
            "--binary",
            "/b",
            "--bundle",
            "/u",
            "--identity",
            "ID",
            "--issuer",
            "ISS",
        ]
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
        let args = parse_args(&argv).expect("parse");
        assert_eq!(args.binary, "/b");
        assert_eq!(args.bundle, "/u");
        assert_eq!(args.identity, "ID");
        assert_eq!(args.issuer, "ISS");
    }

    #[test]
    fn rejects_checksum_only_and_missing_inputs() {
        let base = [
            "dx_verify",
            "--binary",
            "/b",
            "--bundle",
            "/u",
            "--identity",
            "ID",
            "--issuer",
            "ISS",
        ];
        let with_sha = base
            .iter()
            .chain(["--sha256", "/x"].iter())
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        let err = parse_args(&with_sha).expect_err("sha rejected");
        assert!(err.0.contains("checksum-only"));
        for (flag, want) in [
            ("--binary", "missing --binary"),
            ("--bundle", "missing --bundle"),
            ("--identity", "missing --identity"),
            ("--issuer", "missing --issuer"),
        ] {
            let mut argv: Vec<String> = base.iter().map(ToString::to_string).collect();
            let pos = argv.iter().position(|arg| arg == flag).expect("flag");
            argv.drain(pos..pos + 2);
            let err = parse_args(&argv).expect_err("missing rejected");
            assert!(err.0.contains(want), "want {want}, got {}", err.0);
        }
        let sbom_only = [
            "dx_verify",
            "--binary",
            "/b",
            "--bundle",
            "/u",
            "--identity",
            "ID",
            "--issuer",
            "ISS",
            "--sbom",
            "/s",
        ]
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
        assert!(parse_args(&sbom_only).is_err());
    }

    #[test]
    fn accepts_valid_and_installs_without_exec() {
        let args = valid_args();
        let fake = valid_fake();
        let out = verify(&args, &fake).expect("valid");
        assert!(out.contains("tuf-repo-cdn.sigstore.dev"));
        assert!(out.contains("cosign publisher-identity OK"));
        assert!(out.contains("nothing installed, binary never executed"));
    }

    #[test]
    fn rejects_tampered_and_unapproved_signer() {
        let args = valid_args();
        let mut fake = valid_fake();
        fake.write("/tmp/dx-fake", b"tampered");
        assert!(verify(&args, &fake).is_err());
        let mut fake = valid_fake();
        fake.cosign_ok = false;
        assert!(verify(&args, &fake).is_err());
    }

    #[test]
    fn fails_closed_without_verifier() {
        let args = valid_args();
        let mut fake = valid_fake();
        fake.have_cosign = false;
        fake.have_gh = false;
        let err = verify(&args, &fake).expect_err("no verifier");
        assert!(err.contains("no verifier available"));
    }

    #[test]
    fn binds_sbom_bytes() {
        let mut args = valid_args();
        args.sbom = "/tmp/sbom".to_owned();
        args.sbom_bundle = "/tmp/sbom.bundle".to_owned();
        let mut fake = valid_fake();
        fake.write("/tmp/sbom", b"sbom-bytes");
        use sha2::Digest as _;
        let mut hasher = sha2::Sha256::new();
        hasher.update(b"sbom-bytes");
        let digest = hex::encode(hasher.finalize());
        fake.write(
            "/tmp/sbom.bundle",
            format!("bundle-for-{digest}").as_bytes(),
        );
        let out = verify(&args, &fake).expect("sbom valid");
        assert!(out.contains("SBOM publisher-identity OK"));
        fake.write("/tmp/sbom", b"tampered-sbom");
        assert!(verify(&args, &fake).is_err());
    }

    #[test]
    fn installs_with_digest() {
        let mut args = valid_args();
        args.install_dir = "/tmp/install".to_owned();
        let fake = valid_fake();
        let out = verify(&args, &fake).expect("install");
        assert!(out.contains("installed dx-fake to /tmp/install"));
        assert!(out.contains("sha256"));
    }
}
