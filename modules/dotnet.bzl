RULES_DOTNET_VERSION = "0.22.1"
BAZEL_LIB_VERSION = "3.7.0"
DOTNET_VERSION = "10.0.201"

PAKET_DEPENDENCIES = "//third_party/dotnet/paket.dependencies"
PAKET_LOCK = "//third_party/dotnet/paket.lock"
PAKET_REPIN = "bazel run @rules_dotnet//tools/paket2bazel -- --dependencies-file $PWD/third_party/dotnet/paket.dependencies --output-folder $PWD/third_party/dotnet/deps"

PAKET_MEMBERS = [
    "FSharp.Core 10.1.201",
    "xunit.v3 4.0.0",
    "xunit.analyzers 2.0.0",
]
