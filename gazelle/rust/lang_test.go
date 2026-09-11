package rust

import (
	"context"
	"flag"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/bazel-contrib/bazel-gazelle/v2/label"
	"github.com/bazel-contrib/bazel-gazelle/v2/rule"
	"github.com/bazelbuild/bazel-gazelle/config"
	"github.com/bazelbuild/bazel-gazelle/language"
	"github.com/bazelbuild/bazel-gazelle/resolve"
)

func writeFixture(t *testing.T, root, name, content string) {
	t.Helper()
	file := filepath.Join(root, filepath.FromSlash(name))
	if err := os.MkdirAll(filepath.Dir(file), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(file, []byte(content), 0o644); err != nil {
		t.Fatal(err)
	}
}

func generateFixture(t *testing.T, files map[string]string) language.GenerateResult {
	t.Helper()
	root := t.TempDir()
	for name, content := range files {
		writeFixture(t, root, name, content)
	}
	dir := filepath.Join(root, "crates", "demo")
	return NewLanguage().GenerateRules(language.GenerateArgs{
		Config: &config.Config{RepoRoot: root},
		Dir:    dir,
		Rel:    "crates/demo",
	})
}

func TestGenerateSourceOnlyCrate(t *testing.T) {
	result := generateFixture(t, map[string]string{
		"crates/demo/src/lib.rs":          "mod model;\nuse local_dep::Thing;\n#[cfg(test)] mod tests;\n",
		"crates/demo/src/model.rs":        "use std::fmt::Debug;\n",
		"crates/demo/src/tests.rs":        "use test_dep::Helper;\n",
		"crates/demo/tests/common.rs":     "use demo::Thing;\n",
		"crates/demo/tests/helper/mod.rs": "use ignored_nested_root::Nope;\n",
	})
	if len(result.Gen) != 3 || len(result.Imports) != 3 {
		t.Fatalf("generated %d rules and %d import sets, want 3 each", len(result.Gen), len(result.Imports))
	}
	lib := result.Gen[0]
	if lib.Kind() != libraryKind || lib.Name() != "demo" {
		t.Fatalf("library = %s(%s)", lib.Kind(), lib.Name())
	}
	if got := strings.Join(lib.AttrStrings("srcs"), ","); got != "src/lib.rs,src/model.rs,src/tests.rs" {
		t.Errorf("library srcs = %q", got)
	}
	if lib.AttrString("crate_root") != "src/lib.rs" || lib.AttrString("crate_name") != "demo" {
		t.Errorf("library attributes: root=%q crate=%q", lib.AttrString("crate_root"), lib.AttrString("crate_name"))
	}
	unit := result.Gen[1]
	if unit.Kind() != testKind || unit.Name() != "demo_test" || unit.AttrString("crate") != ":demo" || len(unit.AttrStrings("srcs")) != 0 {
		t.Errorf("unit test = %s(%s), crate=%q srcs=%v", unit.Kind(), unit.Name(), unit.AttrString("crate"), unit.AttrStrings("srcs"))
	}
	integration := result.Gen[2]
	if integration.Kind() != testKind || integration.Name() != "common_test" {
		t.Errorf("integration test = %s(%s)", integration.Kind(), integration.Name())
	}
	libImports := result.Imports[0].(targetImports)
	if strings.Join(libImports.production, ",") != "local_dep" || strings.Join(libImports.test, ",") != "test_dep" {
		t.Errorf("library imports = %+v", libImports)
	}
	unitImports := result.Imports[1].(targetImports)
	if len(unitImports.production) != 0 || strings.Join(unitImports.test, ",") != "test_dep" {
		t.Errorf("unit imports = %+v", unitImports)
	}
}

func TestGenerateSourceOnlyBinaryUnitTest(t *testing.T) {
	result := generateFixture(t, map[string]string{
		"crates/demo/src/main.rs": "#[test]\nfn works() {}\n",
	})
	if len(result.Gen) != 2 || result.Gen[0].Kind() != binaryKind || result.Gen[1].AttrString("crate") != ":demo" {
		t.Errorf("binary unit-test generation = %+v", result.Gen)
	}
}

func TestGenerateSourceOnlyFailures(t *testing.T) {
	cases := []struct {
		files map[string]string
		want  string
	}{
		{map[string]string{"crates/demo/src/lib.rs": "mod missing;\n"}, "orphan module"},
		{map[string]string{
			"crates/demo/src/lib.rs":    "mod shared;\n",
			"crates/demo/src/main.rs":   "#[path = \"shared.rs\"] mod shared;\n",
			"crates/demo/src/shared.rs": "",
		}, "owned by both"},
		{map[string]string{
			"crates/demo/src/lib.rs":    "#[test]\nfn works() {}\n",
			"crates/demo/tests/demo.rs": "",
		}, "normalized name"},
	}
	for _, tc := range cases {
		root := t.TempDir()
		for name, content := range tc.files {
			writeFixture(t, root, name, content)
		}
		l := &rustLang{}
		l.GenerateRules(language.GenerateArgs{Config: &config.Config{RepoRoot: root}, Dir: filepath.Join(root, "crates/demo"), Rel: "crates/demo"})
		if len(l.errors) != 1 || !strings.Contains(l.errors[0], tc.want) {
			t.Errorf("files %v errors = %v, want %q", tc.files, l.errors, tc.want)
		}
	}
}

func TestGenerateStaleRules(t *testing.T) {
	f := rule.EmptyFile("BUILD.bazel", "crates/demo")
	f.Rules = append(f.Rules,
		rule.NewRule(libraryKind, "old"),
		rule.NewRule("filegroup", "keep"),
	)
	result := NewLanguage().GenerateRules(language.GenerateArgs{
		Config: &config.Config{RepoRoot: t.TempDir()},
		Dir:    t.TempDir(),
		Rel:    "crates/demo",
		File:   f,
	})
	if len(result.Empty) != 1 || result.Empty[0].Kind() != libraryKind || result.Empty[0].Name() != "old" {
		t.Fatalf("empty rules = %v", result.Empty)
	}
}

func TestImportsIndexesOnlyLibraries(t *testing.T) {
	lang := NewLanguage()
	lib := rule.NewRule(libraryKind, "demo")
	lib.SetAttr("crate_name", "cargo_name")
	imports := lang.Imports(&config.Config{}, lib, nil)
	if len(imports) != 1 || imports[0].Lang != languageName || imports[0].Imp != "cargo_name" {
		t.Errorf("imports = %+v", imports)
	}
	if got := lang.Imports(&config.Config{}, rule.NewRule(testKind, "demo_test"), nil); got != nil {
		t.Errorf("test imports = %+v, want nil", got)
	}
	unnamed := rule.NewRule(libraryKind, "fallback")
	if got := lang.Imports(&config.Config{}, unnamed, nil); len(got) != 1 || got[0].Imp != "fallback" {
		t.Errorf("fallback imports = %+v", got)
	}
}

func TestErrorsAbortBeforeEmission(t *testing.T) {
	l := &rustLang{}
	l.Before(context.Background())
	l.fail("second")
	l.fail("first")
	defer func() {
		got := recover()
		if got == nil {
			t.Fatal("AfterResolvingDeps did not abort")
		}
		message := got.(string)
		if !strings.Contains(message, "first\nsecond") {
			t.Errorf("errors were not deterministic: %s", message)
		}
	}()
	l.AfterResolvingDeps(context.Background())
}

func TestIgnoreDirectiveInheritanceAndStaleCheck(t *testing.T) {
	l := &rustLang{}
	l.Before(context.Background())
	root := config.New()
	file, err := rule.LoadData("BUILD.bazel", "", []byte("# gazelle:dx_ignore_import rust missing_crate\n"))
	if err != nil {
		t.Fatal(err)
	}
	l.Configure(root, "", file)
	child := root.Clone()
	l.Configure(child, "child", nil)
	ignore := matchingIgnore(child, "missing_crate")
	if ignore == nil || ignore.path != "" {
		t.Fatalf("inherited ignore = %+v", ignore)
	}
	ignore.used = true
	l.AfterResolvingDeps(context.Background())
}

func TestStaleIgnoreFails(t *testing.T) {
	l := &rustLang{}
	l.Before(context.Background())
	cfg := config.New()
	file, err := rule.LoadData("BUILD.bazel", "pkg", []byte("# gazelle:dx_ignore_import rust stale\n"))
	if err != nil {
		t.Fatal(err)
	}
	l.Configure(cfg, "pkg", file)
	defer func() {
		if got := recover(); got == nil || !strings.Contains(got.(string), "stale") {
			t.Errorf("stale ignore result = %v", got)
		}
	}()
	l.AfterResolvingDeps(context.Background())
}

func TestMalformedIgnoreFails(t *testing.T) {
	l := &rustLang{}
	l.Before(context.Background())
	cfg := config.New()
	file, err := rule.LoadData("BUILD.bazel", "pkg", []byte("# gazelle:dx_ignore_import rust\n"))
	if err != nil {
		t.Fatal(err)
	}
	l.Configure(cfg, "pkg", file)
	defer func() {
		if got := recover(); got == nil || !strings.Contains(got.(string), "malformed") {
			t.Errorf("malformed ignore result = %v", got)
		}
	}()
	l.AfterResolvingDeps(context.Background())
}

func TestLanguageMetadata(t *testing.T) {
	l := &rustLang{}
	if l.Name() != "rust" || len(l.Kinds()) != 3 || l.CheckFlags(flag.NewFlagSet("test", flag.ContinueOnError), config.New()) != nil {
		t.Fatalf("invalid language metadata")
	}
	l.RegisterFlags(flag.NewFlagSet("test", flag.ContinueOnError), "update", config.New())
	if strings.Join(l.KnownDirectives(), ",") != "dx_ignore_import" || l.Embeds(nil, label.NoLabel) != nil {
		t.Fatal("invalid directives or embeds")
	}
	loads := l.ApparentLoads(func(name string) string {
		if name == "rules_dx" {
			return "renamed_dx"
		}
		if name == "crates" {
			return "renamed_crates"
		}
		return ""
	})
	if loads[0].Name != "@renamed_dx//rust/rules:defs.bzl" || loads[1].Name != "@renamed_crates//:crates.bzl" {
		t.Errorf("apparent loads = %+v", loads)
	}
	if defaults := l.Loads(); defaults[0].Name != "@rules_dx//rust/rules:defs.bzl" {
		t.Errorf("default loads = %+v", defaults)
	}
	defaultApparent := l.ApparentLoads(func(string) string { return "" })
	if defaultApparent[0].Name != "@rules_dx//rust/rules:defs.bzl" || defaultApparent[1].Name != "@crates//:crates.bzl" {
		t.Errorf("default apparent loads = %+v", defaultApparent)
	}
	l.DoneGeneratingRules()
}

func TestConfigureDirectiveForms(t *testing.T) {
	l := &rustLang{}
	l.Before(context.Background())
	cfg := config.New()
	file, err := rule.LoadData("BUILD.bazel", "pkg", []byte("# gazelle:other ignored\n# gazelle:dx_ignore_import rust rust external\n# gazelle:dx_ignore_import python foreign\n"))
	if err != nil {
		t.Fatal(err)
	}
	l.Configure(cfg, "pkg", file)
	if len(l.ignores) != 1 || l.ignores[0].value != "external" {
		t.Errorf("ignores = %+v", l.ignores)
	}
	l.ignores[0].used = true
	l.AfterResolvingDeps(context.Background())
	if matchingIgnore(config.New(), "none") != nil || matchingIgnore(cfg, "none") != nil {
		t.Error("nonmatching ignore found")
	}
}

func TestGenerateCargoAPIAndFailures(t *testing.T) {
	root := t.TempDir()
	writeFixture(t, root, "crates/app/Cargo.toml", "[package]\nname = \"app\"\nedition = \"2021\"\n[dependencies]\nserde_json = \"1\"\n[dev-dependencies]\ntempfile = \"3\"\n[lib]\npath = \"src/lib.rs\"\n")
	writeFixture(t, root, "crates/app/src/lib.rs", "use serde_json::Value;\n#[cfg(test)] use tempfile::tempdir;\n")
	l := &rustLang{}
	result := l.GenerateRules(language.GenerateArgs{
		Config:       &config.Config{RepoRoot: root},
		Dir:          filepath.Join(root, "crates/app"),
		Rel:          "crates/app",
		RegularFiles: []string{"Cargo.toml"},
	})
	if len(l.errors) != 0 || len(result.Gen) != 2 {
		t.Fatalf("cargo generation errors=%v result=%+v", l.errors, result)
	}
	if result.Gen[0].AttrString("edition") != "2021" || result.Gen[1].AttrString("crate") != ":app" {
		t.Errorf("cargo rules = %+v", result.Gen)
	}

	badRoot := t.TempDir()
	writeFixture(t, badRoot, "Cargo.toml", "[package]\nname = \"bad\"\n[[test]]\nname = \"missing\"\npath = \"tests/missing.rs\"\n")
	bad := &rustLang{}
	bad.GenerateRules(language.GenerateArgs{Config: &config.Config{RepoRoot: badRoot}, Dir: badRoot, RegularFiles: []string{"Cargo.toml"}})
	if len(bad.errors) != 1 || !strings.Contains(bad.errors[0], "does not exist") {
		t.Errorf("missing target errors = %v", bad.errors)
	}

	malformed := t.TempDir()
	writeFixture(t, malformed, "Cargo.toml", "[package]\nname = nope\n")
	broken := &rustLang{}
	broken.GenerateRules(language.GenerateArgs{Config: &config.Config{RepoRoot: malformed}, Dir: malformed, RegularFiles: []string{"Cargo.toml"}})
	if len(broken.errors) != 1 || !strings.Contains(broken.errors[0], "quoted string") {
		t.Errorf("manifest errors = %v", broken.errors)
	}

	missingManifest := &rustLang{}
	missingManifest.generateCargo(language.GenerateArgs{Config: &config.Config{RepoRoot: root}, Dir: filepath.Join(root, "missing"), Rel: "missing"}, nil)
	if len(missingManifest.errors) != 1 || !strings.Contains(missingManifest.errors[0], "Cargo.toml") {
		t.Errorf("missing manifest errors = %v", missingManifest.errors)
	}

	missingRoot := t.TempDir()
	writeFixture(t, missingRoot, "Cargo.toml", "[package]\nname = \"app\"\n[[test]]\nname = \"missing\"\npath = \"tests/missing.rs\"\n")
	missing := &rustLang{}
	missing.GenerateRules(language.GenerateArgs{Config: &config.Config{RepoRoot: missingRoot}, Dir: missingRoot, RegularFiles: []string{"Cargo.toml"}})
	if len(missing.errors) != 1 || !strings.Contains(missing.errors[0], "does not exist") {
		t.Errorf("missing root errors = %v", missing.errors)
	}
}

func TestGenerateCargoOwnershipAndImportFailures(t *testing.T) {
	cases := []struct {
		manifest string
		files    map[string]string
		want     string
	}{
		{
			manifest: "[package]\nname = \"app\"\n[lib]\npath = \"src/lib.rs\"\n[[bin]]\nname = \"tool\"\npath = \"src/lib.rs\"\n",
			files:    map[string]string{"src/lib.rs": ""},
			want:     "one source may have only one owner",
		},
		{
			manifest: "[package]\nname = \"app\"\n[lib]\npath = \"src/lib.rs\"\n",
			files:    map[string]string{"src/lib.rs": "use undeclared::Thing;\n"},
			want:     "unresolved production import",
		},
		{
			manifest: "[package]\nname = \"app\"\n[lib]\npath = \"src/lib.rs\"\n",
			files:    map[string]string{"src/lib.rs": "#[cfg(test)] use undeclared::Thing;\n"},
			want:     "unresolved test import",
		},
		{
			manifest: "[package]\nname = \"app\"\n[lib]\npath = \"src/lib.rs\"\n",
			files:    map[string]string{"src/lib.rs": "mod missing;\n"},
			want:     "orphan module",
		},
		{
			manifest: "[package]\nname = \"app\"\n[lib]\nname = \"---\"\n",
			files:    map[string]string{"src/lib.rs": ""},
			want:     "empty target name",
		},
		{
			manifest: "[package]\nname = \"app\"\n[lib]\npath = \"src/lib.rs\"\n[[test]]\nname = \"other\"\npath = \"tests/other.rs\"\n",
			files: map[string]string{
				"src/lib.rs":     "#[path = \"../shared.rs\"] mod shared;\n",
				"tests/other.rs": "#[path = \"../shared.rs\"] mod shared;\n",
				"shared.rs":      "",
			},
			want: "owned by both",
		},
	}
	for _, tc := range cases {
		root := t.TempDir()
		writeFixture(t, root, "Cargo.toml", tc.manifest)
		for name, content := range tc.files {
			writeFixture(t, root, name, content)
		}
		l := &rustLang{}
		l.GenerateRules(language.GenerateArgs{Config: &config.Config{RepoRoot: root}, Dir: root, RegularFiles: []string{"Cargo.toml"}})
		if len(l.errors) == 0 || !strings.Contains(strings.Join(l.errors, "\n"), tc.want) {
			t.Errorf("errors = %v, want %q", l.errors, tc.want)
		}
	}
}

func TestGenerateCargoCustomHarness(t *testing.T) {
	root := t.TempDir()
	writeFixture(t, root, "Cargo.toml", "[package]\nname = \"app\"\n[[test]]\nname = \"custom\"\npath = \"tests/custom.rs\"\nharness = false\n")
	writeFixture(t, root, "tests/custom.rs", "fn main() {}\n")
	l := &rustLang{}
	result := l.GenerateRules(language.GenerateArgs{Config: &config.Config{RepoRoot: root}, Dir: root, RegularFiles: []string{"Cargo.toml"}})
	if len(l.errors) != 0 || len(result.Gen) != 1 || result.Gen[0].Attr("use_libtest_harness") == nil {
		t.Errorf("custom harness generation errors=%v result=%+v", l.errors, result.Gen)
	}
}

func TestSourceFilesBoundaries(t *testing.T) {
	root := t.TempDir()
	writeFixture(t, root, "src/lib.rs", "")
	writeFixture(t, root, "src/.hidden/no.rs", "")
	writeFixture(t, root, "src/nested/BUILD.bazel", "")
	writeFixture(t, root, "src/nested/no.rs", "")
	writeFixture(t, root, "src/legacy/BUILD", "")
	writeFixture(t, root, "src/legacy/no.rs", "")
	writeFixture(t, root, "tests/smoke.rs", "")
	files, err := sourceFiles(root, "pkg")
	if err != nil {
		t.Fatal(err)
	}
	if got := strings.Join(files, ","); got != "pkg/src/lib.rs,pkg/tests/smoke.rs" {
		t.Errorf("source files = %q", got)
	}
}

func TestGenerateEmptyNoRootAndSourceError(t *testing.T) {
	l := &rustLang{}
	root := t.TempDir()
	if result := l.GenerateRules(language.GenerateArgs{Config: &config.Config{RepoRoot: root}, Dir: root}); len(result.Gen) != 0 {
		t.Errorf("empty generation = %+v", result)
	}
	writeFixture(t, root, "src/helper.rs", "")
	if result := l.GenerateRules(language.GenerateArgs{Config: &config.Config{RepoRoot: root}, Dir: root}); len(result.Gen) != 0 {
		t.Errorf("no-root generation = %+v", result)
	}
	missing := filepath.Join(root, "gone")
	if _, err := sourceFiles(missing, "gone"); err == nil {
		t.Error("missing source directory accepted")
	}
	failed := &rustLang{}
	failed.GenerateRules(language.GenerateArgs{Config: &config.Config{RepoRoot: root}, Dir: missing, Rel: "gone"})
	if len(failed.errors) != 1 {
		t.Errorf("GenerateRules source error = %v", failed.errors)
	}
	notDir := filepath.Join(root, "plain-file")
	writeFixture(t, root, "plain-file", "content")
	if _, err := sourceFiles(notDir, "file"); err != nil {
		t.Errorf("single-file scan failed: %v", err)
	}
	bad := t.TempDir()
	writeFixture(t, bad, "src/lib.rs", "mod missing;\n")
	broken := &rustLang{}
	broken.GenerateRules(language.GenerateArgs{Config: &config.Config{RepoRoot: bad}, Dir: bad})
	if len(broken.errors) != 1 || !strings.Contains(broken.errors[0], "orphan module") {
		t.Errorf("source parse errors = %v", broken.errors)
	}
}

func TestGenerateCargoImplicitFailure(t *testing.T) {
	root := t.TempDir()
	writeFixture(t, root, "Cargo.toml", "[package]\nname = \"app\"\n")
	writeFixture(t, root, "tests/---.rs", "")
	l := &rustLang{}
	l.GenerateRules(language.GenerateArgs{Config: &config.Config{RepoRoot: root}, Dir: root, RegularFiles: []string{"Cargo.toml"}})
	if len(l.errors) != 1 || !strings.Contains(l.errors[0], "empty target name") {
		t.Errorf("implicit target errors = %v", l.errors)
	}
}

func TestGenerateExistingNameCollisions(t *testing.T) {
	root := t.TempDir()
	writeFixture(t, root, "crates/demo/src/lib.rs", "")
	f := rule.EmptyFile("BUILD.bazel", "crates/demo")
	f.Rules = append(f.Rules, rule.NewRule("filegroup", "demo"))
	l := &rustLang{}
	l.GenerateRules(language.GenerateArgs{Config: &config.Config{RepoRoot: root}, Dir: filepath.Join(root, "crates/demo"), Rel: "crates/demo", File: f})
	if len(l.errors) != 1 || !strings.Contains(l.errors[0], "existing filegroup") {
		t.Errorf("source collision errors = %v", l.errors)
	}

	cargoRoot := t.TempDir()
	writeFixture(t, cargoRoot, "Cargo.toml", "[package]\nname = \"app\"\n")
	writeFixture(t, cargoRoot, "src/lib.rs", "")
	cargoFile := rule.EmptyFile("BUILD.bazel", "")
	cargoFile.Rules = append(cargoFile.Rules, rule.NewRule("filegroup", "app"))
	cargo := &rustLang{}
	cargo.GenerateRules(language.GenerateArgs{Config: &config.Config{RepoRoot: cargoRoot}, Dir: cargoRoot, RegularFiles: []string{"Cargo.toml"}, File: cargoFile})
	if len(cargo.errors) != 1 || !strings.Contains(cargo.errors[0], "existing filegroup") {
		t.Errorf("Cargo collision errors = %v", cargo.errors)
	}
}

func TestCargoImportValidationAndExpressions(t *testing.T) {
	manifest := &cargoManifest{
		packageName: "app-name",
		normalDeps:  map[string]cargoDependency{"normal": {external: true}, "local": {}},
		devDeps:     map[string]cargoDependency{"dev": {external: true}, "local_dev": {}},
	}
	imports := targetImports{production: []string{"normal", "local"}, test: []string{"dev", "local_dev"}}
	if err := validateCargoImports(manifest, libraryKind, imports); err != nil {
		t.Fatal(err)
	}
	if err := validateCargoImports(manifest, libraryKind, targetImports{production: []string{"unknown"}}); err == nil {
		t.Error("unknown production import accepted")
	}
	if err := validateCargoImports(manifest, testKind, targetImports{production: []string{"dev"}}); err != nil {
		t.Errorf("test dev import rejected: %v", err)
	}
	if err := validateCargoImports(manifest, testKind, targetImports{test: []string{"unknown"}}); err == nil {
		t.Error("unknown test import accepted")
	}
	local := localCargoImports(manifest, imports, true)
	if strings.Join(local.production, ",") != "local" || strings.Join(local.test, ",") != "local_dev" {
		t.Errorf("local imports = %+v", local)
	}
	r := rule.NewRule(libraryKind, "app")
	setCargoAttrs(r, "pkg/app", manifest, imports, true)
	if r.Attr("deps") == nil || r.Attr("aliases") == nil {
		t.Errorf("Cargo expressions missing")
	}
	call := cargoCall("aliases", "pkg/app", true)
	if call.Merge(nil) == nil || (crateDepsCall{names: []string{"normal"}, packageName: "pkg/app"}).Merge(nil) == nil {
		t.Error("generated expressions did not merge")
	}
	withoutDev := localCargoImports(manifest, imports, false)
	if len(withoutDev.test) != 0 {
		t.Errorf("production local imports include dev: %+v", withoutDev)
	}
	empty := rule.NewRule(libraryKind, "empty")
	setCargoAttrs(empty, "pkg/app", manifest, targetImports{}, false)
	if empty.Attr("deps") != nil || empty.Attr("aliases") == nil {
		t.Errorf("empty Cargo attrs = deps:%v aliases:%v", empty.Attr("deps"), empty.Attr("aliases"))
	}
}

func TestLangCoverageClosure(t *testing.T) {
	// A test-scoped import of a local (non-external) normal dependency is
	// retained when dev imports are included.
	manifest := &cargoManifest{
		packageName: "app",
		normalDeps:  map[string]cargoDependency{"shared": {}},
		devDeps:     map[string]cargoDependency{},
	}
	local := localCargoImports(manifest, targetImports{test: []string{"shared"}}, true)
	if strings.Join(local.test, ",") != "shared" {
		t.Errorf("local test imports = %+v, want [shared]", local)
	}
	// Extern-crate declarations feed the import sets like use paths.
	tree := map[string]*FileFacts{"src/lib.rs": {Externs: []ExternCrate{{Name: "serde"}}}}
	imports := importsFor(tree)
	if strings.Join(imports.production, ",") != "serde" {
		t.Errorf("extern imports = %+v, want [serde]", imports)
	}
	// Test-kind rules resolve both production and test imports.
	l := &rustLang{}
	index := resolverIndex(l,
		struct{ pkg, name string }{"lib/prod", "prod"},
		struct{ pkg, name string }{"lib/helpers", "helpers"},
	)
	cfg := resolverConfig(t, nil)
	cfg.Exts[languageName] = &rustConfig{}
	r := rule.NewRule(testKind, "app_test")
	l.Resolve(cfg, index, nil, r, targetImports{production: []string{"prod"}, test: []string{"helpers"}}, label.New("", "app", "app_test"))
	if got := strings.Join(r.AttrStrings("deps"), ","); got != "//lib/helpers,//lib/prod" {
		t.Errorf("test-kind deps = %q", got)
	}
	if len(l.errors) != 0 {
		t.Errorf("test-kind errors = %v", l.errors)
	}
}

func TestOwnershipAndExistingClaimsFail(t *testing.T) {
	tree := map[string]*FileFacts{"src/shared.rs": {}}
	owners := map[string]string{}
	if err := claimSources(owners, "first", tree); err != nil {
		t.Fatal(err)
	}
	if err := claimSources(owners, "second", tree); err == nil || !strings.Contains(err.Error(), "both first and second") {
		t.Errorf("duplicate ownership = %v", err)
	}
	f := rule.EmptyFile("BUILD.bazel", "pkg")
	f.Rules = append(f.Rules, rule.NewRule("filegroup", "collision"))
	generated := []*rule.Rule{rule.NewRule(libraryKind, "collision")}
	if err := checkExistingClaims(f, nil, generated); err == nil || !strings.Contains(err.Error(), "existing filegroup") {
		t.Errorf("existing collision = %v", err)
	}
	if err := checkExistingClaims(nil, []*rule.Rule{rule.NewRule("py_library", "collision")}, generated); err == nil {
		t.Error("other-language collision accepted")
	}
	if err := checkExistingClaims(nil, nil, generated); err != nil {
		t.Errorf("unique generated target rejected: %v", err)
	}
}

func resolverConfig(t *testing.T, directives []rule.Directive) *config.Config {
	t.Helper()
	cfg := config.New()
	resolver := &resolve.Configurer{}
	resolver.RegisterFlags(nil, "update", cfg)
	if err := resolver.CheckFlags(nil, cfg); err != nil {
		t.Fatal(err)
	}
	resolver.Configure(cfg, "app", &rule.File{Directives: directives})
	return cfg
}

func resolverIndex(lang *rustLang, entries ...struct {
	pkg  string
	name string
}) *resolve.RuleIndex {
	index := resolve.NewRuleIndex(func(r *rule.Rule, _ string) resolve.Resolver {
		if _, ok := rustKinds[r.Kind()]; ok {
			return lang
		}
		return nil
	})
	for _, entry := range entries {
		r := rule.NewRule(libraryKind, entry.name)
		r.SetAttr("crate_name", entry.name)
		index.AddRule(config.New(), r, rule.EmptyFile(filepath.Join(entry.pkg, "BUILD.bazel"), entry.pkg))
	}
	index.Finish()
	return index
}

func TestResolveBranches(t *testing.T) {
	l := &rustLang{}
	index := resolverIndex(l,
		struct{ pkg, name string }{"lib/b", "b"},
		struct{ pkg, name string }{"lib/a", "a"},
	)
	cfg := resolverConfig(t, nil)
	cfg.Exts[languageName] = &rustConfig{}
	r := rule.NewRule(binaryKind, "app")
	l.Resolve(cfg, index, nil, r, targetImports{production: []string{"b", "a"}}, label.New("", "app", "app"))
	if got := strings.Join(r.AttrStrings("deps"), ","); got != "//lib/a,//lib/b" {
		t.Errorf("resolved deps = %q", got)
	}

	self := rule.NewRule(libraryKind, "a")
	l.Resolve(cfg, index, nil, self, targetImports{production: []string{"a"}}, label.New("", "lib/a", "a"))
	if self.Attr("deps") != nil {
		t.Errorf("self dependency was emitted: %v", self.AttrStrings("deps"))
	}

	ignored := &ignoreEntry{value: "missing"}
	cfg.Exts[languageName] = &rustConfig{ignores: []*ignoreEntry{ignored}}
	l.Resolve(cfg, index, nil, rule.NewRule(binaryKind, "ignored"), targetImports{production: []string{"missing"}}, label.New("", "app", "ignored"))
	if !ignored.used {
		t.Error("exact ignore was not consumed")
	}

	l.Resolve(cfg, index, nil, rule.NewRule(binaryKind, "unknown"), targetImports{production: []string{"unknown"}}, label.New("", "app", "unknown"))
	if len(l.errors) != 1 || !strings.Contains(l.errors[0], "unresolved import") {
		t.Errorf("unresolved errors = %v", l.errors)
	}
	// Invalid opaque values are ignored by the legacy resolver contract.
	l.Resolve(cfg, index, nil, r, nil, label.New("", "app", "app"))
}

func TestResolveOverrideAndConflict(t *testing.T) {
	l := &rustLang{}
	cfg := resolverConfig(t, []rule.Directive{{Key: "resolve", Value: "rust rust mapped //mapped:dep"}})
	cfg.Exts[languageName] = &rustConfig{}
	r := rule.NewRule(binaryKind, "app")
	l.Resolve(cfg, resolverIndex(l), nil, r, targetImports{production: []string{"mapped"}}, label.New("", "app", "app"))
	if got := strings.Join(r.AttrStrings("deps"), ","); got != "//mapped:dep" {
		t.Errorf("override deps = %q", got)
	}
	ignore := &ignoreEntry{value: "mapped"}
	cfg.Exts[languageName] = &rustConfig{ignores: []*ignoreEntry{ignore}}
	l.Resolve(cfg, resolverIndex(l), nil, rule.NewRule(binaryKind, "conflict"), targetImports{production: []string{"mapped"}}, label.New("", "app", "conflict"))
	if !ignore.used || len(l.errors) != 1 || !strings.Contains(l.errors[0], "both") {
		t.Errorf("mapping-ignore conflict = used:%v errors:%v", ignore.used, l.errors)
	}
}

func TestResolveAmbiguous(t *testing.T) {
	l := &rustLang{}
	cfg := resolverConfig(t, nil)
	cfg.Exts[languageName] = &rustConfig{}
	index := resolverIndex(l,
		struct{ pkg, name string }{"one", "same"},
		struct{ pkg, name string }{"two", "same"},
	)
	l.Resolve(cfg, index, nil, rule.NewRule(binaryKind, "app"), targetImports{production: []string{"same"}}, label.New("", "app", "app"))
	if len(l.errors) != 1 || !strings.Contains(l.errors[0], "ambiguous") || !strings.Contains(l.errors[0], "//one:same") || !strings.Contains(l.errors[0], "//two:same") {
		t.Errorf("ambiguity = %v", l.errors)
	}
}
