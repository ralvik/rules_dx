package rust

import (
	"strings"
	"testing"

	bzl "github.com/bazelbuild/buildtools/build"
)

func TestParseCargoManifest(t *testing.T) {
	manifest, err := parseCargoManifest("Cargo.toml", []byte(`
[package]
name = "demo-app"
edition = "2024"

[dependencies]
serde-json = "1"
renamed = { package = "actual-crate", version = "1" }

[dev-dependencies]
tempfile = "3"

[lib]
name = "demo_core"
path = "src/core.rs"

[[bin]]
name = "demo"
path = "cmd/demo.rs"

[[test]]
name = "integration-fast"
path = "spec/fast.rs"
harness = false
`))
	if err != nil {
		t.Fatal(err)
	}
	if manifest.packageName != "demo-app" || manifest.edition != "2024" || len(manifest.targets) != 3 {
		t.Fatalf("manifest = %+v", manifest)
	}
	if target := manifest.targets[2]; target.name != "integration_fast" || target.path != "spec/fast.rs" || target.harness {
		t.Errorf("test target = %+v", target)
	}
	if !manifest.normalDeps["serde_json"].external || !manifest.normalDeps["renamed"].external || !manifest.devDeps["tempfile"].external {
		t.Errorf("dependency scopes = normal %v dev %v", manifest.normalDeps, manifest.devDeps)
	}
}

func TestParseCargoDefaults(t *testing.T) {
	manifest, err := parseCargoManifest("Cargo.toml", []byte("[package]\nname = \"demo\" # comment\n"))
	if err != nil {
		t.Fatal(err)
	}
	if len(manifest.targets) != 1 || manifest.targets[0].path != "src/lib.rs" || manifest.edition != "2021" {
		t.Errorf("defaults = %+v", manifest)
	}
}

func TestParseCargoFailsClosed(t *testing.T) {
	cases := []string{
		"[package]\nedition = \"2021\"\n",
		"[package]\nname = \"demo\"\n[[test]]\nname = \"x\"\nrequired-features = [\"slow\"]\n",
		"[package]\nname = \"demo\"\n[[bin]]\nname = \"same\"\n[[bin]]\nname = \"same\"\n",
		"[package]\nname = demo\n",
		"[package]\nname = \"demo\"\n[[test]]\nname = \"x\"\nharness = maybe\n",
		"[package]\nname = \"demo\"\n[lib]\nname = 1\n",
		"[package]\nname = \"demo\"\nnot-an-assignment\n",
	}
	for _, source := range cases {
		if _, err := parseCargoManifest("Cargo.toml", []byte(source)); err == nil {
			t.Errorf("expected failure for %q", source)
		} else if !strings.Contains(err.Error(), "Cargo") && !strings.Contains(err.Error(), "package") {
			t.Errorf("missing context: %v", err)
		}
	}
}

func TestCargoCommentAndStringErrors(t *testing.T) {
	if got := stripTomlComment(`name = "a#b" # trailing`); strings.TrimSpace(got) != `name = "a#b"` {
		t.Errorf("comment stripping = %q", got)
	}
	if _, err := tomlString(`"unterminated`); err == nil {
		t.Error("unterminated string accepted")
	}
	if _, err := tomlString(`"bad\q"`); err == nil {
		t.Error("invalid escape accepted")
	}
	if got := stripTomlComment(`name = "a\\\"#b" # trailing`); !strings.Contains(got, "#b") {
		t.Errorf("escaped quote ended string: %q", got)
	}
}

func TestCargoImplicitTargets(t *testing.T) {
	manifest, err := parseCargoManifest("Cargo.toml", []byte("[package]\nname = \"demo-app\"\n"))
	if err != nil {
		t.Fatal(err)
	}
	files := map[string]bool{
		"crates/app/src/lib.rs":          true,
		"crates/app/src/main.rs":         true,
		"crates/app/tests/smoke.rs":      true,
		"crates/app/tests/common/mod.rs": true,
	}
	if err := manifest.withImplicitTargets(files, "crates/app"); err != nil {
		t.Fatal(err)
	}
	if len(manifest.targets) != 3 || manifest.targets[1].kind != binaryKind || manifest.targets[2].name != "smoke_test" {
		t.Errorf("implicit targets = %+v", manifest.targets)
	}
}

func TestCargoExplicitTargetsSuppressImplicitDuplicates(t *testing.T) {
	manifest, err := parseCargoManifest("Cargo.toml", []byte("[package]\nname = \"demo\"\n[lib]\n[[bin]]\nname = \"tool\"\n[[test]]\nname = \"smoke\"\n"))
	if err != nil {
		t.Fatal(err)
	}
	files := map[string]bool{
		"pkg/src/lib.rs":     true,
		"pkg/src/main.rs":    true,
		"pkg/tests/smoke.rs": true,
	}
	if err := manifest.withImplicitTargets(files, "pkg"); err != nil {
		t.Fatal(err)
	}
	if len(manifest.targets) != 3 || manifest.targets[0].path != "src/lib.rs" || manifest.targets[1].path != "src/main.rs" || manifest.targets[2].path != "tests/smoke.rs" {
		t.Errorf("targets = %+v", manifest.targets)
	}
}

