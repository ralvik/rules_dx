//! Deterministic deploy archive tools for `archive_deploy`.
//!
//! Owning contract: `docs/deploy/authoring.md` (Path C).
//!
//! Why hand-rolled tar: Python `tarfile` PAX for short basenames emits plain
//! USTAR (`mtime 0`, `uid/gid 0`, `RECORDSIZE` 10240 padding); the `tar`
//! crate defaults (`typeflag` NUL, 7-digit checksum) differ, so bytes are
//! built explicitly to stay identical.
//! See: `deploy/rules/archive.bzl` (genrule `tools`).
//! Why C zlib: Python `gzip` level 9 is C zlib deflate; pure-Rust backends
//! emit different bytes for identical input, while `flate2` with the `zlib`
//! feature matches exactly.
//! See: `cli/digest/src/lib.rs` (SHA-256 shim).

// Infallible paths must not `expect`/`unwrap` outside tests
// (`cfg_attr(not(test))` keeps `rust_test` bodies ergonomic).
#![cfg_attr(not(test), deny(clippy::expect_used, clippy::unwrap_used))]

use std::io::{self, Write};
use std::path::Path;

/// Tar block size.
const BLOCK: usize = 512;
/// Python `tarfile.RECORDSIZE`: archives pad to multiples of 20 blocks.
const RECORDSIZE: usize = 20 * BLOCK;
/// Fixed metadata matching `archiver.py`.
const MTIME: u64 = 0;
const UID: u64 = 0;
const GID: u64 = 0;

/// Formats `value` as `digits - 1` octal digits plus NUL, matching
/// Python `tarfile.itn` for values fitting the field.
fn octal_field(value: u64, digits: usize) -> io::Result<Vec<u8>> {
    let max = 8u64.pow((digits - 1) as u32);
    if value >= max {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("value {value} overflows {digits}-byte octal field"),
        ));
    }
    let text = format!("{:0>width$o}", value, width = digits - 1);
    let mut out = text.into_bytes();
    out.push(0);
    Ok(out)
}

/// Builds one 512-byte USTAR header with Python `tarfile` PAX short-name
/// bytes (`mtime 0`, `uid/gid 0`, empty `uname/gname`, NUL device fields,
/// `ustar\\0` + `00` magic, `typeflag` `0`, checksum `"%06o\\0 "`).
fn tar_header(name: &str, size: u64, mode: u32) -> io::Result<[u8; BLOCK]> {
    let name_bytes = name.as_bytes();
    if name_bytes.is_empty() || name_bytes.len() > 100 || name_bytes.contains(&0) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("tar member name {name:?} must be 1-100 bytes with no NUL"),
        ));
    }
    if name_bytes.contains(&b'/') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("tar member name {name:?} must be a basename with no slash"),
        ));
    }
    let mut header = [0u8; BLOCK];
    header[0..name_bytes.len()].copy_from_slice(name_bytes);
    let mode_field = octal_field(u64::from(mode & 0o7777), 8)?;
    header[100..108].copy_from_slice(&mode_field);
    let uid_field = octal_field(UID, 8)?;
    header[108..116].copy_from_slice(&uid_field);
    let gid_field = octal_field(GID, 8)?;
    header[116..124].copy_from_slice(&gid_field);
    let size_field = octal_field(size, 12)?;
    header[124..136].copy_from_slice(&size_field);
    let mtime_field = octal_field(MTIME, 12)?;
    header[136..148].copy_from_slice(&mtime_field);
    // Checksum placeholder is eight spaces for the summation below.
    // See: Python `tarfile._create_header` (`b"        "`).
    for slot in header.iter_mut().take(156).skip(148) {
        *slot = b' ';
    }
    header[156] = b'0';
    // Linkname 100 NULs already zeroed; magic covers `ustar\\0` + `00`.
    header[257..265].copy_from_slice(b"ustar\x0000");
    // Uname/gname/devmajor/devminor/prefix/pad stay NUL for this tool.
    let mut sum: u32 = 0;
    for byte in header {
        sum += u32::from(byte);
    }
    let checksum_text = format!("{sum:06o}\0 ");
    let checksum_bytes = checksum_text.as_bytes();
    header[148..156].copy_from_slice(checksum_bytes);
    Ok(header)
}

