package rust

import (
	"os"
	"path"
	"path/filepath"
	"sort"
	"strings"
	"testing"

	"github.com/bazelbuild/bazel-gazelle/config"
	"github.com/bazelbuild/bazel-gazelle/language"
	"github.com/bazelbuild/bazel-gazelle/rule"
)

// runNativeGenerate mimics the Gazelle walk for one directory: parent
// configuration first so directives inherit, then GenerateRules with the
// direct-child regular files and the parsed BUILD file.
func runNativeGenerate(t *testing.T, rel string, files map[string]string, build string) (*rustLang, language.GenerateResult) {
	t.Helper()
	root := t.TempDir()
	for name, content := range files {
		writeFixture(t, root, name, content)
	}
	dir := root
	if rel != "" {
		dir = filepath.Join(root, filepath.FromSlash(rel))
		if err := os.MkdirAll(dir, 0o755); err != nil {
			t.Fatal(err)
		}
	}
	var regular []string
	for name := range files {
		parent := path.Dir(name)
		if parent == "." {
			parent = ""
		}
		if parent == rel {
			regular = append(regular, path.Base(name))
		}
	}
	sort.Strings(regular)
	cfg := config.New()
	cfg.RepoRoot = root
	l := &rustLang{}
	l.Configure(cfg, "", nil)
	var file *rule.File
	if build != "" {
		var err error
		file, err = rule.LoadData("BUILD.bazel", rel, []byte(build))
		if err != nil {
			t.Fatal(err)
		}
		if rel != "" {
			l.Configure(cfg, rel, file)
		} else {
			l.Configure(cfg, "", file)
		}
	} else if rel != "" {
		l.Configure(cfg, rel, nil)
	}
	result := l.GenerateRules(language.GenerateArgs{
		Config:       cfg,
		Dir:          dir,
		Rel:          rel,
		RegularFiles: regular,
		File:         file,
	})
	return l, result
}

func findGenerated(result language.GenerateResult, kind, name string) *rule.Rule {
	for _, r := range result.Gen {
		if r.Kind() == kind && r.Name() == name {
			return r
		}
	}
	return nil
}

func findEmpty(result language.GenerateResult, kind, name string) bool {
	for _, r := range result.Empty {
		if r.Kind() == kind && r.Name() == name {
			return true
		}
	}
	return false
}

func TestNativeConfigRecognition(t *testing.T) {
	_, result := runNativeGenerate(t, "site", map[string]string{
		"site/src/lib.rs":       "pub fn current() {}\n",
		"site/rustfmt.toml":     "edition = \"2021\"\n",
		"site/clippy.toml":      "[lints]\n",
		"site/.vale.ini":        "StylesPath = styles\n",
		"site/styles/org/Rules.yml": "extends: existence\n",
		"site/taplo.toml":       "[formatting]\n",
		"site/.buildifier.json": "{}\n",
		"site/custom.toml":      "[custom]\n",
		"site/vale_test.ini":    "StylesPath = styles\n",
	}, "")
	want := map[string]string{
		"rustfmt_config":    "rustfmt.toml",
		"clippy_config":     "clippy.toml",
		"vale_config":       ".vale.ini",
		"taplo_config":      "taplo.toml",
		"buildifier_config": ".buildifier.json",
	}
	if len(result.Gen) != len(want)+1 {
		names := []string{}
		for _, r := range result.Gen {
			names = append(names, r.Kind()+":"+r.Name())
		}
		t.Fatalf("generated %d rules %v, want lib plus %d configs", len(result.Gen), names, len(want))
	}
	lib := findGenerated(result, libraryKind, "site")
	if lib == nil {
		t.Fatal("missing dx_rust_library(site)")
	}
	if got := strings.Join(lib.AttrStrings("aspect_hints"), ","); got != ":clippy_config,:rustfmt_config" {
		t.Errorf("library aspect_hints = %q, want managed rust additions in canonical order", got)
	}
	for name, src := range want {
		kind := strings.TrimSuffix(name, "_config") + "_config"
		r := findGenerated(result, kind, name)
		if r == nil {
			t.Errorf("missing %s(%s)", kind, name)
			continue
		}
		if r.AttrString("src") != src {
			t.Errorf("%s src = %q, want %q", name, r.AttrString("src"), src)
		}
		if got := strings.Join(r.AttrStrings("visibility"), ","); got != "//site:__subpackages__" {
			t.Errorf("%s visibility = %q, want package-scoped", name, got)
		}
	}
	if findGenerated(result, "clippy_config", "clippy_cfg") != nil {
		t.Error("unrecognized vale_test.ini/custom.toml produced a config target")
	}
	vale := findGenerated(result, "vale_config", "vale_config")
	if got := strings.Join(vale.AttrStrings("data"), ","); got != "site/styles/org/Rules.yml" {
		t.Errorf("vale data = %q, want styles closure", got)
	}
	if rustfmt := findGenerated(result, "rustfmt_config", "rustfmt_config"); len(rustfmt.AttrStrings("data")) != 0 {
		t.Errorf("rustfmt data = %v, want no closure", rustfmt.AttrStrings("data"))
	}
}

