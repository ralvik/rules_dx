"""Wrapper conformance tests (M02).

`dx_wrapper_registry_tests` pins the `QualitySourcesInfo` registry facts the
wrappers rely on. `dx_wrapper_conformance_tests` observes one wrapper subject
per `dx_rust_*` shape (library, binary, `crate =` test, proc macro, shared
library, static library) and pins the full rendering: provider preservation,
single source owner, lint markers from the pinned toolchain, and pinned tool
identities. The Cc-linking shapes (shared/static) are observed through
`dx_wrapper_cc_subject`: upstream provides no `CrateInfo` there, so crate
facts come from the `TestCrateInfo`-wrapped crate and the linking surface is
pinned by `CcInfo` linker-input count.
"""

load("//libs/starlark:defs.bzl", "expect_equal", "starlark_test")
load("//quality:sources.bzl", "KNOWN_SEMANTIC_FILE_CLASSES", "RUST")

def dx_wrapper_registry_tests(name):
    starlark_test(
        name = name,
        mode = "unit",
        checks = [
            expect_equal("RUST class id", RUST, "rust"),
            expect_equal("rust is a known class", RUST in KNOWN_SEMANTIC_FILE_CLASSES, True),
        ],
    )

# Filled in from the observed rendering on the pinned stack; any wrapper or
# upstream change that alters providers, owners, markers, or tool identities
# fails here first.
EXPECTED_OBSERVATIONS = """subject //rust/hello:hello_cdylib_subject
file hello_cdylib_subject.txt
field cargo_tool=cargo
field cc_linker_inputs=2
field clippy_markers=hello_cdylib.clippy.ok,hello_derive.clippy.ok,hello_staticlib.clippy.ok
field clippy_tool=clippy-driver
field crate_edition=2021
field crate_is_test=False
field crate_name=hello_cdylib
field crate_owner=//rust/hello:hello_cdylib_dx_upstream
field crate_root=cdylib.rs
field crate_srcs=cdylib.rs
field crate_type=cdylib
field direct_sources=rust:cdylib.rs
field fmt_markers=hello_cdylib.rustfmt.ok,hello_derive.rustfmt.ok,hello_staticlib.rustfmt.ok
field preserved_cc_inputs=True
field preserved_deps=True
field preserved_edition=True
field preserved_name=True
field preserved_root=True
field preserved_srcs=True
field preserved_type=True
field rustc_tool=rustc
field rustfmt_tool=rustfmt
field tools_pinned=True
field upstream=//rust/hello:hello_cdylib_dx_upstream
field upstream_has_instrumented_files=True
field upstream_has_quality_sources=False
field wrapper=//rust/hello:hello_cdylib
field wrapper_has_instrumented_files=True
field wrapper_has_quality_sources=True
subject //rust/hello:hello_derive_subject
file hello_derive_subject.txt
field cargo_tool=cargo
field clippy_markers=hello_cdylib.clippy.ok,hello_derive.clippy.ok,hello_staticlib.clippy.ok
field clippy_tool=clippy-driver
field crate_edition=2021
field crate_is_test=False
field crate_name=hello_derive
field crate_owner=//rust/hello:hello_derive_dx_upstream
field crate_root=derive.rs
field crate_srcs=derive.rs
field crate_type=proc-macro
field direct_sources=rust:derive.rs
field fmt_markers=hello_cdylib.rustfmt.ok,hello_derive.rustfmt.ok,hello_staticlib.rustfmt.ok
field preserved_deps=True
field preserved_edition=True
field preserved_name=True
field preserved_root=True
field preserved_srcs=True
field preserved_type=True
field rustc_tool=rustc
field rustfmt_tool=rustfmt
field tools_pinned=True
field upstream=//rust/hello:hello_derive_dx_upstream
field upstream_has_instrumented_files=True
field upstream_has_quality_sources=False
field wrapper=//rust/hello:hello_derive
field wrapper_has_instrumented_files=True
field wrapper_has_quality_sources=True
subject //rust/hello:hello_lib_subject
file hello_lib_subject.txt
field cargo_tool=cargo
field clippy_markers=hello.clippy.ok,hello_lib.clippy.ok
field clippy_tool=clippy-driver
field crate_edition=2021
field crate_is_test=False
field crate_name=hello
field crate_owner=//rust/hello:hello_lib_dx_upstream
field crate_root=lib.rs
field crate_srcs=lib.rs
field crate_type=rlib
field direct_sources=rust:lib.rs
field fmt_markers=hello.rustfmt.ok,hello_lib.rustfmt.ok
field preserved_deps=True
field preserved_edition=True
field preserved_name=True
field preserved_root=True
field preserved_srcs=True
field preserved_type=True
field rustc_tool=rustc
field rustfmt_tool=rustfmt
field tools_pinned=True
field upstream=//rust/hello:hello_lib_dx_upstream
field upstream_has_instrumented_files=True
field upstream_has_quality_sources=False
field wrapper=//rust/hello:hello_lib
field wrapper_has_instrumented_files=True
field wrapper_has_quality_sources=True
subject //rust/hello:hello_staticlib_subject
file hello_staticlib_subject.txt
field cargo_tool=cargo
field cc_linker_inputs=2
field clippy_markers=hello_cdylib.clippy.ok,hello_derive.clippy.ok,hello_staticlib.clippy.ok
field clippy_tool=clippy-driver
field crate_edition=2021
field crate_is_test=False
field crate_name=hello_staticlib
field crate_owner=//rust/hello:hello_staticlib_dx_upstream
field crate_root=staticlib.rs
field crate_srcs=staticlib.rs
field crate_type=staticlib
field direct_sources=rust:staticlib.rs
field fmt_markers=hello_cdylib.rustfmt.ok,hello_derive.rustfmt.ok,hello_staticlib.rustfmt.ok
field preserved_cc_inputs=True
field preserved_deps=True
field preserved_edition=True
field preserved_name=True
field preserved_root=True
field preserved_srcs=True
field preserved_type=True
field rustc_tool=rustc
field rustfmt_tool=rustfmt
field tools_pinned=True
field upstream=//rust/hello:hello_staticlib_dx_upstream
field upstream_has_instrumented_files=True
field upstream_has_quality_sources=False
field wrapper=//rust/hello:hello_staticlib
field wrapper_has_instrumented_files=True
field wrapper_has_quality_sources=True
subject //rust/hello:hello_subject
file hello_subject.txt
field cargo_tool=cargo
field clippy_markers=hello.clippy.ok,hello_lib.clippy.ok
field clippy_tool=clippy-driver
field crate_edition=2021
field crate_is_test=False
field crate_name=hello
field crate_owner=//rust/hello:hello_dx_upstream
field crate_root=main.rs
field crate_srcs=main.rs
field crate_type=bin
field direct_sources=rust:main.rs
field fmt_markers=hello.rustfmt.ok,hello_lib.rustfmt.ok
field preserved_deps=True
field preserved_edition=True
field preserved_name=True
field preserved_root=True
field preserved_srcs=True
field preserved_type=True
field rustc_tool=rustc
field rustfmt_tool=rustfmt
field tools_pinned=True
field upstream=//rust/hello:hello_dx_upstream
field upstream_has_instrumented_files=True
field upstream_has_quality_sources=False
field wrapper=//rust/hello:hello
field wrapper_has_instrumented_files=True
field wrapper_has_quality_sources=True
subject //rust/hello:hello_test_subject
file hello_test_subject.txt
field cargo_tool=cargo
field clippy_markers=hello.clippy.ok,hello_lib.clippy.ok
field clippy_tool=clippy-driver
field crate_edition=2021
field crate_is_test=True
field crate_name=hello
field crate_owner=//rust/hello:hello_test_dx_upstream
field crate_root=lib.rs
field crate_srcs=lib.rs
field crate_type=bin
field direct_sources=(none)
field fmt_markers=hello.rustfmt.ok,hello_lib.rustfmt.ok
field preserved_deps=True
field preserved_edition=True
field preserved_name=True
field preserved_root=True
field preserved_srcs=True
field preserved_type=True
field rustc_tool=rustc
field rustfmt_tool=rustfmt
field tools_pinned=True
field upstream=//rust/hello:hello_test_dx_upstream
field upstream_has_instrumented_files=True
field upstream_has_quality_sources=False
field wrapper=//rust/hello:hello_test
field wrapper_has_instrumented_files=True
field wrapper_has_quality_sources=True"""

def dx_wrapper_conformance_tests(name, subjects):
    starlark_test(
        name = name,
        mode = "analysis",
        subjects = subjects,
        expected_observations = EXPECTED_OBSERVATIONS,
    )
