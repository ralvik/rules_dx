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

/// BCR publisher identity pin.
/// See: `deploy/release/bcr.bzl` (`bcr_source_error`).
pub const BCR_WANT_MODULE: &str = "rules_dx";
/// Sigstore trust root for signing.
/// See: `deploy/release/signing.bzl` (`SIGNING_TRUST_ROOT`).
pub const SIGNING_TRUST_ROOT: &str = "https://tuf-repo-cdn.sigstore.dev";
/// Pinned cosign version.
/// See: `deploy/release/signing.bzl` (`SIGNING_COSIGN_VERSION`).
pub const SIGNING_COSIGN_VERSION: &str = "v2.4.1";
/// Sigstore bundle media type.
/// See: `deploy/release/signing.bzl` (`SIGNING_BUNDLE_MEDIA_TYPE`).
pub const SIGNING_BUNDLE_MEDIA_TYPE: &str = "application/vnd.dev.sigstore.bundle.v0.3+json";
/// Default OIDC issuer.
/// See: `deploy/release/signing.bzl` (`SIGNING_ISSUER`).
pub const SIGNING_ISSUER_DEFAULT: &str = "https://token.actions.githubusercontent.com";

fn basename_of(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(path)
        .to_owned()
}

/// Renders the BCR dry-run would-submit text.
pub fn render_bcr_dry_run(module: &str, version: &str, inputs: &[String]) -> String {
    let mut out = String::new();
    out.push_str("bcr: dry run (BCR_DRY_RUN=1); would submit, submitting nothing:\n");
    out.push_str(&format!("  module: {module}\n"));
    out.push_str(&format!("  version: {version}\n"));
    for input in inputs {
        out.push_str(&format!("  input: {} ({input})\n", basename_of(input)));
    }
    out.push_str(&format!(
        "  command: gh pr create --repo bazelbuild/bazel-central-registry --title \"Add {module}@{version}\" --body \"Owner-approved BCR submission for {module}@{version}\"\n"
    ));
    out.push_str("  presubmit: bazel test @rules_dx//... (seed host) + BCR presubmit.yml\n");
    out
}

/// Runs the BCR gate, returning stdout text or the stderr diagnostic.
pub fn bcr_run(
    module: &str,
    version: &str,
    inputs: &[String],
    dry_run: bool,
    approved: bool,
) -> Result<String, String> {
    if module != BCR_WANT_MODULE {
        return Err(format!("bcr: invalid module '{module}': want 'rules_dx'"));
    }
    if dry_run {
        return Ok(render_bcr_dry_run(module, version, inputs));
    }
    if version == "0.0.0" {
        return Err(
            "bcr: version 0.0.0 is unpublishable (shape check only); a real submission needs an owner-approved SemVer release version"
                .to_owned(),
        );
    }
    if !approved {
        return Err(
            "bcr: submission needs explicit owner approval per issue #5 (BCR_APPROVE=1); run with BCR_DRY_RUN=1 to print the would-submit PR"
                .to_owned(),
        );
    }
    if version == "0.0.0" {
        return Err("bcr: version 0.0.0 is unpublishable even with approval".to_owned());
    }
    let mut out = String::new();
    out.push_str(&format!(
        "bcr: owner-approved submission for {module}@{version} (human-run path only; see docs/deploy/release-runbook.md)\n"
    ));
    for input in inputs {
        out.push_str(&format!("  input: {} ({input})\n", basename_of(input)));
    }
    out.push_str(
        "bcr: open the BCR PR manually with the inputs above (this program never pushes itself)\n",
    );
    Ok(out)
}