func TestNativeConfigSelection(t *testing.T) {
	files := map[string]string{
		"site/src/lib.rs":   "pub fn current() {}\n",
		"site/rustfmt.toml": "edition = \"2021\"\n",
		"site/clippy.toml":  "[lints]\n",
	}
	_, result := runNativeGenerate(t, "site", files, "# gazelle:dx_native_tools rustfmt\n")
	if findGenerated(result, "rustfmt_config", "rustfmt_config") == nil {
		t.Error("selected rustfmt produced no target")
	}
	if findGenerated(result, "clippy_config", "clippy_config") != nil {
		t.Error("skipped clippy produced a target")
	}
	lib := findGenerated(result, libraryKind, "site")
	if got := strings.Join(lib.AttrStrings("aspect_hints"), ","); got != ":rustfmt_config" {
		t.Errorf("library aspect_hints = %q, want only the selected tool", got)
	}

	l, _ := runNativeGenerate(t, "site", files, "# gazelle:dx_native_tools rustfmt bogus\n")
	if len(l.errors) != 1 || !strings.Contains(l.errors[0], `unknown native tool "bogus"`) {
		t.Errorf("unknown tool errors = %v", l.errors)
	}

	bare, _ := runNativeGenerate(t, "site", files, "# gazelle:dx_native_tools\n")
	if len(bare.errors) != 1 || !strings.Contains(bare.errors[0], "malformed") {
		t.Errorf("bare directive errors = %v", bare.errors)
	}
}

func TestNativeConfigDirectiveInheritance(t *testing.T) {
	cfg := config.New()
	l := &rustLang{}
	l.Configure(cfg, "", nil)
	parent, err := rule.LoadData("BUILD.bazel", "", []byte("# gazelle:dx_native_tools clippy\n"))
	if err != nil {
		t.Fatal(err)
	}
	l.Configure(cfg, "", parent)
	if got := strings.Join(selectedNativeTools(cfg), ","); got != "clippy" {
		t.Fatalf("root tools = %q, want clippy", got)
	}
	child, err := rule.LoadData("BUILD.bazel", "sub", []byte("# build comment, no directive\n"))
	if err != nil {
		t.Fatal(err)
	}
	l.Configure(cfg, "sub", child)
	if got := strings.Join(selectedNativeTools(cfg), ","); got != "clippy" {
		t.Errorf("inherited tools = %q, want clippy", got)
	}
	override, err := rule.LoadData("BUILD.bazel", "other", []byte("# gazelle:dx_native_tools rustfmt\n# gazelle:dx_native_tools taplo\n"))
	if err != nil {
		t.Fatal(err)
	}
	l.Configure(cfg, "other", override)
	if got := strings.Join(selectedNativeTools(cfg), ","); got != "taplo" {
		t.Errorf("overridden tools = %q, want nearest directive to win", got)
	}
}

func TestNativeHintsMerge(t *testing.T) {
	build := `load("@rules_dx//rust/rules:defs.bzl", "dx_rust_library")

dx_rust_library(
    name = "site",
    srcs = ["src/lib.rs"],
    crate_name = "site",
    crate_root = "src/lib.rs",
    aspect_hints = [
        ":hand_cfg",
        ":rustfmt_config",
        "//site:clippy_config",
    ],
)
`
	_, result := runNativeGenerate(t, "site", map[string]string{
		"site/src/lib.rs":  "pub fn current() {}\n",
		"site/clippy.toml": "[lints]\n",
	}, build)
	lib := findGenerated(result, libraryKind, "site")
	if lib == nil {
		t.Fatal("missing dx_rust_library(site)")
	}
	// :rustfmt_config is stale (file gone) and drops; the hand entry and
	// the canonical-form managed entry survive in place; clippy resolves
	// to the existing target so nothing appends.
	if got := strings.Join(lib.AttrStrings("aspect_hints"), ","); got != ":hand_cfg,//site:clippy_config" {
		t.Errorf("merged aspect_hints = %q", got)
	}
	if findGenerated(result, "rustfmt_config", "rustfmt_config") != nil {
		t.Error("absent rustfmt.toml produced a target")
	}
}

