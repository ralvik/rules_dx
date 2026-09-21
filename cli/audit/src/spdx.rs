//! SPDX 2.3 JSON reporting for `dx audit license`.
//!
//! Pure rendering of the license-family report shape pinned in
//! [`crate::license_notice`]: one SPDX 2.3 JSON document per
//! invocation, package IDs as package URLs, `DESCRIBES` relations from
//! each audited root, and `CONTAINS` relations where the lock graph is
//! known. Per-root attribution rides the relationships; per-ecosystem
//! license identities ride the package license fields parsed via
//! [`crate::license_expr::parse_license`].
//!
//! This module renders over injected package and relationship records
//! only, so the document shape stays deterministic and unit-testable
//! without any lockfile, advisory snapshot, or TOML policy file.
//! Policy-table loading lives in [`crate::license_policy`],
//! expression evaluation in [`crate::license_expr`], and notice-text
//! inputs in [`crate::license_notice`]; this module projects their
//! assessed outputs into the shared `--report` document.
//!
//! Shape pinned under issue #632 (See: `docs/cli/commands/audit-update-bazel.md#dx-audit`): exactly one document per invocation
//! (`SPDX-2.3`, `CC0-1.0`, `SPDXRef-DOCUMENT`, name `dx-audit-license`),
//! packages sorted by ID with `licenseConcluded`/`licenseDeclared`,
//! `NOASSERTION` copyright, single purl `externalRefs`, plus
//! `DESCRIBES` from each audited root first (sorted) then `CONTAINS`
//! (V1 emits none: no lock-graph edges projected yet). Live emission
//! goldens live in `dx_cli::exec::audit`; partial documents stay
//! non-authoritative (gated on `results_complete`, never uploaded as a
//! replacement scan).

use packageurl::PackageUrl;
use serde::{Deserialize, Serialize};

/// SPDX document version pinned by the license contract.
pub const SPDX_VERSION: &str = "2.3";

/// SPDX data license for generated documents.
pub const DATA_LICENSE: &str = "CC0-1.0";

/// Document describes relationship: each audited root describes the document.
pub const DESCRIBES: &str = "DESCRIBES";

/// Containment relationship: lock-graph containment where known.
pub const CONTAINS: &str = "CONTAINS";

/// One SPDX package entry: an audited dependency with its claimed
/// license and package-URL identity.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SpdxPackage {
    /// SPDX package ID (package-URL form, e.g. `pkg:cargo/serde@1.0.100`).
    #[serde(rename = "SPDXID")]
    pub id: String,
    /// Package name.
    pub name: String,
    /// Locked version.
    #[serde(rename = "versionInfo")]
    pub version: String,
    /// Claimed license expression text (SPDX or `NOASSERTION`).
    #[serde(rename = "licenseConcluded")]
    pub license: String,
    /// Declared license text (same as concluded for V1 local matching).
    #[serde(rename = "licenseDeclared")]
    pub declared: String,
    /// Copyright text when collected, else `NOASSERTION`.
    #[serde(rename = "copyrightText")]
    pub copyright: String,
    /// Package-URL external reference.
    #[serde(rename = "externalRefs")]
    pub external: Vec<SpdxExternalRef>,
}

/// One SPDX external reference (package-URL identity).
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SpdxExternalRef {
    /// Reference category (`PACKAGE-MANAGER`).
    #[serde(rename = "referenceCategory")]
    pub category: String,
    /// Reference type (`purl`).
    #[serde(rename = "referenceType")]
    pub ref_type: String,
    /// Package URL string.
    #[serde(rename = "referenceLocator")]
    pub locator: String,
}

/// One SPDX relationship: audited roots describe the document;
/// containment records the lock graph where known.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SpdxRelationship {
    /// Source element ID.
    #[serde(rename = "spdxElementId")]
    pub from: String,
    /// Relationship type (`DESCRIBES` or `CONTAINS`).
    #[serde(rename = "relationshipType")]
    pub rel_type: String,
    /// Target element ID.
    #[serde(rename = "relatedSpdxElement")]
    pub to: String,
}