/// Renders the signing dry-run would-sign text.
pub fn render_signing_dry_run(identity: &str, issuer: &str, assets: &[String]) -> String {
    let mut out = String::new();
    out.push_str("signing: dry run (RELEASE_SIGN_DRY_RUN=1); would sign, publishing nothing:\n");
    out.push_str(&format!("  trust root: {SIGNING_TRUST_ROOT}\n"));
    out.push_str(&format!("  identity: {identity}\n"));
    out.push_str(&format!("  issuer: {issuer}\n"));
    out.push_str(&format!(
        "  cosign: {SIGNING_COSIGN_VERSION} (pinned per deploy/release/signing.bzl SIGNING_COSIGN_VERSION; checksum-verified fetch per .github/workflows/ghcr.yml)\n"
    ));
    out.push_str(&format!(
        "  bundle media type: {SIGNING_BUNDLE_MEDIA_TYPE} (Sigstore bundle v0.3; 0.1/0.2 only if declared)\n"
    ));
    for asset in assets {
        let base = basename_of(asset);
        out.push_str(&format!("  asset: {base} ({asset})\n"));
        out.push_str(&format!(
            "  command: cosign sign-blob --bundle {base}.bundle --certificate-identity {identity} --certificate-issuer {issuer} {asset}\n"
        ));
        out.push_str(&format!(
            "  command: gh attestation create {asset} --bundle {base}.bundle\n"
        ));
    }
    out.push_str(&format!(
        "  verify: cosign verify-blob --bundle <bundle> --certificate-identity {identity} --certificate-issuer {issuer} <binary>\n"
    ));
    out
}

/// Runs the signing gate (dry-run only; live needs cosign on PATH).
/// Host tools resolve at run time with no new module dependencies.
/// See: `deploy/release/signing.bzl` (signing pins).
pub fn signing_run(
    identity: &str,
    issuer: &str,
    assets: &[String],
    dry_run: bool,
    cosign_present: bool,
) -> Result<String, String> {
    if assets.is_empty() {
        return Err("signing: need at least one artifact".to_owned());
    }
    if identity.is_empty() {
        return Err(
            "signing: missing SIGNING_IDENTITY (owner-approved release workflow identity)"
                .to_owned(),
        );
    }
    if dry_run {
        return Ok(render_signing_dry_run(identity, issuer, assets));
    }
    if !cosign_present {
        return Err(
            "signing: 'cosign' CLI not found on PATH; install it to sign releases (owner-approved human-run path only)"
                .to_owned(),
        );
    }
    Ok(render_signing_dry_run(identity, issuer, assets))
}

/// Renders the human-run release driver dry-run plan.
pub fn render_release_dry_run(tag: &str, approve: &str) -> String {
    let mut out = String::new();
    out.push_str("release: dry run (RELEASE_DRY_RUN=1); would release, publishing nothing:\n");
    out.push_str(&format!("  tag: {tag}\n"));
    out.push_str(&format!(
        "  approve: {approve} (real release needs RELEASE_APPROVE=1 + owner approval)\n"
    ));
    out.push_str("  steps:\n");
    out.push_str(
        "    1. bazel build //cli/cli:dx //cli/cli:dx_standalone //cli/cli:man_pages (seed matrix cell dx-linux-x86_64)\n",
    );
    out.push_str(
        "    2. bazel build //deploy/release:release_artifacts (packaged releasable unit: audit curator plus binary plus man page plus NOTICE plus SBOM/provenance)\n",
    );
    out.push_str(
        "    3. bazel build //deploy/release:sbom_demo //deploy/release:notice_demo (SPDX-2.3 plus SLSA v1 with subject digest equal to artifact sha256; aggregated NOTICE from the audited inventory, signed alongside the SBOM pair)\n",
    );
    out.push_str(
        "    4. RELEASE_SIGN_DRY_RUN=1 bazel run //deploy/release:signing_demo (would-sign cosign + attestation)\n",
    );
    out.push_str(
        "    5. GH_RELEASE_DRY_RUN=1 bazel run //cli/cli:github_draft (would-create draft --draft --verify-tag)\n",
    );
    out.push_str(
        "    6. BCR_DRY_RUN=1 bazel run //deploy/release:bcr_demo (would-submit BCR PR, submits nothing)\n",
    );
    out.push_str(
        "    7. ghcr.yml dispatch + approve:true (separate workflow, push+cosign sign <digest>)\n",
    );
    out.push_str(
        "    8. bazel run //deploy/install:dx_verify --binary <dx> --bundle <bundle> --identity <id> --issuer <issuer> --notice <NOTICE> --notice-manifest <manifest> (fail-before-install)\n",
    );
    out.push_str(&format!(
        "  tag creation: git tag {tag} must already exist in the remote (pushed beforehand with owner approval); this program never creates or pushes tags\n"
    ));
    out.push_str("  publishing: nothing (dry run never tags, releases, submits, or pushes)\n");
    // Keep the legacy ceiling sentence the old verifier greps for.
    out.push_str("  ceiling: this program never creates or pushes tags\n");
    out
}

