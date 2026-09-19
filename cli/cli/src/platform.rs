//! Startup platform gate for the `dx` CLI (issues #298, #410).
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
/// The Linux x86_64 seed host plus Linux arm64 glibc native (issue #410)
/// are delivered. Provisional: extend this list as remaining ADR 0014
/// required-platform evidence lands (tracked in issue #298, closed, with
/// per-host successors owning each host); the startup refusal below reads
/// the same list, so support flips on automatically with the evidence entry.
pub fn qualified_hosts() -> &'static [(&'static str, &'static str)] {
    &[("linux", "x86_64"), ("linux", "aarch64")]
}

/// Clean-refusal diagnostic for an unqualified host, or `None` when the
/// host is qualified. The message carries the qualification pointer and
/// the exact missing evidence (no qualified route for this host yet).
pub fn refusal(os: &str, arch: &str) -> Option<String> {
    if qualified_hosts().contains(&(os, arch)) {
        return None;
    }
    Some(format!(
        "unsupported_platform: {os}/{arch} has no qualified platform evidence; dx is delivered on Linux x86_64 and Linux arm64 (glibc) only (see docs/product/support-matrix.md and ADR 0014, tracked in issue #298 with per-host successors such as issue #410)"
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
        // Every remaining ADR 0014 host (required, best-effort, and
        // out-of-v1) refuses cleanly until its per-host evidence lands:
        // static-musl profiles share the same OS/arch pairs as the now
        // qualified Linux glibc hosts, macOS arm64 (required) plus x86_64
        // (best-effort), Windows x86_64 (required, backend blocked)
        // plus arm64 (out of v1).
        for (os, arch) in [
            ("macos", "aarch64"),
            ("macos", "x86_64"),
            ("windows", "x86_64"),
            ("windows", "aarch64"),
        ] {
            let message = refusal(os, arch).expect("unqualified host must be refused");
            assert!(message.starts_with("unsupported_platform"), "{message}");
            assert!(message.contains(&format!("{os}/{arch}")), "{message}");
            assert!(message.contains("298"), "{message}");
        }
    }

    #[test]
    fn qualified_set_is_seed_plus_arm64() {
        assert_eq!(
            qualified_hosts(),
            &[("linux", "x86_64"), ("linux", "aarch64")]
        );
    }
}
