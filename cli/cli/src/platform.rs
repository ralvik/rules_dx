//! Startup platform gate for the `dx` CLI (issue #213).
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
/// Today only the Linux x86_64 seed host is delivered. Provisional: extend
/// this list as ADR 0014 required-platform evidence lands (tracked in
/// issue #213); the startup refusal below reads the same list, so support
/// flips on automatically with the evidence entry.
pub fn qualified_hosts() -> &'static [(&'static str, &'static str)] {
    &[("linux", "x86_64")]
}

/// Clean-refusal diagnostic for an unqualified host, or `None` when the
/// host is qualified. The message carries the qualification pointer and
/// the exact missing evidence (no qualified route for this host yet).
pub fn refusal(os: &str, arch: &str) -> Option<String> {
    if qualified_hosts().contains(&(os, arch)) {
        return None;
    }
    Some(format!(
        "unsupported_platform: {os}/{arch} has no qualified platform evidence; dx is delivered on Linux x86_64 only (see docs/product/support-matrix.md and ADR 0014, tracked in issue #213)"
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
    fn unqualified_hosts_are_refused_with_pointer() {
        for (os, arch) in [
            ("macos", "aarch64"),
            ("macos", "x86_64"),
            ("windows", "x86_64"),
            ("linux", "aarch64"),
        ] {
            let message = refusal(os, arch).expect("unqualified host must be refused");
            assert!(message.starts_with("unsupported_platform"), "{message}");
            assert!(message.contains(&format!("{os}/{arch}")), "{message}");
            assert!(message.contains("213"), "{message}");
        }
    }

    #[test]
    fn qualified_set_is_seed_only() {
        assert_eq!(qualified_hosts(), &[("linux", "x86_64")]);
    }
}