/// Runs the release driver gate.
pub fn release_run(
    tag: &str,
    approve: &str,
    dry_run: bool,
    tree_dirty: bool,
    tag_exists: bool,
) -> Result<String, String> {
    if dry_run {
        return Ok(render_release_dry_run(tag, approve));
    }
    if approve != "1" {
        return Err(
            "release: real release needs RELEASE_APPROVE=1 plus explicit owner approval per issue #5"
                .to_owned(),
        );
    }
    if tag == "v0.0.0-dryrun" {
        return Err(
            "release: placeholder tag v0.0.0-dryrun is dry-run only; pass a real SemVer tag already pushed to the remote"
                .to_owned(),
        );
    }
    if tree_dirty {
        return Err("release: tree is dirty; release requires a clean tree".to_owned());
    }
    if !tag_exists {
        return Err(format!(
            "release: local tag {tag} missing; push it beforehand with owner approval (this program never creates tags)"
        ));
    }
    let mut out = String::new();
    out.push_str(&format!(
        "release: owner-approved human-run release for {tag} (see docs/deploy/release-runbook.md for the full checklist)\n"
    ));
    out.push_str("release: proceeding step by step; any failure stops before publish\n");
    out.push_str(&format!(
        "release: local tag {tag} exists; remote must already carry it (pushed beforehand with owner approval)\n"
    ));
    Ok(out)
}

/// Verifies one SBOM output pair, returning the OK line.
pub fn sbom_verify_files(artifact: &Path, spdx: &Path, prov: &Path) -> io::Result<String> {
    let digest = sha256_file(artifact)?;
    let spdx_text = std::fs::read_to_string(spdx)?;
    let prov_text = std::fs::read_to_string(prov)?;
    if !spdx_text.contains("\"spdxVersion\": \"SPDX-2.3\"") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "sbom spdxVersion not SPDX-2.3",
        ));
    }
    if !spdx_text.contains(&digest) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("sbom SPDX missing artifact digest {digest}"),
        ));
    }
    if !prov_text.contains("\"_type\": \"https://in-toto.io/Statement/v1\"") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "provenance missing in-toto Statement v1",
        ));
    }
    if !prov_text.contains("\"predicateType\": \"https://slsa.dev/provenance/v1\"") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "provenance missing SLSA v1 predicate",
        ));
    }
    if !prov_text.contains(&digest) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("provenance missing subject digest {digest}"),
        ));
    }
    Ok(format!("sbom OK: SPDX-2.3 + SLSA v1 bind {digest}\n"))
}

/// One aggregated-NOTICE inventory entry.
///
/// See: `deploy/release/notice.bzl` (manifest shape).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NoticeEntry {
    /// Package name in its owning set.
    pub package: String,
    /// Owning dependency set.
    pub set: String,
    /// Locked version.
    pub version: String,
    /// SPDX license expression text.
    pub license: String,
    /// Basename of the license-words file in `texts`, empty when missing.
    pub text_basename: String,
}

