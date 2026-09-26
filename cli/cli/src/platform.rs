//! Startup platform gate for the `dx` CLI.
//!
//! Only hosts with platform evidence stay on the execution path; every
//! other host gets a clean refusal naming the host and the qualification
//! pointer, never partial execution presented as success. The qualified
//! set follows [ADR 0014](../../../docs/decisions/0014-tested-platform-release-stack.md)
//! and the [support matrix](../../../docs/product/support-matrix.md): add a
//! host here exactly when its required-platform evidence lands, so support
//! flips on with the evidence.

/// Hosts with platform evidence: `(std::env::consts::OS, ARCH)` pairs.
///
/// The Linux x86_64 seed host plus Linux arm64 glibc native
/// plus macOS arm64 native plus Windows x86_64 MSVC-compatible native are
/// delivered (See: `docs/product/support-matrix.md`).
///
/// macOS arm64 runs natively on `macos-14` (arm64) runners
/// through the pinned upstream toolchains; the hermetic-llvm Apple-SDK
/// backend stays provisional with immutable lazy fetch and no
/// host-installed SDK fallback (never approved). Exact pins, hosts,
/// floors, and SDK/CRT identities stay owned per ADR 0014.
///
/// macOS x86_64 is Not planned and never planned for support (See: `docs/product/support-matrix.md`, issue #976):
/// no CI, coverage, or artifact footprint is provisioned and `dx` refuses
/// cleanly with `unsupported_platform`. The retired `macos-13` plus
/// `macos-15-intel` runner history stays in docs only for rotation
/// context, never as qualified evidence.
///
/// Windows x86_64 MSVC-compatible runs natively on
/// `windows-latest` runners through the pinned upstream toolchains; the
/// toolchains_msvc clang-cl/Microsoft-STL backend stays provisional with
/// immutable lazy fetch and explicit EULA acceptance (never automatic).
/// Exact pins, hosts, floors, and SDK/CRT identities stay owned
/// per ADR 0014.
pub fn qualified_hosts() -> &'static [(&'static str, &'static str)] {
    &[
        ("linux", "x86_64"),
        ("linux", "aarch64"),
        ("macos", "aarch64"),
        ("windows", "x86_64"),
    ]
}

/// Clean-refusal diagnostic for an unqualified host, or `None` when the
/// host is qualified. The message carries the qualification pointer and
/// the exact missing evidence (no qualified route for this host yet).
pub fn refusal(os: &str, arch: &str) -> Option<String> {
    if qualified_hosts().contains(&(os, arch)) {
        return None;
    }
    Some(format!(
        "unsupported_platform: {os}/{arch} has no qualified platform evidence; dx is delivered on Linux x86_64 and Linux arm64 glibc plus macOS arm64 (issue #412; host-installed SDK fallback never approved) plus Windows x86_64 MSVC-compatible (issue #414; toolchains_msvc clang-cl/Microsoft-STL provisional, explicit EULA acceptance never automatic) only (See: docs/product/support-matrix.md and ADR 0014, tracked in issue #298 with per-host successors such as issues #410/#412/#414; macOS x86_64 is Not planned per issue #976)"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_host_is_qualified() {
        assert_eq!(refusal("linux", "x86_64"), None);
    }

    #[test]
    fn arm64_host_is_qualified() {
        assert_eq!(refusal("linux", "aarch64"), None);
    }

    #[test]
    fn unqualified_hosts_are_refused_with_pointer() {
        // Every remaining ADR 0014 host (out-of-v1) plus Not-planned macOS
        // x86_64 refuses cleanly until its per-host evidence lands: Linux
        // plus macOS arm64 plus Windows x86_64 MSVC-compatible are
        // qualified, so Windows arm64 (out of v1) plus macOS x86_64
        // (Not planned per #976) remain here.
        for (os, arch) in [("windows", "aarch64"), ("macos", "x86_64")] {
            let message = refusal(os, arch).expect("unqualified host must be refused");
            assert!(message.starts_with("unsupported_platform"), "{message}");
            assert!(message.contains(&format!("{os}/{arch}")), "{message}");
            assert!(message.contains("298"), "{message}");
        }
    }

    #[test]
    fn qualified_set_is_seed_plus_arm64_plus_macos_plus_windows() {
        assert_eq!(
            qualified_hosts(),
            &[
                ("linux", "x86_64"),
                ("linux", "aarch64"),
                ("macos", "aarch64"),
                ("windows", "x86_64")
            ]
        );
    }

    #[test]
    fn macos_arm64_host_is_qualified() {
        assert_eq!(refusal("macos", "aarch64"), None);
    }

    #[test]
    fn macos_x86_64_is_refused_not_planned() {
        // Not planned and never planned per issue #976 (See: `docs/product/support-matrix.md`): no CI/coverage
        // footprint, clean refusal with the support-matrix pointer.
        let message = refusal("macos", "x86_64").expect("macOS x86_64 must be refused");
        assert!(message.starts_with("unsupported_platform"), "{message}");
        assert!(message.contains("macos/x86_64"), "{message}");
        assert!(message.contains("976"), "{message}");
    }

    #[test]
    fn windows_x86_64_host_is_qualified() {
        assert_eq!(refusal("windows", "x86_64"), None);
    }
}
