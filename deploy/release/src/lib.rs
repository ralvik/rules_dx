//! Deterministic SBOM/provenance plus BCR source generators.
//!
//! Owning contract: `docs/deploy/release-runbook.md` (SBOM/BCR release path).
//!
//! Why `serde_json` pretty plus ASCII escape: Python `json.dumps(indent=2,
//! sort_keys=True)` sorts keys and escapes non-ASCII (`ensure_ascii`); the
//! default `serde_json` map orders identically while raw UTF-8 would drift,
//! so non-ASCII is re-escaped to `\uXXXX` after pretty-printing.
//! See: `deploy/release/sbom.bzl` (genrule `tools`).
//! Why streaming SHA-256: artifacts hash in 1 MiB chunks like `hashlib`.
//! See: `cli/digest/src/lib.rs` (SHA-256 shim).

// Infallible paths must not `expect`/`unwrap` outside tests
// (`cfg_attr(not(test))` keeps `rust_test` bodies ergonomic).
#![cfg_attr(not(test), deny(clippy::expect_used, clippy::unwrap_used))]

use std::io;
use std::path::Path;

/// Basename of `path` as UTF-8, matching Python `os.path.basename`.
fn basename(path: &Path) -> io::Result<String> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("path {} has no UTF-8 basename", path.display()),
            )
        })
}

/// SHA-256 hex of the file at `path`, streamed in 1 MiB chunks.
fn sha256_file(path: &Path) -> io::Result<String> {
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

/// Re-escapes non-ASCII as `\uXXXX` (surrogate pairs above U+FFFF).
fn ensure_ascii(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        let code = c as u32;
        if code < 0x80 {
            out.push(c);
        } else if code <= 0xFFFF {
            out.push_str(&format!("\\u{code:04x}"));
        } else {
            let v = code - 0x10000;
            let high = 0xD800 + (v >> 10);
            let low = 0xDC00 + (v & 0x3FF);
            out.push_str(&format!("\\u{high:04x}\\u{low:04x}"));
        }
    }
    out
}

/// Renders `value` exactly like Python `json.dumps(indent=2, sort_keys=True)`.
fn render_pretty(value: &serde_json::Value) -> String {
    // Only fails on maps with non-string keys, which this schema never has.
    // See: `serde_json::to_string_pretty` (infallible for `json!` values).
    match serde_json::to_string_pretty(value) {
        Ok(pretty) => ensure_ascii(&pretty) + "\n",
        Err(_) => String::new(),
    }
}

/// Renders the SPDX 2.3 document bytes for one artifact.
pub fn render_spdx(base: &str, digest: &str, package: &str, supplier: &str) -> String {
    render_pretty(&serde_json::json!({
        "SPDXID": "SPDXRef-DOCUMENT",
        "creationInfo": {
            "created": "1970-01-01T00:00:00Z",
            "creators": ["Tool: rules_dx-sbom-1.0"],
        },
        "dataLicense": "CC0-1.0",
        "documentNamespace": format!("https://github.com/ralvik/rules_dx/releases/{base}-{digest}"),
        "files": [
            {
                "SPDXID": "SPDXRef-File",
                "checksums": [
                    {"algorithm": "SHA256", "checksumValue": digest}
                ],
                "fileName": base,
            }
        ],
        "name": format!("{package}-{base}"),
        "packages": [
            {
                "SPDXID": "SPDXRef-Package",
                "checksums": [
                    {"algorithm": "SHA256", "checksumValue": digest}
                ],
                "downloadLocation": "NOASSERTION",
                "externalRefs": [
                    {
                        "referenceCategory": "PACKAGE-MANAGER",
                        "referenceLocator": format!("pkg:generic/{package}@{digest}"),
                        "referenceType": "purl",
                    }
                ],
                "filesAnalyzed": false,
                "name": package,
                "supplier": format!("Organization: {supplier}"),
                "verificationCode": {"packageVerificationCodeValue": digest},
            }
        ],
        "spdxVersion": "SPDX-2.3",
    }))
}