/// One SPDX 2.3 JSON document per audit invocation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SpdxDocument {
    /// SPDX version (`2.3`).
    #[serde(rename = "spdxVersion")]
    pub version: String,
    /// Data license (`CC0-1.0`).
    #[serde(rename = "dataLicense")]
    pub data_license: String,
    /// Document ID (`SPDXRef-DOCUMENT`).
    #[serde(rename = "SPDXID")]
    pub id: String,
    /// Document name.
    pub name: String,
    /// Document namespace (invocation-unique URI).
    #[serde(rename = "documentNamespace")]
    pub namespace: String,
    /// Audited packages.
    pub packages: Vec<SpdxPackage>,
    /// Root attribution and containment.
    pub relationships: Vec<SpdxRelationship>,
}

/// Build one package-URL locator for a locked package via
/// [`PackageUrl`]: V1 shapes are `pkg:<ecosystem>/<name>@<version>`
/// with ecosystem `cargo`, `npm`, `maven`, `nuget`, or `golang`.
/// Names with scopes keep their spelling; Maven `group:artifact`
/// maps to `pkg:maven/<group>/<artifact>`. Encoding and
/// canonicalization follow the purl spec (e.g. npm `@scope` encodes
/// as `%40scope`, names/versions percent-encode); simple
/// `cargo/npm/maven/nuget/go/generic` vectors stay byte-stable.
/// Falls back to the legacy `format!` shape when the builder rejects
/// an edge input so rendering stays infallible.
pub fn package_url(set: &str, name: &str, version: &str) -> String {
    // Split `a/b/c` into namespace `a/b` + name `c` so slashes stay
    // separators instead of `%2F` (go paths, npm scopes, generic).
    fn split_namespace(full: &str) -> (Option<&str>, &str) {
        match full.rsplit_once('/') {
            Some((ns, base)) if !ns.is_empty() && !base.is_empty() => (Some(ns), base),
            _ => (None, full),
        }
    }

    fn build(ty: &str, namespace: Option<&str>, name: &str, version: &str) -> Option<String> {
        let mut purl = PackageUrl::new(ty, name).ok()?;
        if let Some(ns) = namespace {
            purl.with_namespace(ns).ok()?;
        }
        purl.with_version(version).ok()?;
        Some(purl.to_string())
    }

    match set {
        "maven" => {
            if let Some((group, artifact)) = name.split_once(':') {
                if !group.is_empty() && !artifact.is_empty() && !artifact.contains('/') {
                    if let Some(text) = build("maven", Some(group), artifact, version) {
                        return text;
                    }
                } else if !artifact.is_empty() && artifact.contains('/') {
                    // Unusual `group:a/b`: keep slashes as separators.
                    let (ns_extra, base) = split_namespace(artifact);
                    if !base.is_empty() {
                        let ns = match ns_extra {
                            Some(extra) => format!("{group}/{extra}"),
                            None => group.to_owned(),
                        };
                        if let Some(text) = build("maven", Some(&ns), base, version) {
                            return text;
                        }
                    }
                }
                format!("pkg:maven/{group}/{artifact}@{version}")
            } else {
                build("maven", None, name, version)
                    .unwrap_or_else(|| format!("pkg:maven/{name}@{version}"))
            }
        }
        "npm" => {
            let (ns, base) = split_namespace(name);
            build("npm", ns, base, version).unwrap_or_else(|| format!("pkg:npm/{name}@{version}"))
        }
        "cargo" => build("cargo", None, name, version)
            .unwrap_or_else(|| format!("pkg:cargo/{name}@{version}")),
        "nuget" => build("nuget", None, name, version)
            .unwrap_or_else(|| format!("pkg:nuget/{name}@{version}")),
        "go" => {
            let (ns, base) = split_namespace(name);
            build("golang", ns, base, version)
                .unwrap_or_else(|| format!("pkg:golang/{name}@{version}"))
        }
        _ => {
            let (ns, base) = split_namespace(name);
            build("generic", ns, base, version)
                .unwrap_or_else(|| format!("pkg:generic/{name}@{version}"))
        }
    }
}

