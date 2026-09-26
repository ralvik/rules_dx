"""JVM tool acquisition inventory."""

# Checked-in inventory of the `jvm_tools` extension repos re-exported by the
JVM_TOOL_REPOS = [
    "jvm_checkstyle",
    "jvm_google_java_format",
    "jvm_ktfmt",
    "jvm_ktlint",
    "jvm_pmd_dist",
    "jvm_spotbugs_dist",
]

# Artifact metadata for the `jvm_tools` extension (see extension.bzl).
# Versions mirror `//modules:java-scala-kotlin.bzl` `JVM_TOOL_VERSIONS`;
# digests are the trust anchor, URLs availability only.
JVM_TOOLS = {
    "jvm_google_java_format": {
        "kind": "file",
        "downloaded_file_path": "google-java-format.jar",
        "sha256": "bfb7f9ead6cd328389bc2da53860443bc0e805dfd08cc889bfdf43b26cb2a6e8",
        "urls": [
            "https://repo1.maven.org/maven2/com/google/googlejavaformat/google-java-format/1.35.0/google-java-format-1.35.0-all-deps.jar",
            "https://maven-central.storage-download.googleapis.com/maven2/com/google/googlejavaformat/google-java-format/1.35.0/google-java-format-1.35.0-all-deps.jar",
        ],
    },
    "jvm_checkstyle": {
        "kind": "file",
        "downloaded_file_path": "checkstyle.jar",
        "sha256": "51e2bc7fed1bb56808aa39045f655a316194997acd24bac5195253dcf342b380",
        "urls": ["https://github.com/checkstyle/checkstyle/releases/download/checkstyle-14.1.0/checkstyle-14.1.0-all.jar"],
    },
    "jvm_pmd_dist": {
        "kind": "archive",
        "build_file": "//quality/tools/jvm:pmd_dist.BUILD",
        "sha256": "4ae396ffaf2b0d3ef0b73a10b2925e77066f73d57a4ce9078c60e7302bcddec9",
        "strip_prefix": "pmd-bin-7.27.0",
        "urls": ["https://github.com/pmd/pmd/releases/download/pmd_releases%2F7.27.0/pmd-dist-7.27.0-bin.zip"],
    },
    "jvm_spotbugs_dist": {
        "kind": "archive",
        "build_file": "//quality/tools/jvm:spotbugs_dist.BUILD",
        "sha256": "72bc0d4edd686e462c0f71f42a049b27bf4da6708797ff7b2b56dd202714b4e5",
        "strip_prefix": "spotbugs-4.10.4",
        "urls": ["https://github.com/spotbugs/spotbugs/releases/download/4.10.4/spotbugs-4.10.4.tgz"],
    },
    "jvm_ktfmt": {
        "kind": "file",
        "downloaded_file_path": "ktfmt.jar",
        "sha256": "a015521ddb1c7a80c41edb56b91b4a231439592ffd2e85ac866ff8134c37c112",
        "urls": [
            "https://repo1.maven.org/maven2/com/facebook/ktfmt/0.63/ktfmt-0.63-with-dependencies.jar",
            "https://maven-central.storage-download.googleapis.com/maven2/com/facebook/ktfmt/0.63/ktfmt-0.63-with-dependencies.jar",
        ],
    },
    "jvm_ktlint": {
        "kind": "file",
        "downloaded_file_path": "ktlint.jar",
        "sha256": "369ad2b789f95a011f807e1fcb690ccef80bd7cd014fd139e73ae82dcc0baeab",
        "urls": ["https://repo1.maven.org/maven2/com/pinterest/ktlint/ktlint-cli/1.8.0/ktlint-cli-1.8.0-all.jar"],
    },
}