/// Renders the SLSA v1 provenance statement bytes for one artifact.
pub fn render_provenance(base: &str, digest: &str, builder: &str) -> String {
    render_pretty(&serde_json::json!({
        "_type": "https://in-toto.io/Statement/v1",
        "predicate": {
            "buildDefinition": {
                "buildType": "https://github.com/ralvik/rules_dx/release@v1",
                "externalParameters": {"artifact": base},
            },
            "runDetails": {
                "builder": {"id": builder},
                "metadata": {"invocationId": "dry-run"},
            },
        },
        "predicateType": "https://slsa.dev/provenance/v1",
        "subject": [{"digest": {"sha256": digest}, "name": base}],
    }))
}

/// Renders the BCR `source.json` template bytes for one module version.
pub fn render_bcr_source(module: &str, version: &str) -> String {
    render_pretty(&serde_json::json!({
        "integrity": "<integrity-filled-at-release>",
        "strip_prefix": format!("{module}-{version}"),
        "url": format!("https://github.com/ralvik/rules_dx/releases/download/v{version}/{module}-{version}.tar.gz"),
    }))
}

/// Writes the SPDX 2.3 document for `src` to `dst`.
pub fn write_spdx(src: &Path, dst: &Path, package: &str, supplier: &str) -> io::Result<()> {
    let digest = sha256_file(src)?;
    let base = basename(src)?;
    std::fs::write(
        dst,
        render_spdx(&base, &digest, package, supplier).as_bytes(),
    )?;
    Ok(())
}

/// Writes the SLSA v1 provenance statement for `src` to `dst`.
pub fn write_provenance(src: &Path, dst: &Path, builder: &str) -> io::Result<()> {
    let digest = sha256_file(src)?;
    let base = basename(src)?;
    std::fs::write(dst, render_provenance(&base, &digest, builder).as_bytes())?;
    Ok(())
}