/// Render one SPDX 2.3 JSON document: packages sorted by ID for
/// determinism, relationships with `DESCRIBES` from each audited root
/// first (sorted), then `CONTAINS` where the lock graph is known
/// (sorted). Exactly one document per invocation, never one per
/// package, set, or root.
pub fn render_spdx(
    roots: &[String],
    packages: &[SpdxPackage],
    contains: &[(String, String)],
    namespace: &str,
) -> String {
    let mut sorted_packages = packages.to_vec();
    sorted_packages.sort_by(|a, b| a.id.cmp(&b.id));
    let mut sorted_roots = roots.to_vec();
    sorted_roots.sort();
    let mut sorted_contains = contains.to_vec();
    sorted_contains.sort();
    let mut relationships = Vec::new();
    for root in &sorted_roots {
        relationships.push(SpdxRelationship {
            from: root.clone(),
            rel_type: DESCRIBES.to_owned(),
            to: "SPDXRef-DOCUMENT".to_owned(),
        });
    }
    for (from, to) in &sorted_contains {
        relationships.push(SpdxRelationship {
            from: from.clone(),
            rel_type: CONTAINS.to_owned(),
            to: to.clone(),
        });
    }
    let document = SpdxDocument {
        version: format!("SPDX-{SPDX_VERSION}"),
        data_license: DATA_LICENSE.to_owned(),
        id: "SPDXRef-DOCUMENT".to_owned(),
        name: "dx-audit-license".to_owned(),
        namespace: namespace.to_owned(),
        packages: sorted_packages,
        relationships,
    };
    serde_json::to_string_pretty(&document).unwrap_or_else(|_| "{}".to_owned())
}

