"""Layer-2 adapter-less plus composition plus depcheck opens pins."""

FRAMEWORK_VUE = "vue"
FRAMEWORK_SVELTE = "svelte"
FRAMEWORK_ASTRO = "astro"
FRAMEWORK_MDX = "mdx"

REJECTED_ADAPTER_LESS_AS_PASS = "adapter-less as pass rejected"
REJECTED_FRAMEWORK_FALLBACK = "generic container parser plus regex extraction plus implicit JS/TS fallback rejected"
CLOSED_303_ONLY = "Closed #303 only"
ADAPTERS_416_420_PARTIAL = "adapters partially under #416-420 with digests plus adapters staying owned"
NO_SUPPORTED = "no Supported claim"
SEED_ONLY = "qualified seed-only under issue #510"

COMPOSITION_FIXTURE = "examples/mixed/hello"
COMPOSITION_VUE = "examples/mixed/hello/Hello.vue"
COMPOSITION_SVELTE = "examples/mixed/hello/Hello.svelte"
COMPOSITION_ASTRO = "examples/mixed/hello/Hello.astro"
COMPOSITION_MDX = "examples/mixed/hello/Hello.mdx"
COMPOSITION_HELPER = "examples/mixed/hello/helper.js"
COMPOSITION_TEST = "examples/mixed/hello:hello_test"
GAZELLE_MIXED = "gazelle/mixed"
GAZELLE_MIXED_OWNER = "Owner"
GAZELLE_MIXED_OWNERS = "Owners"

DEPCHECK_FIXTURES = "tools/depcheck/testdata"
DEPCHECK_PNPM_ROUTE = "framework-composition depcheck stays with the JS/TS pnpm route"
DEPCHECK_LANGUAGES = [
    "rust",
    "python",
    "js",
    "ts",
    "go",
    "java",
    "kotlin",
    "scala",
    "csharp",
    "fsharp",
    "cc",
]

OWNING_JVM_COHORT = "adapters delivered under #796 (successor to closed #416); digests stay owned"
OWNING_SCALA_DOTNET_COHORT = "adapters delivered under #797 (successor to closed #417); digests stay owned"
OWNING_NATIVE_COHORT = "adapters delivered under #798 (successor to closed #418); digests stay owned"
OWNING_STRUCTURED_COHORT = "digests plus adapters stay owned under #419"
OWNING_FILE_COHORT = "digests plus adapters stay owned under #420"
OWNING_DEFAULTS_485 = "versions plus rule-sets qualified under #485"
OWNING_DEFAULTS_486 = "versions plus rule-sets qualified under #486"
OWNING_DEFAULTS_487 = "versions plus rule-sets qualified under #487"
OWNING_FOUNDATIONS_476_484 = "wrappers plus runners plus locks qualified under #476-#484"

LAYER2_OPENS_PROOF = "bazel build //quality/tests/fixtures/layer2_opens:corpus_starlark plus bazel build //examples/mixed/hello/..."