/// Builds the deterministic tar image: one header plus file bytes plus
/// two zero blocks, padded to `RECORDSIZE` multiples like Python
/// `tarfile.close` (`-b20`).
fn tar_image(name: &str, data: &[u8], mode: u32) -> io::Result<Vec<u8>> {
    let header = tar_header(name, data.len() as u64, mode)?;
    let data_blocks = data.len().div_ceil(BLOCK) * BLOCK;
    let used = BLOCK + data_blocks + 2 * BLOCK;
    let padded = used.div_ceil(RECORDSIZE) * RECORDSIZE;
    let mut out = Vec::with_capacity(padded);
    out.extend_from_slice(&header);
    out.extend_from_slice(data);
    out.resize(out.len() + (data_blocks - data.len()), 0);
    out.resize(out.len() + 2 * BLOCK, 0);
    out.resize(padded, 0);
    Ok(out)
}

/// Compresses `tar` with gzip level 9, `mtime 0`, no filename, `OS` 255,
/// matching Python `gzip.GzipFile(filename="", compresslevel=9, mtime=0)`.
fn gzip_compress(tar: &[u8]) -> io::Result<Vec<u8>> {
    let mut encoder = flate2::GzBuilder::new()
        .mtime(0)
        .operating_system(255)
        .write(Vec::new(), flate2::Compression::best());
    encoder.write_all(tar)?;
    encoder.finish()
}

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

/// File mode for the tar member: `0755` when any executable bit is set,
/// else `0644`, matching `archiver.py` (`st_mode & 0o111`).
fn member_mode(path: &Path) -> io::Result<u32> {
    let metadata = std::fs::metadata(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let executable = metadata.permissions().mode() & 0o111 != 0;
        Ok(if executable { 0o755 } else { 0o644 })
    }
    #[cfg(not(unix))]
    {
        let _ = metadata;
        Ok(0o644)
    }
}

/// Archives one staged file into a deterministic tar.gz, dereferencing
/// symlinks like `tar -h` (both read and stat follow links).
pub fn archive_file(src: &Path, dst: &Path) -> io::Result<()> {
    let data = std::fs::read(src)?;
    let name = basename(src)?;
    let mode = member_mode(src)?;
    let tar = tar_image(&name, &data, mode)?;
    let gz = gzip_compress(&tar)?;
    std::fs::write(dst, gz)?;
    Ok(())
}

/// Builds the deterministic tar.gz bytes in memory (parity-test entry).
pub fn archive_bytes(data: &[u8], name: &str, executable: bool) -> io::Result<Vec<u8>> {
    let mode = if executable { 0o755 } else { 0o644 };
    let tar = tar_image(name, data, mode)?;
    gzip_compress(&tar)
}