func TestNativeHandConfigWins(t *testing.T) {
	build := `load("//quality:native_config.bzl", "clippy_config")

clippy_config(
    name = "clippy_cfg",
    src = "custom.toml",
)
`
	_, result := runNativeGenerate(t, "site", map[string]string{
		"site/src/lib.rs":   "pub fn current() {}\n",
		"site/clippy.toml":  "[lints]\n",
		"site/custom.toml":  "[custom]\n",
	}, build)
	if findGenerated(result, "clippy_config", "clippy_config") != nil {
		t.Error("hand clippy_cfg did not suppress generation")
	}
	lib := findGenerated(result, libraryKind, "site")
	if got := strings.Join(lib.AttrStrings("aspect_hints"), ","); got != ":clippy_cfg" {
		t.Errorf("library aspect_hints = %q, want the hand target", got)
	}
}

func TestNativeRemoval(t *testing.T) {
	build := `load("//quality:native_config.bzl", "rustfmt_config")
load("@rules_dx//rust/rules:defs.bzl", "dx_rust_library")

rustfmt_config(
    name = "rustfmt_config",
    src = "rustfmt.toml",
)

dx_rust_library(
    name = "site",
    srcs = ["src/lib.rs"],
    crate_name = "site",
    crate_root = "src/lib.rs",
    aspect_hints = [":rustfmt_config"],
)
`
	// The config file is gone: the generated-shaped rule stubs out and
	// the stale hint disappears with it, leaving no residue.
	_, result := runNativeGenerate(t, "site", map[string]string{
		"site/src/lib.rs": "pub fn current() {}\n",
	}, build)
	if !findEmpty(result, "rustfmt_config", "rustfmt_config") {
		t.Error("removed rustfmt.toml did not stub the generated target")
	}
	lib := findGenerated(result, libraryKind, "site")
	if lib.Attr("aspect_hints") != nil {
		t.Errorf("library aspect_hints = %v, want the attribute gone", lib.AttrStrings("aspect_hints"))
	}
}

func TestNativeHandSurvivesRemoval(t *testing.T) {
	build := `load("//quality:native_config.bzl", "rustfmt_config")
load("@rules_dx//rust/rules:defs.bzl", "dx_rust_library")

rustfmt_config(
    name = "rustfmt_config",
    src = "custom.toml",
)

dx_rust_library(
    name = "site",
    srcs = ["src/lib.rs"],
    crate_name = "site",
    crate_root = "src/lib.rs",
    aspect_hints = [":rustfmt_config"],
)
`
	// Same default name but a hand-owned src: never stubbed even though
	// the recognized file is absent, and the hint keeps resolving.
	_, result := runNativeGenerate(t, "site", map[string]string{
		"site/src/lib.rs":  "pub fn current() {}\n",
		"site/custom.toml": "[custom]\n",
	}, build)
	if findEmpty(result, "rustfmt_config", "rustfmt_config") {
		t.Error("hand-customized target was stubbed")
	}
	lib := findGenerated(result, libraryKind, "site")
	if got := strings.Join(lib.AttrStrings("aspect_hints"), ","); got != ":rustfmt_config" {
		t.Errorf("library aspect_hints = %q, want the surviving hand target", got)
	}
}

func TestNativeDeselectedOmitsGenerated(t *testing.T) {
	build := `# gazelle:dx_native_tools clippy
load("//quality:native_config.bzl", "clippy_config", "rustfmt_config")

rustfmt_config(
    name = "rustfmt_config",
    src = "rustfmt.toml",
)

clippy_config(
    name = "clippy_cfg",
    src = "custom.toml",
)
`
	// A skipped tool omits its generated target (stubbed here because the
	// rule is generator-shaped) but preserves the hand-maintained one.
	_, result := runNativeGenerate(t, "site", map[string]string{
		"site/rustfmt.toml": "edition = \"2021\"\n",
		"site/custom.toml":  "[custom]\n",
	}, build)
	if !findEmpty(result, "rustfmt_config", "rustfmt_config") {
		t.Error("deselected rustfmt did not omit its generated target")
	}
	if findEmpty(result, "clippy_config", "clippy_cfg") {
		t.Error("hand clippy_cfg was stubbed")
	}
	if findGenerated(result, "clippy_config", "clippy_config") != nil {
		t.Error("deselected clippy file absence still produced a target")
	}
}