func TestCargoImplicitTargetFailures(t *testing.T) {
	manifest := &cargoManifest{targets: []cargoTarget{
		{kind: libraryKind, name: "same", path: "src/lib.rs"},
		{kind: binaryKind, name: "same", path: "src/main.rs"},
	}}
	if err := manifest.validateTargetClaims(); err == nil {
		t.Error("duplicate target name accepted")
	}
	manifest.targets[1].name = "other"
	manifest.targets[1].path = "src/lib.rs"
	if err := manifest.validateTargetClaims(); err == nil {
		t.Error("duplicate target path accepted")
	}
	manifest = &cargoManifest{packageName: "app"}
	if err := manifest.withImplicitTargets(map[string]bool{"pkg/tests/---.rs": true}, "pkg"); err == nil {
		t.Error("invalid implicit test name accepted")
	}
	manifest = &cargoManifest{packageName: "app"}
	if err := manifest.withImplicitTargets(map[string]bool{"pkg/src/lib.rs": true}, "pkg"); err != nil || len(manifest.targets) != 1 {
		t.Errorf("direct implicit library = %+v, %v", manifest.targets, err)
	}
}

func TestCargoLibBinDisambiguation(t *testing.T) {
	// Explicit same-name library and binary: the library takes the `_lib`
	// Bazel name while keeping the Rust crate name for dependents.
	manifest, err := parseCargoManifest("Cargo.toml", []byte("[package]\nname = \"demo\"\n[lib]\nname = \"demo\"\npath = \"src/lib.rs\"\n[[bin]]\nname = \"demo\"\npath = \"src/main.rs\"\n"))
	if err != nil {
		t.Fatal(err)
	}
	if len(manifest.targets) != 2 || manifest.targets[0].name != "demo_lib" || manifest.targets[0].crate() != "demo" || manifest.targets[1].name != "demo" {
		t.Errorf("explicit lib/bin targets = %+v", manifest.targets)
	}
	// Implicit same-name pair disambiguates the same way.
	implicit := &cargoManifest{packageName: "app"}
	files := map[string]bool{"pkg/src/lib.rs": true, "pkg/src/main.rs": true}
	if err := implicit.withImplicitTargets(files, "pkg"); err != nil {
		t.Fatal(err)
	}
	if len(implicit.targets) != 2 || implicit.targets[0].name != "app_lib" || implicit.targets[0].crate() != "app" || implicit.targets[1].name != "app" {
		t.Errorf("implicit lib/bin targets = %+v", implicit.targets)
	}
	// A lone library keeps its bare name and crate.
	solo, err := parseCargoManifest("Cargo.toml", []byte("[package]\nname = \"solo\"\n[lib]\nname = \"solo\"\npath = \"src/lib.rs\"\n"))
	if err != nil {
		t.Fatal(err)
	}
	if len(solo.targets) != 1 || solo.targets[0].name != "solo" || solo.targets[0].crate() != "solo" {
		t.Errorf("solo library targets = %+v", solo.targets)
	}
}

func TestSiblingLibName(t *testing.T) {
	withLib := &cargoManifest{targets: []cargoTarget{{kind: libraryKind, name: "demo_lib"}, {kind: binaryKind, name: "demo"}}}
	if got := siblingLibName(withLib, cargoTarget{kind: binaryKind, name: "demo"}); got != "demo_lib" {
		t.Errorf("siblingLibName with library = %q, want %q", got, "demo_lib")
	}
	binOnly := &cargoManifest{targets: []cargoTarget{{kind: binaryKind, name: "tool"}}}
	if got := siblingLibName(binOnly, cargoTarget{kind: binaryKind, name: "tool"}); got != "" {
		t.Errorf("siblingLibName without library = %q, want empty", got)
	}
}

func TestCargoCallNamesDefensive(t *testing.T) {
	// Non-call bases and foreign calls yield no names instead of panicking.
	if got := cargoCallNames(&bzl.StringExpr{Value: "deps"}); got != nil {
		t.Errorf("cargoCallNames(string) = %v, want nil", got)
	}
	if got := cargoCallNames(&bzl.CallExpr{X: &bzl.Ident{Name: "deps"}, List: []bzl.Expr{strListExpr("a")}}); got != nil {
		t.Errorf("cargoCallNames(deps(...)) = %v, want nil", got)
	}
	if got := cargoCallNames(&bzl.CallExpr{X: &bzl.Ident{Name: "crate_deps"}}); got != nil {
		t.Errorf("cargoCallNames(crate_deps()) = %v, want nil", got)
	}
	if got := cargoCallNames(&bzl.CallExpr{X: &bzl.Ident{Name: "crate_deps"}, List: []bzl.Expr{&bzl.StringExpr{Value: "x"}}}); got != nil {
		t.Errorf("cargoCallNames(crate_deps(string)) = %v, want nil", got)
	}
	if got := cargoCallNames(crateDepsFileExpr([]string{"serde_json"}, "pkg")); len(got) != 1 || got[0] != "serde_json" {
		t.Errorf("cargoCallNames(crate_deps([...])) = %v, want [serde_json]", got)
	}
}