/// SHA-256 hex of `data`, matching `hashlib.sha256().hexdigest`.
fn sha256_hex(data: &[u8]) -> String {
    use sha2::Digest as _;
    let mut hasher = sha2::Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// Writes a `sha256sum`-compatible `<digest>  <basename>\\n` line.
pub fn hash_file(src: &Path, dst: &Path) -> io::Result<()> {
    let base = basename(src)?;
    let mut hasher = sha2::Sha256::new();
    use sha2::Digest as _;
    let mut file = std::fs::File::open(src)?;
    let mut buf = [0u8; 1 << 20];
    loop {
        use std::io::Read as _;
        let read = file.read(&mut buf)?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
    }
    let line = format!("{}  {base}\n", hex::encode(hasher.finalize()));
    std::fs::write(dst, line.as_bytes())?;
    Ok(())
}

/// Formats the checksum line in memory (parity-test entry).
pub fn hash_line(data: &[u8], basename: &str) -> String {
    format!("{}  {basename}\n", sha256_hex(data))
}

/// Shared thin-binary helpers (issue #914): every `bin_*` shim reports
/// usage to stderr and returns 1 through these, so `eprintln!` + exit
/// codes cannot drift between shims. See: `docs/deploy/authoring.md`.
pub fn bin_usage(prog: &str, usage: &str) -> i32 {
    eprintln!("usage: {prog} {usage}");
    1
}

/// Reports `<prog>: cannot <action> <target>: <error>` to stderr for a
/// failed thin-binary operation. Returns the process exit code (1).
pub fn bin_cannot(prog: &str, action: &str, target: &Path, error: impl std::fmt::Display) -> i32 {
    eprintln!("{prog}: cannot {action} {}: {error}", target.display());
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_dir() -> tempfile::TempDir {
        tempfile::TempDir::new().expect("scratch")
    }

    #[test]
    fn octal_fields_match_python_tarfile() {
        assert_eq!(octal_field(0o755, 8).expect("mode"), b"0000755\0");
        assert_eq!(octal_field(0, 8).expect("uid"), b"0000000\0");
        assert_eq!(octal_field(12, 12).expect("size"), b"00000000014\0");
        assert_eq!(octal_field(0, 12).expect("mtime"), b"00000000000\0");
        assert!(octal_field(8u64.pow(7), 8).is_err());
        assert!(octal_field(8u64.pow(11), 12).is_err());
    }

    #[test]
    fn header_bytes_match_python_ustar() {
        let header = tar_header("hello", 12, 0o755).expect("header");
        assert_eq!(&header[0..6], b"hello\0");
        assert!(header[6..100].iter().all(|byte| *byte == 0));
        assert_eq!(&header[100..108], b"0000755\0");
        assert_eq!(&header[108..116], b"0000000\0");
        assert_eq!(&header[116..124], b"0000000\0");
        assert_eq!(&header[124..136], b"00000000014\0");
        assert_eq!(&header[136..148], b"00000000000\0");
        assert_eq!(&header[148..156], b"006771\0 ");
        assert_eq!(header[156], b'0');
        assert!(header[157..257].iter().all(|byte| *byte == 0));
        assert_eq!(&header[257..265], b"ustar\x0000");
        assert!(header[265..512].iter().all(|byte| *byte == 0));
        // Non-executable mode changes the checksum accordingly.
        let plain = tar_header("hello", 12, 0o644).expect("plain header");
        assert_eq!(&plain[100..108], b"0000644\0");
        assert_ne!(&plain[148..156], &header[148..156]);
    }

    #[test]
    fn header_rejects_bad_names() {
        assert!(tar_header("", 0, 0o644).is_err());
        assert!(tar_header(&"n".repeat(101), 0, 0o644).is_err());
        assert!(tar_header("a/b", 0, 0o644).is_err());
        assert!(tar_header("a\0b", 0, 0o644).is_err());
    }

    #[test]
    fn tar_image_pads_to_recordsize() {
        let tiny = tar_image("f", b"hi", 0o644).expect("tiny");
        assert_eq!(tiny.len(), RECORDSIZE);
        assert_eq!(&tiny[0..1], b"f");
        assert_eq!(&tiny[512..514], b"hi");
        assert!(tiny[514..].iter().all(|byte| *byte == 0));
        // 10240 payload bytes need two records (header + 20 data + 2 end).
        let big_data = vec![b'x'; 10240];
        let big = tar_image("f", &big_data, 0o644).expect("big");
        assert_eq!(big.len(), 2 * RECORDSIZE);
    }

    #[test]
    fn gzip_header_matches_python_gzip() {
        let tar = tar_image("hello", b"hello world\n", 0o755).expect("tar");
        let gz = gzip_compress(&tar).expect("gz");
        assert_eq!(
            &gz[0..10],
            &[0x1f, 0x8b, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0xff]
        );
        // Round-trips exactly through the standard decoder.
        let mut decoder = flate2::read::GzDecoder::new(&gz[..]);
        let mut decoded = Vec::new();
        use std::io::Read as _;
        decoder.read_to_end(&mut decoded).expect("decode");
        assert_eq!(decoded, tar);
    }

    #[test]
    fn archive_bytes_match_python_golden() {
        // Golden from `archiver.py` over `hello world\\n` with mode 0755:
        // header checksum `006771\\0 `, `RECORDSIZE` 10240 tar, gzip level 9
        // `mtime 0`, `OS` 255. Regenerate with
        // `bazel build //deploy/rules:archiver` output before editing this test.
        let gz = archive_bytes(b"hello world\n", "hello", true).expect("archive");
        assert_eq!(gz.len(), 110);
        assert_eq!(
            &gz[0..10],
            &[0x1f, 0x8b, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0xff]
        );
        let mut decoder = flate2::read::GzDecoder::new(&gz[..]);
        let mut tar = Vec::new();
        use std::io::Read as _;
        decoder.read_to_end(&mut tar).expect("decode");
        assert_eq!(tar.len(), RECORDSIZE);
        assert_eq!(&tar[0..6], b"hello\0");
        assert_eq!(&tar[100..108], b"0000755\0");
        assert_eq!(&tar[148..156], b"006771\0 ");
        assert_eq!(&tar[512..524], b"hello world\n");
    }

    #[test]
    fn archive_file_round_trips_modes_and_symlinks() {
        let scratch = scratch_dir();
        let executable = scratch.path().join("run.sh");
        std::fs::write(&executable, b"#!/bin/sh\necho hi\n").expect("write exe");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o755))
                .expect("chmod");
        }
        let plain = scratch.path().join("data.txt");
        std::fs::write(&plain, b"plain\n").expect("write plain");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            std::fs::set_permissions(&plain, std::fs::Permissions::from_mode(0o644))
                .expect("chmod");
        }
        let exe_out = scratch.path().join("exe.tar.gz");
        archive_file(&executable, &exe_out).expect("archive exe");
        let plain_out = scratch.path().join("plain.tar.gz");
        archive_file(&plain, &plain_out).expect("archive plain");
        for (path, want_mode) in [(&exe_out, "0000755"), (&plain_out, "0000644")] {
            let gz = std::fs::read(path).expect("read gz");
            let mut decoder = flate2::read::GzDecoder::new(&gz[..]);
            let mut tar = Vec::new();
            use std::io::Read as _;
            decoder.read_to_end(&mut tar).expect("decode");
            let mode = String::from_utf8_lossy(&tar[100..107]).into_owned();
            assert_eq!(mode, want_mode);
        }
        // Symlink dereference like `tar -h`: archiving the link archives
        // the target bytes under the link basename.
        #[cfg(unix)]
        {
            let link = scratch.path().join("link.sh");
            std::os::unix::fs::symlink(&executable, &link).expect("symlink");
            let link_out = scratch.path().join("link.tar.gz");
            archive_file(&link, &link_out).expect("archive link");
            let gz = std::fs::read(&link_out).expect("read link gz");
            let mut decoder = flate2::read::GzDecoder::new(&gz[..]);
            let mut tar = Vec::new();
            use std::io::Read as _;
            decoder.read_to_end(&mut tar).expect("decode link");
            assert_eq!(&tar[0..7], b"link.sh");
            assert_eq!(&tar[512..530], b"#!/bin/sh\necho hi\n");
        }
    }

    #[test]
    fn archive_file_rejects_missing_basename() {
        let scratch = scratch_dir();
        let missing = scratch.path().join("does-not-exist");
        let out = scratch.path().join("out.tar.gz");
        assert!(archive_file(&missing, &out).is_err());
        assert!(basename(Path::new("/")).is_err());
    }

    #[test]
    fn sha256_vectors_match_hashlib() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            hash_line(b"hello world\n", "release.tar.gz"),
            format!("{}  release.tar.gz\n", sha256_hex(b"hello world\n"))
        );
    }

    #[test]
    fn hash_file_writes_sha256sum_line() {
        let scratch = scratch_dir();
        let src = scratch.path().join("payload.bin");
        let payload = b"archive payload\n";
        std::fs::write(&src, payload).expect("write");
        let dst = scratch.path().join("payload.sha256");
        hash_file(&src, &dst).expect("hash");
        let text = std::fs::read_to_string(&dst).expect("read line");
        assert_eq!(text, hash_line(payload, "payload.bin"));
        assert!(text.ends_with('\n'));
        assert!(!text.ends_with("\r\n"));
        let parts: Vec<&str> = text.trim_end().split("  ").collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[1], "payload.bin");
        assert_eq!(parts[0].len(), 64);
    }

    #[test]
    fn hash_file_rejects_missing_inputs() {
        let scratch = scratch_dir();
        let missing = scratch.path().join("missing");
        let out = scratch.path().join("out.sha256");
        assert!(hash_file(&missing, &out).is_err());
    }

    #[test]
    fn bin_shims_share_usage_and_failure_exit() {
        assert_eq!(bin_usage("dx_archiver", "<src> <dst>"), 1);
        assert_eq!(
            bin_cannot(
                "dx_archiver",
                "write",
                Path::new("/tmp/out.tar.gz"),
                "read-only filesystem"
            ),
            1
        );
    }
}
