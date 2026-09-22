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
/// delivered. Provisional: extend this list as remaining ADR
/// 0014 required-platform evidence lands (tracked in, closed,
/// with per-host successors owning each host); the startup refusal below
/// reads the same list, so support flips on automatically with the
/// evidence entry.
///
/// Static-musl profiles share these OS/arch pairs: the two
/// Linux hosts above run static-musl closures natively, so no new host
/// entry is needed for them. See [`qualified_static_musl_profiles`] for
/// the target-profile list that flips independently.
///
/// macOS arm64 runs natively on `macos-14` (arm64) runners
/// through the pinned upstream toolchains; the hermetic-llvm Apple-SDK
/// backend stays provisional with immutable lazy fetch and no
/// host-installed SDK fallback (never approved). Exact pins, hosts,
/// floors, and SDK/CRT identities stay owned per ADR 0014.
///
/// macOS x86_64 is Not planned and never planned for support (issue #976):
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

/// Static-musl target profiles with platform evidence.
///
/// Hermetic-llvm static-only musl targets on x86_64/arm64; dynamic musl
/// stays explicitly out of scope per ADR 0014. Build scripts and proc
/// macros keep execution-platform tools while applications link target
/// musl libraries (docs/generation/rust.md#build-scripts); prebuilt glibc
/// libraries never become musl-compatible by linker change alone.
/// Exact pins, hosts, floors, and SDK/CRT identities stay owned
/// per ADR 0014 and are not pinned here.
pub fn qualified_static_musl_profiles() -> &'static [&'static str] {
    &["linux_x86_64_static_musl", "linux_arm64_static_musl"]
}

/// Clean-refusal diagnostic for an unqualified host, or `None` when the
/// host is qualified. The message carries the qualification pointer and
/// the exact missing evidence (no qualified route for this host yet).
pub fn refusal(os: &str, arch: &str) -> Option<String> {
    if qualified_hosts().contains(&(os, arch)) {
        return None;
    }
    Some(format!(
        "unsupported_platform: {os}/{arch} has no qualified platform evidence; dx is delivered on Linux x86_64 and Linux arm64 (glibc plus static musl, issue #411; dynamic musl explicitly out of scope) plus macOS arm64 (issue #412; host-installed SDK fallback never approved) plus Windows x86_64 MSVC-compatible (issue #414; toolchains_msvc clang-cl/Microsoft-STL provisional, explicit EULA acceptance never automatic) only (See: docs/product/support-matrix.md and ADR 0014, tracked in issue #298 with per-host successors such as issues #410/#411/#412/#414; macOS x86_64 is Not planned per issue #976)"
    ))
}

/// Clean-refusal diagnostic for a musl profile name, or `None` when the
/// static profile is qualified. Dynamic musl is explicitly refused: it is
/// not an initial requirement per ADR 0014 and has no static-closure
/// evidence. Any other spelling is an unknown profile, also refused.
pub fn musl_profile_refusal(profile: &str) -> Option<String> {
    if qualified_static_musl_profiles().contains(&profile) {
        return None;
    }
    if profile.contains("musl") {
        return Some(format!(
            "unsupported_platform: {profile} has no qualified static-musl evidence; only linux_x86_64_static_musl and linux_arm64_static_musl are qualified (issue #411; dynamic musl explicitly out of scope, See: docs/product/support-matrix.md and ADR 0014)"
        ));
    }
    Some(format!(
        "unsupported_platform: {profile} is not a qualified static-musl profile; only linux_x86_64_static_musl and linux_arm64_static_musl are qualified (issue #411; See: docs/product/support-matrix.md)"
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
        // Not planned and never planned per issue #976: no CI/coverage
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

    #[test]
    fn static_musl_profiles_are_qualified() {
        assert_eq!(
            qualified_static_musl_profiles(),
            &["linux_x86_64_static_musl", "linux_arm64_static_musl"]
        );
        assert_eq!(musl_profile_refusal("linux_x86_64_static_musl"), None);
        assert_eq!(musl_profile_refusal("linux_arm64_static_musl"), None);
    }

    #[test]
    fn dynamic_musl_is_explicitly_refused() {
        // Dynamic musl stays explicitly out of scope per ADR 0014 (issue
        // every dynamic spelling refuses with the static-only
        // pointer, never a silent fallback.
        for profile in [
            "linux_x86_64_dynamic_musl",
            "linux_arm64_dynamic_musl",
            "linux_x86_64_musl_dynamic",
            "linux_x86_64_musl",
        ] {
            let message = musl_profile_refusal(profile).expect("dynamic musl must be refused");
            assert!(message.starts_with("unsupported_platform"), "{message}");
            assert!(message.contains(profile), "{message}");
            assert!(message.contains("dynamic musl"), "{message}");
            assert!(message.contains("411"), "{message}");
        }
    }

    #[test]
    fn unknown_profiles_are_refused() {
        let message =
            musl_profile_refusal("linux_x86_64_glibc").expect("unknown profile must be refused");
        assert!(message.starts_with("unsupported_platform"), "{message}");
        assert!(message.contains("411"), "{message}");
    }
}