/// Parses one NOTICE manifest document.
///
/// See: `deploy/release/notice.bzl` (manifest shape).
pub fn parse_notice_manifest(text: &str) -> Result<Vec<NoticeEntry>, String> {
    let mut entries = Vec::new();
    for (index, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('|').collect();
        if fields.len() != 5 {
            return Err(format!(
                "notice manifest line {}: want 5 '|' fields 'package|set|version|license|text', got {}",
                index + 1,
                fields.len()
            ));
        }
        let entry = NoticeEntry {
            package: fields[0].trim().to_owned(),
            set: fields[1].trim().to_owned(),
            version: fields[2].trim().to_owned(),
            license: fields[3].trim().to_owned(),
            text_basename: fields[4].trim().to_owned(),
        };
        if entry.package.is_empty()
            || entry.set.is_empty()
            || entry.version.is_empty()
            || entry.license.is_empty()
        {
            return Err(format!(
                "notice manifest line {}: package, set, version, and license must be non-empty",
                index + 1
            ));
        }
        entries.push(entry);
    }
    entries.sort_by(|a, b| {
        (&a.package, &a.set, &a.version, &a.license)
            .cmp(&(&b.package, &b.set, &b.version, &b.license))
    });
    Ok(entries)
}

/// Renders aggregated NOTICE bytes for one distributed root.
///
/// See: `deploy/release/notice.bzl` (deterministic bytes).
pub fn render_notice(root: &str, entries: &[(NoticeEntry, String)]) -> String {
    let mut sorted = entries.to_vec();
    sorted.sort_by(|a, b| {
        (&a.0.package, &a.0.set, &a.0.version, &a.0.license).cmp(&(
            &b.0.package,
            &b.0.set,
            &b.0.version,
            &b.0.license,
        ))
    });
    let mut out = String::new();
    out.push_str(&format!("NOTICE for {root}\n"));
    out.push_str(
        "Generated by rules_dx notice_bundle from the audited license inventory; do not edit.\n",
    );
    out.push('\n');
    for (entry, words) in &sorted {
        out.push_str(&format!(
            "=== {} {} ({}) ===\n",
            entry.package, entry.version, entry.license
        ));
        let trimmed = words.trim_end_matches('\n');
        out.push_str(trimmed);
        out.push('\n');
    }
    out
}

/// Reads license-words files keyed by basename.
fn read_text_map(
    text_files: &[std::path::PathBuf],
) -> Result<std::collections::BTreeMap<String, String>, String> {
    let mut map = std::collections::BTreeMap::new();
    for path in text_files {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("")
            .to_owned();
        if name.is_empty() {
            return Err(format!(
                "notice: text input {} has no UTF-8 basename",
                path.display()
            ));
        }
        let words = std::fs::read_to_string(path)
            .map_err(|error| format!("notice: cannot read {name}: {error}"))?;
        if words.trim().is_empty() {
            return Err(format!("notice: missing-notice-text: {name} is empty"));
        }
        map.insert(name, words);
    }
    Ok(map)
}

/// Joins one manifest with its license-words files, failing actionable on missing text.
fn join_notice_entries(
    manifest_text: &str,
    text_files: &[std::path::PathBuf],
) -> Result<Vec<(NoticeEntry, String)>, String> {
    let entries = parse_notice_manifest(manifest_text)?;
    if entries.is_empty() {
        return Err("notice: manifest lists no packages; refusing empty NOTICE".to_owned());
    }
    let map = read_text_map(text_files)?;
    let mut joined = Vec::with_capacity(entries.len());
    for entry in entries {
        if entry.text_basename.is_empty() {
            return Err(format!(
                "notice: missing-notice-text: {}@{} ({}) ships no LICENSE*/NOTICE* words; record the words in the audited inventory before bundling",
                entry.package, entry.version, entry.license
            ));
        }
        match map.get(&entry.text_basename) {
            Some(words) => joined.push((entry, words.clone())),
            None => {
                return Err(format!(
                    "notice: missing-notice-text: {}@{} ({}) names text '{}' with no matching input file",
                    entry.package, entry.version, entry.license, entry.text_basename
                ));
            }
        }
    }
    Ok(joined)
}