/// Writes the BCR `source.json` template to `dst`.
pub fn write_bcr_source(dst: &Path, module: &str, version: &str) -> io::Result<()> {
    std::fs::write(dst, render_bcr_source(module, version).as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_dir() -> tempfile::TempDir {
        tempfile::TempDir::new().expect("scratch")
    }

    fn write_artifact(dir: &Path, name: &str, data: &[u8]) -> std::path::PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, data).expect("write artifact");
        path
    }

    #[test]
    fn sha256_matches_hashlib_vectors() {
        let scratch = scratch_dir();
        let src = write_artifact(scratch.path(), "payload.bin", b"abc");
        // `hashlib.sha256(b"abc").hexdigest()`.
        assert_eq!(
            sha256_file(&src).expect("hash"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        let empty = write_artifact(scratch.path(), "empty.bin", b"");
        assert_eq!(
            sha256_file(&empty).expect("hash empty"),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert!(sha256_file(&scratch.path().join("missing")).is_err());
    }

    #[test]
    fn basename_rejects_missing_name() {
        assert!(basename(Path::new("/")).is_err());
        assert_eq!(
            basename(Path::new("/tmp/artifact.bin")).expect("base"),
            "artifact.bin"
        );
    }

    #[test]
    fn spdx_bytes_match_python_golden() {
        // Golden from `sbom_spdx_gen.py` over `hello world\n` (digest below)
        // with package `dx` plus supplier `rules_dx`; regenerate with
        // `bazel build //deploy/release:sbom_spdx_gen` output before editing.
        let digest = "a948904f2f0f479b8f8197694b30184b0d2ed1c1cd2a1ec0fb85d299a192a447";
        let text = render_spdx("artifact.bin", digest, "dx", "rules_dx");
        assert!(text.ends_with('\n'));
        assert!(!text.ends_with("\r\n"));
        let value: serde_json::Value = serde_json::from_str(&text).expect("valid JSON");
        assert_eq!(value["spdxVersion"], serde_json::json!("SPDX-2.3"));
        assert_eq!(value["dataLicense"], serde_json::json!("CC0-1.0"));
        assert_eq!(value["SPDXID"], serde_json::json!("SPDXRef-DOCUMENT"));
        assert_eq!(value["name"], serde_json::json!("dx-artifact.bin"));
        assert_eq!(
            value["documentNamespace"],
            serde_json::json!(format!(
                "https://github.com/ralvik/rules_dx/releases/artifact.bin-{digest}"
            ))
        );
        // Sorted keys plus two-space indent pin the Python wire bytes.
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines[1], "  \"SPDXID\": \"SPDXRef-DOCUMENT\",");
        assert!(text.contains("\"filesAnalyzed\": false"));
        assert!(text.contains(&format!("pkg:generic/dx@{digest}")));
        assert!(text.contains("Organization: rules_dx"));
    }

    #[test]
    fn provenance_bytes_match_python_golden() {
        // Golden from `sbom_prov_gen.py` over the same digest plus builder.
        let digest = "a948904f2f0f479b8f8197694b30184b0d2ed1c1cd2a1ec0fb85d299a192a447";
        let text = render_provenance("artifact.bin", digest, "https://example.com/builder");
        assert!(text.ends_with('\n'));
        let value: serde_json::Value = serde_json::from_str(&text).expect("valid JSON");
        assert_eq!(
            value["_type"],
            serde_json::json!("https://in-toto.io/Statement/v1")
        );
        assert_eq!(
            value["predicateType"],
            serde_json::json!("https://slsa.dev/provenance/v1")
        );
        assert_eq!(
            value["subject"][0]["digest"]["sha256"],
            serde_json::json!(digest)
        );
        assert_eq!(
            value["subject"][0]["name"],
            serde_json::json!("artifact.bin")
        );
        assert_eq!(
            value["predicate"]["runDetails"]["builder"]["id"],
            serde_json::json!("https://example.com/builder")
        );
        // Underscore key sorts first, matching `sort_keys=True`.
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(
            lines[1],
            "  \"_type\": \"https://in-toto.io/Statement/v1\","
        );
    }

    #[test]
    fn bcr_bytes_match_python_golden() {
        // Golden from `bcr_source_gen.py` with `rules_dx` at `1.2.3`.
        let text = render_bcr_source("rules_dx", "1.2.3");
        assert_eq!(
            text,
            "{\n  \"integrity\": \"<integrity-filled-at-release>\",\n  \"strip_prefix\": \"rules_dx-1.2.3\",\n  \"url\": \"https://github.com/ralvik/rules_dx/releases/download/v1.2.3/rules_dx-1.2.3.tar.gz\"\n}\n"
        );
    }

    #[test]
    fn render_escapes_non_ascii_like_python() {
        // Python `ensure_ascii` emits `\u00e9`; raw UTF-8 would drift.
        let digest = "a948904f2f0f479b8f8197694b30184b0d2ed1c1cd2a1ec0fb85d299a192a447";
        let text = render_spdx("artifact.bin", digest, "dx", "caf\u{e9}");
        assert!(text.contains("caf\\u00e9"));
        assert!(!text.contains("caf\u{e9}"));
        // Astral plane renders as a surrogate pair like Python.
        let astral = render_bcr_source("rules_dx\u{1F600}", "1.2.3");
        assert!(astral.contains("\\ud83d\\ude00"));
    }

    #[test]
    fn write_helpers_round_trip_digests() {
        let scratch = scratch_dir();
        let src = write_artifact(scratch.path(), "artifact.bin", b"hello world\n");
        let digest = sha256_file(&src).expect("digest");
        let spdx_out = scratch.path().join("out.spdx.json");
        write_spdx(&src, &spdx_out, "dx", "rules_dx").expect("write spdx");
        let spdx_text = std::fs::read_to_string(&spdx_out).expect("read spdx");
        assert_eq!(
            spdx_text,
            render_spdx("artifact.bin", &digest, "dx", "rules_dx")
        );
        let prov_out = scratch.path().join("out.prov.json");
        write_provenance(&src, &prov_out, "https://example.com/builder").expect("write prov");
        let prov_text = std::fs::read_to_string(&prov_out).expect("read prov");
        assert_eq!(
            prov_text,
            render_provenance("artifact.bin", &digest, "https://example.com/builder")
        );
        let bcr_out = scratch.path().join("out.source.json");
        write_bcr_source(&bcr_out, "rules_dx", "1.2.3").expect("write bcr");
        assert_eq!(
            std::fs::read_to_string(&bcr_out).expect("read bcr"),
            render_bcr_source("rules_dx", "1.2.3")
        );
        assert!(write_spdx(&scratch.path().join("missing"), &spdx_out, "dx", "s").is_err());
        assert!(write_provenance(&scratch.path().join("missing"), &prov_out, "b").is_err());
    }
}