func TestNativeAmbiguousFails(t *testing.T) {
	build := `load("//quality:native_config.bzl", "clippy_config")

clippy_config(
    name = "clippy_cfg",
    src = "custom.toml",
)

clippy_config(
    name = "clippy_extra",
    src = "extra.toml",
)
`
	l, result := runNativeGenerate(t, "site", map[string]string{
		"site/clippy.toml": "[lints]\n",
	}, build)
	if len(result.Gen) != 0 || len(result.Empty) != 0 {
		t.Error("ambiguous configs produced a partial result")
	}
	if len(l.errors) != 1 || !strings.Contains(l.errors[0], "2 config targets") {
		t.Errorf("ambiguous errors = %v", l.errors)
	}
}

func TestNativeConfigOnlyDir(t *testing.T) {
	_, result := runNativeGenerate(t, "site", map[string]string{
		"site/taplo.toml": "[formatting]\n",
	}, "")
	if len(result.Gen) != 1 {
		t.Fatalf("generated %d rules, want only taplo_config", len(result.Gen))
	}
	taplo := findGenerated(result, "taplo_config", "taplo_config")
	if taplo == nil || taplo.AttrString("src") != "taplo.toml" {
		t.Errorf("taplo target = %v", taplo)
	}
}

func TestNativeRootVisibility(t *testing.T) {
	_, result := runNativeGenerate(t, "", map[string]string{
		"rustfmt.toml": "edition = \"2021\"\n",
	}, "")
	r := findGenerated(result, "rustfmt_config", "rustfmt_config")
	if r == nil {
		t.Fatal("missing root rustfmt_config")
	}
	if got := strings.Join(r.AttrStrings("visibility"), ","); got != "//:__subpackages__" {
		t.Errorf("root visibility = %q", got)
	}
}

func TestNativeValeStylesVariants(t *testing.T) {
	// Custom StylesPath closes over its own directory.
	_, custom := runNativeGenerate(t, "site", map[string]string{
		"site/.vale.ini":            "StylesPath = config/styles\n",
		"site/config/styles/a.yml":  "extends: existence\n",
		"site/config/styles/b/c.yml": "extends: occurrence\n",
	}, "")
	vale := findGenerated(custom, "vale_config", "vale_config")
	if got := strings.Join(vale.AttrStrings("data"), ","); got != "site/config/styles/a.yml,site/config/styles/b/c.yml" {
		t.Errorf("custom styles data = %q", got)
	}
	// No styles directory means a self-contained config with no data.
	_, bare := runNativeGenerate(t, "site", map[string]string{
		"site/.vale.ini": "MinAlertLevel = suggestion\n",
	}, "")
	plain := findGenerated(bare, "vale_config", "vale_config")
	if plain == nil {
		t.Fatal("missing self-contained vale_config")
	}
	if plain.Attr("data") != nil {
		t.Errorf("self-contained vale data = %v, want absent", plain.AttrStrings("data"))
	}
}

func TestNativeCargoHints(t *testing.T) {
	_, result := runNativeGenerate(t, "app", map[string]string{
		"app/Cargo.toml":    "[package]\nname = \"app\"\nversion = \"0.1.0\"\n",
		"app/src/main.rs":   "fn main() {}\n",
		"app/rustfmt.toml":  "edition = \"2021\"\n",
	}, "")
	bin := findGenerated(result, binaryKind, "app")
	if bin == nil {
		t.Fatal("missing dx_rust_binary(app)")
	}
	if got := strings.Join(bin.AttrStrings("aspect_hints"), ","); got != ":rustfmt_config" {
		t.Errorf("binary aspect_hints = %q", got)
	}
	if findGenerated(result, "rustfmt_config", "rustfmt_config") == nil {
		t.Error("cargo directory produced no rustfmt_config")
	}
}