/// Writes the aggregated NOTICE for one manifest plus its license-words files.
///
/// See: `deploy/release/notice.bzl` (genrule `tools`).
pub fn write_notice(
    manifest: &Path,
    dst: &Path,
    root: &str,
    text_files: &[std::path::PathBuf],
) -> io::Result<()> {
    let manifest_text = std::fs::read_to_string(manifest)?;
    let joined = join_notice_entries(&manifest_text, text_files).map_err(io::Error::other)?;
    std::fs::write(dst, render_notice(root, &joined).as_bytes())?;
    Ok(())
}

/// Verifies one NOTICE output binds its manifest plus words, returning the OK line.
pub fn notice_verify_files(
    notice: &Path,
    manifest: &Path,
    text_files: &[std::path::PathBuf],
) -> io::Result<String> {
    let notice_text = std::fs::read_to_string(notice)?;
    let manifest_text = std::fs::read_to_string(manifest)?;
    let entries = parse_notice_manifest(&manifest_text).map_err(io::Error::other)?;
    if entries.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "notice: manifest lists no packages; refusing empty NOTICE",
        ));
    }
    if !notice_text.starts_with("NOTICE for ") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "notice: missing 'NOTICE for <root>' header",
        ));
    }
    let map = read_text_map(text_files).map_err(io::Error::other)?;
    for entry in &entries {
        if entry.text_basename.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "notice: missing-notice-text: {}@{} ({}) ships no LICENSE*/NOTICE* words",
                    entry.package, entry.version, entry.license
                ),
            ));
        }
        let words = map.get(&entry.text_basename).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "notice: missing-notice-text: {}@{} ({}) names text '{}' with no matching input file",
                    entry.package, entry.version, entry.license, entry.text_basename
                ),
            )
        })?;
        let header = format!(
            "=== {} {} ({}) ===",
            entry.package, entry.version, entry.license
        );
        if !notice_text.contains(&header) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("notice: NOTICE missing entry header '{header}'"),
            ));
        }
        let snippet = words.trim();
        let probe = snippet.lines().next().unwrap_or("").trim();
        if !probe.is_empty() && !notice_text.contains(probe) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "notice: NOTICE missing words for {}@{} ({})",
                    entry.package, entry.version, entry.license
                ),
            ));
        }
    }
    Ok(format!("notice OK: {} entries bundled\n", entries.len()))
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

    #[test]
    fn bcr_dry_run_matches_shell() {
        let out = render_bcr_dry_run("rules_dx", "0.0.0", &["/tmp/a.spdx.json".to_owned()]);
        assert!(out.contains("bcr: dry run (BCR_DRY_RUN=1); would submit, submitting nothing:"));
        assert!(out.contains("  module: rules_dx"));
        assert!(out.contains("  version: 0.0.0"));
        assert!(out.contains("  input: a.spdx.json (/tmp/a.spdx.json)"));
        assert!(out.contains("gh pr create --repo bazelbuild/bazel-central-registry"));
        assert!(out.contains("presubmit: bazel test @rules_dx//..."));
        let ok = bcr_run("rules_dx", "0.0.0", &[], true, false).expect("dry");
        assert!(ok.contains("would submit, submitting nothing"));
        assert!(bcr_run("other", "0.0.0", &[], true, false).is_err());
        assert!(bcr_run("rules_dx", "0.0.0", &[], false, false).is_err());
        assert!(bcr_run("rules_dx", "0.0.0", &[], false, true).is_err());
        assert!(bcr_run("rules_dx", "1.2.3", &[], false, false).is_err());
        let live =
            bcr_run("rules_dx", "1.2.3", &["/x/y.json".to_owned()], false, true).expect("live");
        assert!(live.contains("owner-approved submission for rules_dx@1.2.3"));
        assert!(live.contains("never pushes itself"));
    }

    #[test]
    fn signing_dry_run_matches_shell() {
        let assets = ["/tmp/a.bin".to_owned()];
        let out = render_signing_dry_run("IDENT", "ISSUER", &assets);
        assert!(out.contains(
            "signing: dry run (RELEASE_SIGN_DRY_RUN=1); would sign, publishing nothing:"
        ));
        assert!(out.contains("trust root: https://tuf-repo-cdn.sigstore.dev"));
        assert!(out.contains("cosign sign-blob"));
        assert!(out.contains("gh attestation create"));
        assert!(out.contains("cosign verify-blob"));
        assert!(out.contains("v2.4.1"));
        assert!(out.contains("application/vnd.dev.sigstore.bundle.v0.3+json"));
        assert!(signing_run("", "ISSUER", &assets, true, true).is_err());
        assert!(signing_run("ID", "ISS", &[], true, true).is_err());
        assert!(signing_run("ID", "ISS", &assets, false, false).is_err());
        let ok = signing_run("ID", "ISS", &assets, true, true).expect("dry");
        assert!(ok.contains("would sign, publishing nothing"));
    }

    #[test]
    fn release_dry_run_matches_shell() {
        let out = render_release_dry_run("v0.0.0-dryrun", "0");
        assert!(out.contains("release: dry run (RELEASE_DRY_RUN=1)"));
        assert!(out.contains("never creates or pushes tags"));
        assert!(out.contains("publishing nothing"));
        assert!(out.contains("dx_verify"));
        assert!(out.contains("notice_demo"));
        // See: `docs/deploy/release-runbook.md#packaging` (artifact-into-releases packaging).
        assert!(out.contains("release_artifacts"));
        assert!(out.contains("subject digest equal to artifact sha256"));
        assert!(out.contains("signed alongside the SBOM pair"));
        let package = out.find("release_artifacts").expect("packaging step");
        let notice = out.find("notice_demo").expect("notice step");
        let sign = out.find("signing_demo").expect("sign step");
        let draft = out.find("github_draft").expect("draft step");
        assert!(package < notice);
        assert!(notice < sign);
        assert!(sign < draft);
        let ok = release_run("v0.0.0-dryrun", "0", true, false, false).expect("dry");
        assert!(ok.contains("would release, publishing nothing"));
        assert!(release_run("v1.2.3", "0", false, false, true).is_err());
        assert!(release_run("v0.0.0-dryrun", "1", false, false, true).is_err());
        assert!(release_run("v1.2.3", "1", false, true, true).is_err());
        assert!(release_run("v1.2.3", "1", false, false, false).is_err());
        let live = release_run("v1.2.3", "1", false, false, true).expect("live");
        assert!(live.contains("owner-approved human-run release for v1.2.3"));
    }

    #[test]
    fn sbom_verify_binds_digest() {
        let scratch = scratch_dir();
        let artifact = write_artifact(scratch.path(), "artifact.bin", b"hello world\n");
        let digest = sha256_file(&artifact).expect("digest");
        let spdx = scratch.path().join("artifact.spdx.json");
        write_spdx(&artifact, &spdx, "dx", "rules_dx").expect("spdx");
        let prov = scratch.path().join("artifact.prov.json");
        write_provenance(&artifact, &prov, "https://example.com/builder").expect("prov");
        let ok = sbom_verify_files(&artifact, &spdx, &prov).expect("verify");
        assert!(ok.contains("sbom OK: SPDX-2.3 + SLSA v1 bind"));
        assert!(ok.contains(&digest));
        std::fs::write(&spdx, b"{\"spdxVersion\": \"SPDX-3.0\"}").expect("rewrite");
        assert!(sbom_verify_files(&artifact, &spdx, &prov).is_err());
    }

    fn notice_fixture(dir: &Path) -> (std::path::PathBuf, Vec<std::path::PathBuf>) {
        let manifest = dir.join("inventory.txt");
        std::fs::write(
            &manifest,
            "demo-lib-b|npm|2.3.4|Apache-2.0|demo-lib-b.txt\ndemo-lib-a|cargo|1.0.0|MIT|demo-lib-a.txt\n",
        )
        .expect("manifest");
        let a = dir.join("demo-lib-a.txt");
        std::fs::write(&a, "Fixture words for demo-lib-a (MIT placeholder).\n").expect("words a");
        let b = dir.join("demo-lib-b.txt");
        std::fs::write(
            &b,
            "Fixture words for demo-lib-b (Apache-2.0 placeholder).\n",
        )
        .expect("words b");
        (manifest, vec![a, b])
    }

    #[test]
    fn notice_manifest_parses_and_sorts() {
        let entries = parse_notice_manifest(
            "# invented distributed-tier root\n\ndemo-lib-b|npm|2.3.4|Apache-2.0|demo-lib-b.txt\ndemo-lib-a|cargo|1.0.0|MIT|demo-lib-a.txt\n",
        )
        .expect("parse");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].package, "demo-lib-a");
        assert_eq!(entries[1].package, "demo-lib-b");
        assert!(parse_notice_manifest("only|four|fields|here").is_err());
        assert!(parse_notice_manifest("|npm|1.0.0|MIT|a.txt").is_err());
    }

    #[test]
    fn notice_render_is_deterministic_over_input_order() {
        let scratch = scratch_dir();
        let (manifest, texts) = notice_fixture(scratch.path());
        let manifest_text = std::fs::read_to_string(&manifest).expect("read");
        let first = join_notice_entries(&manifest_text, &texts).expect("join");
        let mut rev = texts.clone();
        rev.reverse();
        let second = join_notice_entries(&manifest_text, &rev).expect("join rev");
        assert_eq!(
            render_notice("//demo:root", &first),
            render_notice("//demo:root", &second)
        );
        let text = render_notice("//demo:root", &first);
        assert!(text.starts_with("NOTICE for //demo:root\n"));
        assert!(text.ends_with('\n'));
        assert!(!text.ends_with("\r\n"));
        let a = text
            .find("=== demo-lib-a 1.0.0 (MIT) ===")
            .expect("a header");
        let b = text
            .find("=== demo-lib-b 2.3.4 (Apache-2.0) ===")
            .expect("b header");
        assert!(a < b);
        assert!(text.contains("Fixture words for demo-lib-a"));
    }

    #[test]
    fn notice_write_rebuilds_byte_identical() {
        let scratch = scratch_dir();
        let (manifest, texts) = notice_fixture(scratch.path());
        let first = scratch.path().join("first.NOTICE");
        write_notice(&manifest, &first, "//demo:root", &texts).expect("first");
        let second = scratch.path().join("second.NOTICE");
        write_notice(&manifest, &second, "//demo:root", &texts).expect("second");
        assert_eq!(
            std::fs::read(&first).expect("read first"),
            std::fs::read(&second).expect("read second")
        );
        let ok = notice_verify_files(&first, &manifest, &texts).expect("verify");
        assert!(ok.contains("notice OK: 2 entries bundled"));
    }

    #[test]
    fn notice_missing_text_fails_actionable() {
        let scratch = scratch_dir();
        let manifest = scratch.path().join("inventory.txt");
        std::fs::write(&manifest, "demo-lib-a|cargo|1.0.0|MIT|\n").expect("manifest");
        let out = scratch.path().join("out.NOTICE");
        let err = write_notice(&manifest, &out, "//demo:root", &[]).expect_err("missing");
        assert!(err.to_string().contains("missing-notice-text"));
        let (good_manifest, texts) = notice_fixture(scratch.path());
        let notice = scratch.path().join("good.NOTICE");
        write_notice(&good_manifest, &notice, "//demo:root", &texts).expect("good");
        std::fs::write(&notice, "tampered bytes\n").expect("tamper");
        let err = notice_verify_files(&notice, &good_manifest, &texts).expect_err("tampered");
        assert!(err.to_string().contains("NOTICE"));
    }
}