/// Build one SPDX package entry from a locked package plus its claimed
/// license text. Copyright defaults to `NOASSERTION` in V1 (notice
/// texts are collected as inputs per [`crate::license_notice`], not
/// aggregated into releases yet).
pub fn spdx_package(
    set: &str,
    name: &str,
    version: &str,
    license: &str,
    index: usize,
) -> SpdxPackage {
    let locator = package_url(set, name, version);
    SpdxPackage {
        id: format!("SPDXRef-Package-{index}"),
        name: name.to_owned(),
        version: version.to_owned(),
        license: license.to_owned(),
        declared: license.to_owned(),
        copyright: "NOASSERTION".to_owned(),
        external: vec![SpdxExternalRef {
            category: "PACKAGE-MANAGER".to_owned(),
            ref_type: "purl".to_owned(),
            locator,
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_urls_follow_purl_shapes() {
        assert_eq!(
            package_url("cargo", "serde", "1.0.100"),
            "pkg:cargo/serde@1.0.100"
        );
        assert_eq!(
            package_url("npm", "react", "18.2.0"),
            "pkg:npm/react@18.2.0"
        );
        assert_eq!(
            package_url("maven", "junit:junit", "4.13.2"),
            "pkg:maven/junit/junit@4.13.2"
        );
        assert_eq!(
            package_url("nuget", "FSharp.Core", "8.0.0"),
            "pkg:nuget/FSharp.Core@8.0.0"
        );
        assert_eq!(
            package_url("go", "example.com/hello", "1.0.0"),
            "pkg:golang/example.com/hello@1.0.0"
        );
    }

    #[test]
    fn package_urls_round_trip_through_packageurl() {
        use std::str::FromStr;
        let vectors: &[(&str, &str, &str, &str, Option<&str>, &str, &str, &str)] = &[
            (
                "cargo",
                "serde",
                "1.0.100",
                "cargo",
                None,
                "serde",
                "1.0.100",
                "pkg:cargo/serde@1.0.100",
            ),
            (
                "npm",
                "react",
                "18.2.0",
                "npm",
                None,
                "react",
                "18.2.0",
                "pkg:npm/react@18.2.0",
            ),
            (
                "maven",
                "junit:junit",
                "4.13.2",
                "maven",
                Some("junit"),
                "junit",
                "4.13.2",
                "pkg:maven/junit/junit@4.13.2",
            ),
            (
                "nuget",
                "FSharp.Core",
                "8.0.0",
                "nuget",
                None,
                "FSharp.Core",
                "8.0.0",
                "pkg:nuget/FSharp.Core@8.0.0",
            ),
            (
                "go",
                "example.com/hello",
                "1.0.0",
                "golang",
                Some("example.com"),
                "hello",
                "1.0.0",
                "pkg:golang/example.com/hello@1.0.0",
            ),
            (
                "go",
                "github.com/gorilla/mux",
                "1.8.0",
                "golang",
                Some("github.com/gorilla"),
                "mux",
                "1.8.0",
                "pkg:golang/github.com/gorilla/mux@1.8.0",
            ),
            (
                "npm",
                "@angular/animation",
                "12.3.1",
                "npm",
                Some("@angular"),
                "animation",
                "12.3.1",
                "pkg:npm/%40angular/animation@12.3.1",
            ),
            (
                "maven",
                "single",
                "1.0.0",
                "maven",
                None,
                "single",
                "1.0.0",
                "pkg:maven/single@1.0.0",
            ),
            (
                "other",
                "mytool",
                "2.0.0",
                "generic",
                None,
                "mytool",
                "2.0.0",
                "pkg:generic/mytool@2.0.0",
            ),
        ];
        for (set, name, version, ty, ns, base, ver, expected) in vectors {
            let text = package_url(set, name, version);
            assert_eq!(&text, expected, "stable purl for {set}:{name}@{version}");
            let parsed = PackageUrl::from_str(&text).expect("parse-back valid purl");
            assert_eq!(parsed.ty(), *ty);
            assert_eq!(parsed.namespace(), *ns);
            assert_eq!(parsed.name(), *base);
            assert_eq!(parsed.version(), Some(*ver));
            assert_eq!(parsed.to_string(), text, "Display idempotent");
        }
    }

    #[test]
    fn spdx_document_pins_version_package_urls_and_relations() {
        let roots = vec!["//services/payments:image".to_owned()];
        let packages = vec![
            spdx_package("cargo", "serde", "1.0.100", "MIT", 1),
            spdx_package("npm", "react", "18.2.0", "MIT", 2),
        ];
        let contains = vec![(
            "SPDXRef-Package-1".to_owned(),
            "SPDXRef-Package-2".to_owned(),
        )];
        let text = render_spdx(
            &roots,
            &packages,
            &contains,
            "https://example.com/dx-audit-1",
        );
        let value: serde_json::Value = serde_json::from_str(&text).expect("valid JSON");
        assert_eq!(value["spdxVersion"], serde_json::json!("SPDX-2.3"));
        assert_eq!(value["dataLicense"], serde_json::json!("CC0-1.0"));
        assert_eq!(value["SPDXID"], serde_json::json!("SPDXRef-DOCUMENT"));
        let pkgs = value["packages"].as_array().expect("packages");
        assert_eq!(pkgs.len(), 2);
        assert_eq!(
            pkgs[0]["externalRefs"][0]["referenceLocator"],
            serde_json::json!("pkg:cargo/serde@1.0.100")
        );
        let rels = value["relationships"].as_array().expect("relationships");
        assert_eq!(rels[0]["relationshipType"], serde_json::json!("DESCRIBES"));
        assert_eq!(
            rels[0]["spdxElementId"],
            serde_json::json!("//services/payments:image")
        );
        assert_eq!(rels[1]["relationshipType"], serde_json::json!("CONTAINS"));
    }

    #[test]
    fn spdx_render_is_deterministic_over_input_order() {
        let roots = vec!["//b:two".to_owned(), "//a:one".to_owned()];
        let packages = vec![
            spdx_package("npm", "react", "18.2.0", "MIT", 2),
            spdx_package("cargo", "serde", "1.0.100", "MIT", 1),
        ];
        let first = render_spdx(&roots, &packages, &[], "ns");
        let mut rev_roots = roots.clone();
        rev_roots.reverse();
        let mut rev_pkgs = packages.clone();
        rev_pkgs.reverse();
        let second = render_spdx(&rev_roots, &rev_pkgs, &[], "ns");
        assert_eq!(first, second);
    }

    #[test]
    fn frozen_shape_constants() {
        assert_eq!(SPDX_VERSION, "2.3");
        assert_eq!(DATA_LICENSE, "CC0-1.0");
        assert_eq!(DESCRIBES, "DESCRIBES");
        assert_eq!(CONTAINS, "CONTAINS");
    }

    #[test]
    fn spdx_golden_pins_full_document_shape() {
        // Issue #632 (See: `docs/cli/commands/audit-update-bazel.md#dx-audit`): one document per invocation with the frozen
        // envelope, per-package purl identities, and ordered
        // DESCRIBES-then-CONTAINS relations. V1 live emission carries
        // no CONTAINS edges (no lock-graph projection yet); the golden
        // below pins the empty-CONTAINS live shape plus one explicit
        // CONTAINS edge for the unit projection.
        let roots = vec!["//b:two".to_owned(), "//a:one".to_owned()];
        let packages = vec![
            spdx_package("maven", "junit:junit", "4.13.2", "EPL-1.0", 2),
            spdx_package("cargo", "serde", "1.0.100", "MIT", 1),
            spdx_package("go", "example.com/hello", "1.0.0", "NOASSERTION", 3),
        ];
        let text = render_spdx(&roots, &packages, &[], "https://example.com/dx-audit-1");
        let value: serde_json::Value = serde_json::from_str(&text).expect("valid JSON");
        // Envelope: exactly one document per invocation, never per
        // package/set/root.
        assert_eq!(value["spdxVersion"], serde_json::json!("SPDX-2.3"));
        assert_eq!(value["dataLicense"], serde_json::json!("CC0-1.0"));
        assert_eq!(value["SPDXID"], serde_json::json!("SPDXRef-DOCUMENT"));
        assert_eq!(value["name"], serde_json::json!("dx-audit-license"));
        assert_eq!(
            value["documentNamespace"],
            serde_json::json!("https://example.com/dx-audit-1")
        );
        // Packages sorted by ID for determinism with the full V1 field
        // set: concluded/declared, NOASSERTION copyright, single purl ref.
        let pkgs = value["packages"].as_array().expect("packages");
        assert_eq!(pkgs.len(), 3);
        assert_eq!(pkgs[0]["SPDXID"], serde_json::json!("SPDXRef-Package-1"));
        assert_eq!(pkgs[0]["name"], serde_json::json!("serde"));
        assert_eq!(pkgs[0]["versionInfo"], serde_json::json!("1.0.100"));
        assert_eq!(pkgs[0]["licenseConcluded"], serde_json::json!("MIT"));
        assert_eq!(pkgs[0]["licenseDeclared"], serde_json::json!("MIT"));
        assert_eq!(pkgs[0]["copyrightText"], serde_json::json!("NOASSERTION"));
        assert_eq!(
            pkgs[0]["externalRefs"],
            serde_json::json!([{
                "referenceCategory": "PACKAGE-MANAGER",
                "referenceType": "purl",
                "referenceLocator": "pkg:cargo/serde@1.0.100",
            }])
        );
        assert_eq!(
            pkgs[1]["externalRefs"][0]["referenceLocator"],
            serde_json::json!("pkg:maven/junit/junit@4.13.2")
        );
        assert_eq!(
            pkgs[2]["externalRefs"][0]["referenceLocator"],
            serde_json::json!("pkg:golang/example.com/hello@1.0.0")
        );
        // Relationships: DESCRIBES from each audited root first
        // (sorted), then CONTAINS (empty in V1 live emission).
        let rels = value["relationships"].as_array().expect("relationships");
        assert_eq!(rels.len(), 2);
        assert_eq!(
            rels[0],
            serde_json::json!({
                "spdxElementId": "//a:one",
                "relationshipType": "DESCRIBES",
                "relatedSpdxElement": "SPDXRef-DOCUMENT",
            })
        );
        assert_eq!(
            rels[1],
            serde_json::json!({
                "spdxElementId": "//b:two",
                "relationshipType": "DESCRIBES",
                "relatedSpdxElement": "SPDXRef-DOCUMENT",
            })
        );
        // Explicit CONTAINS projection stays ordered after DESCRIBES.
        let with_contains = render_spdx(
            &["//a:one".to_owned()],
            &packages[..2],
            &[(
                "SPDXRef-Package-2".to_owned(),
                "SPDXRef-Package-1".to_owned(),
            )],
            "ns",
        );
        let with_value: serde_json::Value =
            serde_json::from_str(&with_contains).expect("valid JSON");
        let with_rels = with_value["relationships"].as_array().expect("rels");
        assert_eq!(with_rels.len(), 2);
        assert_eq!(
            with_rels[0]["relationshipType"],
            serde_json::json!("DESCRIBES")
        );
        assert_eq!(
            with_rels[1]["relationshipType"],
            serde_json::json!("CONTAINS")
        );
        // Partial documents stay non-authoritative: the shape carries no
        // completeness flag itself; callers gate authoritative upload on
        // `results_complete` (see `dx_cli::exec::audit`).
    }
}
