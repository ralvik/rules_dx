package svelte

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

func generateFixture(t *testing.T, files map[string]string, regular []string) language.GenerateResult {
	t.Helper()
	root := t.TempDir()
	for name, content := range files {
		writeFixture(t, root, name, content)
	}
	dir := filepath.Join(root, "pkg", "demo")
	return NewLanguage().GenerateRules(language.GenerateArgs{
		Config:       &config.Config{RepoRoot: root},
		Dir:          dir,
		Rel:          "pkg/demo",
		RegularFiles: regular,
	})
}

func component(script string) string {
	return "<script>\n" + script + "</script>\n\n<div class=\"hello\">hello</div>\n\n<style>\n.hello {\n  color: black;\n}\n</style>\n"
}

func TestGenerateSourceOnlyPackage(t *testing.T) {
	regular := []string{"demo.svelte", "helper.svelte", "notes.txt", "helper.js"}
	result := generateFixture(t, map[string]string{
		"pkg/demo/demo.svelte":   component("import helper from \"./helper.svelte\";\nimport fs from \"fs\";\n"),
		"pkg/demo/helper.svelte": component("export const label = \"\";\n"),
		"pkg/demo/notes.txt":     "not a source\n",
		"pkg/demo/helper.js":     "export const x = 1;\n",
	}, regular)
	if len(result.Gen) != 2 || len(result.Imports) != 2 {
		t.Fatalf("generated %d rules and %d import sets, want 2 each", len(result.Gen), len(result.Imports))
	}
	lib := result.Gen[0]
	if lib.Kind() != libraryKind || lib.Name() != "demo" {
		t.Fatalf("library = %s(%s)", lib.Kind(), lib.Name())
	}
	if got := strings.Join(lib.AttrStrings("srcs"), ","); got != "demo.svelte" {
		t.Errorf("library srcs = %q", got)
	}
	helper := result.Gen[1]
	if helper.Kind() != libraryKind || helper.Name() != "helper" {
		t.Fatalf("helper = %s(%s)", helper.Kind(), helper.Name())
	}
	if got := strings.Join(helper.AttrStrings("srcs"), ","); got != "helper.svelte" {
		t.Errorf("helper srcs = %q", got)
	}
	libImports := result.Imports[0].(targetImports)
	if strings.Join(libImports.imports, ",") != "helper" {
		t.Errorf("library imports = %+v, want [helper]", libImports)
	}
	helperImports := result.Imports[1].(targetImports)
	if len(helperImports.imports) != 0 {
		t.Errorf("helper imports = %+v, want empty", helperImports)
	}
}

func TestGenerateModuleScriptImports(t *testing.T) {
	regular := []string{"demo.svelte", "helper.svelte"}
	result := generateFixture(t, map[string]string{
		"pkg/demo/demo.svelte":   "<script context=\"module\">\nimport helper from \"./helper.svelte\";\n</script>\n\n<script>\nimport fs from \"fs\";\n</script>\n\n<div>x</div>\n",
		"pkg/demo/helper.svelte": component("export const label = \"\";\n"),
	}, regular)
	if len(result.Gen) != 2 || len(result.Imports) != 2 {
		t.Fatalf("generated %d rules and %d import sets, want 2 each", len(result.Gen), len(result.Imports))
	}
	libImports := result.Imports[0].(targetImports)
	if strings.Join(libImports.imports, ",") != "helper" {
		t.Errorf("library imports = %+v, want [helper]", libImports)
	}
}

func TestGenerateEmptySweepsStale(t *testing.T) {
	result := generateFixture(t, nil, []string{"notes.txt"})
	if len(result.Gen) != 0 || len(result.Empty) != 0 {
		t.Fatalf("empty generation = %+v", result)
	}
	f := rule.EmptyFile("BUILD.bazel", "pkg/demo")
	f.Rules = append(f.Rules,
		rule.NewRule(libraryKind, "old"),
		rule.NewRule("filegroup", "keep"),
		rule.NewRule(libraryKind, "demo"),
	)
	root := t.TempDir()
	writeFixture(t, root, "pkg/demo/demo.svelte", component("export const x = 1;\n"))
	dir := filepath.Join(root, "pkg", "demo")
	result = NewLanguage().GenerateRules(language.GenerateArgs{
		Config:       &config.Config{RepoRoot: root},
		Dir:          dir,
		Rel:          "pkg/demo",
		RegularFiles: []string{"demo.svelte"},
		File:         f,
	})
	if len(result.Gen) != 1 {
		t.Fatalf("generated %d rules, want 1", len(result.Gen))
	}
	if len(result.Empty) != 1 || result.Empty[0].Name() != "old" {
		t.Fatalf("empty rules = %v, want [old]", result.Empty)
	}
}

func TestGenerateFailures(t *testing.T) {
	cases := []struct {
		name    string
		files   map[string]string
		regular []string
		want    string
	}{
		{"readError", map[string]string{}, []string{"missing.svelte"}, "read missing.svelte"},
		{"emptyName", map[string]string{"pkg/demo/---.svelte": component("export const x = 1;\n")}, []string{"---.svelte"}, "empty target name"},
		{"collision", map[string]string{
			"pkg/demo/a-b.svelte": component("export const x = 1;\n"),
			"pkg/demo/a_b.svelte": component("export const x = 1;\n"),
		}, []string{"a-b.svelte", "a_b.svelte"}, "claimed by a-b.svelte, a_b.svelte"},
	}
	for _, tc := range cases {
		root := t.TempDir()
		for name, content := range tc.files {
			writeFixture(t, root, name, content)
		}
		l := &svelteLang{}
		result := l.GenerateRules(language.GenerateArgs{
			Config:       &config.Config{RepoRoot: root},
			Dir:          filepath.Join(root, "pkg", "demo"),
			Rel:          "pkg/demo",
			RegularFiles: tc.regular,
		})
		if len(result.Gen) != 0 {
			t.Errorf("%s: generated %d rules, want none", tc.name, len(result.Gen))
		}
		if len(l.errors) != 1 || !strings.Contains(l.errors[0], tc.want) {
			t.Errorf("%s: errors = %v, want %q", tc.name, l.errors, tc.want)
		}
	}
}

func TestCheckClaims(t *testing.T) {
	claimants := []Claimant{{Name: "demo", Source: "demo.svelte", Kind: libraryKind}}
	if err := checkClaims(nil, nil, claimants); err != nil {
		t.Errorf("unique claims = %v", err)
	}
	sameKind := rule.EmptyFile("BUILD.bazel", "pkg")
	sameKind.Rules = append(sameKind.Rules, rule.NewRule(libraryKind, "demo"))
	if err := checkClaims(sameKind, nil, claimants); err != nil {
		t.Errorf("same-kind handwritten claims = %v", err)
	}
	otherSame := []*rule.Rule{rule.NewRule(libraryKind, "demo")}
	if err := checkClaims(nil, otherSame, claimants); err != nil {
		t.Errorf("same-kind other claims = %v", err)
	}
	wrongKind := rule.EmptyFile("BUILD.bazel", "pkg")
	wrongKind.Rules = append(wrongKind.Rules, rule.NewRule("filegroup", "demo"))
	wrongKind.Rules = append(wrongKind.Rules, rule.NewRule("filegroup", "a_b"))
	if err := checkClaims(wrongKind, nil, claimants); err == nil || !strings.Contains(err.Error(), "existing filegroup") {
		t.Errorf("kind mismatch = %v", err)
	}
	otherWrong := []*rule.Rule{rule.NewRule("filegroup", "demo")}
	if err := checkClaims(nil, otherWrong, claimants); err == nil || !strings.Contains(err.Error(), "existing filegroup") {
		t.Errorf("other kind mismatch = %v", err)
	}
	dupes := []Claimant{{Name: "a_b", Source: "a-b.svelte", Kind: libraryKind}, {Name: "a_b", Source: "a_b.svelte", Kind: libraryKind}}
	if err := checkClaims(wrongKind, nil, dupes); err == nil || !strings.Contains(err.Error(), "handwritten") {
		t.Errorf("collision with handwritten = %v", err)
	}
}

func TestMergeStale(t *testing.T) {
	if got := mergeStale(nil, language.GenerateResult{}); len(got.Empty) != 0 {
		t.Errorf("nil file stale = %v", got.Empty)
	}
	f := rule.EmptyFile("BUILD.bazel", "pkg")
	f.Rules = append(f.Rules,
		rule.NewRule(libraryKind, "old"),
		rule.NewRule("filegroup", "keep"),
	)
	kept := rule.NewRule(libraryKind, "demo")
	result := mergeStale(f, language.GenerateResult{Gen: []*rule.Rule{kept}})
	if len(result.Empty) != 1 || result.Empty[0].Name() != "old" {
		t.Fatalf("stale = %v", result.Empty)
	}
}

func TestImportsIndexesLibraries(t *testing.T) {
	lang := NewLanguage()
	lib := rule.NewRule(libraryKind, "demo")
	lib.SetAttr("srcs", []string{"demo.svelte", "notes.txt"})
	imports := lang.Imports(&config.Config{}, lib, nil)
	if len(imports) != 1 || imports[0].Lang != languageName || imports[0].Imp != "demo" {
		t.Errorf("imports = %+v", imports)
	}
	if got := lang.Imports(&config.Config{}, rule.NewRule("filegroup", "demo"), nil); got != nil {
		t.Errorf("other-kind imports = %+v, want nil", got)
	}
	if got := lang.Imports(&config.Config{}, rule.NewRule(libraryKind, "empty"), nil); got != nil {
		t.Errorf("srcless imports = %+v, want nil", got)
	}
}

func TestErrorsAbortBeforeEmission(t *testing.T) {
	l := &svelteLang{}
	l.Before(context.Background())
	l.fail("second")
	l.fail("first")
	defer func() {
		got := recover()
		if got == nil {
			t.Fatal("AfterResolvingDeps did not abort")
		}
		if message := got.(string); !strings.Contains(message, "first\nsecond") {
			t.Errorf("errors were not deterministic: %s", message)
		}
	}()
	l.AfterResolvingDeps(context.Background())
}

func TestAfterResolvingDepsClean(t *testing.T) {
	l := &svelteLang{}
	l.Before(context.Background())
	l.AfterResolvingDeps(context.Background())
	if len(l.errors) != 0 {
		t.Errorf("errors = %v", l.errors)
	}
}

func TestLanguageMetadata(t *testing.T) {
	l := &svelteLang{}
	if l.Name() != "svelte" || len(l.Kinds()) != 1 || l.CheckFlags(flag.NewFlagSet("test", flag.ContinueOnError), config.New()) != nil {
		t.Fatal("invalid language metadata")
	}
	l.RegisterFlags(flag.NewFlagSet("test", flag.ContinueOnError), "update", config.New())
	l.Configure(config.New(), "pkg", nil)
	l.DoneGeneratingRules()
	if got := l.KnownDirectives(); len(got) != 1 || got[0] != "dx_ignore_import" {
		t.Fatalf("directives = %v, want [dx_ignore_import]", got)
	}
	if l.Embeds(nil, label.NoLabel) != nil {
		t.Fatal("invalid directives or embeds")
	}
	loads := l.ApparentLoads(func(name string) string {
		if name == "rules_dx" {
			return "renamed_dx"
		}
		return ""
	})
	if len(loads) != 1 || loads[0].Name != "@renamed_dx//svelte/rules:defs.bzl" || strings.Join(loads[0].Symbols, ",") != "dx_svelte_library" {
		t.Errorf("apparent loads = %+v", loads)
	}
	if defaults := l.Loads(); len(defaults) != 1 || defaults[0].Name != "@rules_dx//svelte/rules:defs.bzl" {
		t.Errorf("default loads = %+v", defaults)
	}
	defaultApparent := l.ApparentLoads(func(string) string { return "" })
	if defaultApparent[0].Name != "@rules_dx//svelte/rules:defs.bzl" {
		t.Errorf("default apparent loads = %+v", defaultApparent)
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

func resolverIndex(lang *svelteLang, entries ...struct {
	pkg  string
	name string
	ext  string
}) *resolve.RuleIndex {
	index := resolve.NewRuleIndex(func(r *rule.Rule, _ string) resolve.Resolver {
		if _, ok := svelteKinds[r.Kind()]; ok {
			return lang
		}
		return nil
	})
	for _, entry := range entries {
		ext := entry.ext
		if ext == "" {
			ext = ".svelte"
		}
		r := rule.NewRule(libraryKind, entry.name)
		r.SetAttr("srcs", []string{entry.name + ext})
		index.AddRule(config.New(), r, rule.EmptyFile(filepath.Join(entry.pkg, "BUILD.bazel"), entry.pkg))
	}
	index.Finish()
	return index
}

func TestResolveBranches(t *testing.T) {
	l := &svelteLang{}
	index := resolverIndex(l,
		struct{ pkg, name string; ext string }{"lib/b", "b", ".svelte"},
		struct{ pkg, name string; ext string }{"lib/a", "a", ".svelte"},
	)
	cfg := resolverConfig(t, nil)
	r := rule.NewRule(libraryKind, "app")
	l.Resolve(cfg, index, nil, r, targetImports{imports: []string{"b", "a"}}, label.New("", "app", "app"))
	if got := strings.Join(r.AttrStrings("deps"), ","); got != "//lib/a,//lib/b" {
		t.Errorf("resolved deps = %q", got)
	}

	self := rule.NewRule(libraryKind, "a")
	l.Resolve(cfg, index, nil, self, targetImports{imports: []string{"a"}}, label.New("", "lib/a", "a"))
	if self.Attr("deps") != nil {
		t.Errorf("self dependency was emitted: %v", self.AttrStrings("deps"))
	}

	otherKind := rule.NewRule("filegroup", "other")
	l.Resolve(cfg, index, nil, otherKind, targetImports{imports: []string{"a"}}, label.New("", "app", "other"))
	if otherKind.Attr("deps") != nil {
		t.Errorf("non-library resolution emitted deps = %v", otherKind.AttrStrings("deps"))
	}

	std := rule.NewRule(libraryKind, "uses_std")
	l.Resolve(cfg, index, nil, std, targetImports{imports: []string{"fs"}}, label.New("", "app", "uses_std"))
	if std.Attr("deps") != nil || len(l.errors) != 0 {
		t.Errorf("stdlib resolution = %v, errors = %v", std.AttrStrings("deps"), l.errors)
	}

	hand := rule.NewRule(libraryKind, "hand")
	hand.SetAttr("deps", []string{":kept"})
	l.Resolve(cfg, index, nil, hand, targetImports{imports: []string{"a"}}, label.New("", "app", "hand"))
	if got := strings.Join(hand.AttrStrings("deps"), ","); got != "//lib/a,:kept" {
		t.Errorf("merged deps = %q", got)
	}

	l.Resolve(cfg, index, nil, rule.NewRule(libraryKind, "unknown"), targetImports{imports: []string{"unknown"}}, label.New("", "app", "unknown"))
	if len(l.errors) != 1 || !strings.Contains(l.errors[0], "unresolved import") {
		t.Errorf("unresolved errors = %v", l.errors)
	}
	l.Resolve(cfg, index, nil, r, nil, label.New("", "app", "app"))
}

func TestResolveOverride(t *testing.T) {
	l := &svelteLang{}
	cfg := resolverConfig(t, []rule.Directive{{Key: "resolve", Value: "svelte svelte mapped //mapped:dep"}})
	r := rule.NewRule(libraryKind, "app")
	l.Resolve(cfg, resolverIndex(l), nil, r, targetImports{imports: []string{"mapped"}}, label.New("", "app", "app"))
	if got := strings.Join(r.AttrStrings("deps"), ","); got != "//mapped:dep" {
		t.Errorf("override deps = %q", got)
	}
}

func TestResolveAmbiguous(t *testing.T) {
	l := &svelteLang{}
	cfg := resolverConfig(t, nil)
	index := resolverIndex(l,
		struct{ pkg, name string; ext string }{"one", "same", ".svelte"},
		struct{ pkg, name string; ext string }{"two", "same", ".svelte"},
	)
	l.Resolve(cfg, index, nil, rule.NewRule(libraryKind, "app"), targetImports{imports: []string{"same"}}, label.New("", "app", "app"))
	if len(l.errors) != 1 || !strings.Contains(l.errors[0], "ambiguous") || !strings.Contains(l.errors[0], "//one:same") || !strings.Contains(l.errors[0], "//two:same") {
		t.Errorf("ambiguity = %v", l.errors)
	}
}

func TestUnionStrings(t *testing.T) {
	if got := unionStrings([]string{"b", "a", "a"}, []string{"c", "a"}); strings.Join(got, ",") != "a,b,c" {
		t.Errorf("union = %q", got)
	}
	if got := unionStrings(nil, nil); len(got) != 0 {
		t.Errorf("empty union = %q", got)
	}
}

func TestIgnoreDirectiveInheritanceAndStaleCheck(t *testing.T) {
	l := &svelteLang{}
	l.Before(context.Background())
	root := config.New()
	file, err := rule.LoadData("BUILD.bazel", "", []byte("# gazelle:dx_ignore_import svelte missing_dep\n"))
	if err != nil {
		t.Fatal(err)
	}
	l.Configure(root, "", file)
	child := root.Clone()
	l.Configure(child, "child", nil)
	ignore := matchingIgnore(child, "missing_dep")
	if ignore == nil || ignore.path != "" {
		t.Fatalf("inherited ignore = %+v", ignore)
	}
	ignore.used = true
	l.AfterResolvingDeps(context.Background())
}

func TestStaleIgnoreFails(t *testing.T) {
	l := &svelteLang{}
	l.Before(context.Background())
	cfg := config.New()
	file, err := rule.LoadData("BUILD.bazel", "pkg", []byte("# gazelle:dx_ignore_import svelte stale\n"))
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

func TestResolveIgnore(t *testing.T) {
	l := &svelteLang{}
	l.Before(context.Background())
	cfg := resolverConfig(t, nil)
	file, err := rule.LoadData("BUILD.bazel", "app", []byte("# gazelle:dx_ignore_import svelte external\n"))
	if err != nil {
		t.Fatal(err)
	}
	l.Configure(cfg, "app", file)
	r := rule.NewRule(libraryKind, "app")
	l.Resolve(cfg, resolverIndex(l), nil, r, targetImports{imports: []string{"external"}}, label.New("", "app", "app"))
	if r.Attr("deps") != nil {
		t.Errorf("ignored import produced deps = %v", r.AttrStrings("deps"))
	}
	if len(l.errors) != 0 {
		t.Errorf("ignored import errors = %v", l.errors)
	}
	l.AfterResolvingDeps(context.Background())
}

func TestResolveMappingIgnoreConflict(t *testing.T) {
	l := &svelteLang{}
	l.Before(context.Background())
	cfg := resolverConfig(t, []rule.Directive{{Key: "resolve", Value: "svelte svelte mapped //mapped:dep"}})
	file, err := rule.LoadData("BUILD.bazel", "app", []byte("# gazelle:dx_ignore_import svelte mapped\n"))
	if err != nil {
		t.Fatal(err)
	}
	l.Configure(cfg, "app", file)
	r := rule.NewRule(libraryKind, "app")
	l.Resolve(cfg, resolverIndex(l), nil, r, targetImports{imports: []string{"mapped"}}, label.New("", "app", "app"))
	if len(l.errors) != 1 || !strings.Contains(l.errors[0], "both an exact resolve mapping and ignore") {
		t.Errorf("conflict errors = %v", l.errors)
	}
}

func TestConfigureSkipsOtherDirectives(t *testing.T) {
	l := &svelteLang{}
	l.Before(context.Background())
	cfg := config.New()
	file, err := rule.LoadData("BUILD.bazel", "pkg", []byte("# gazelle:resolve svelte svelte foo //foo:bar\n# gazelle:dx_ignore_import python foo\n"))
	if err != nil {
		t.Fatal(err)
	}
	l.Configure(cfg, "pkg", file)
	if got := matchingIgnore(cfg, "foo"); got != nil {
		t.Errorf("other-directive ignore = %+v, want nil", got)
	}
	if len(l.errors) != 0 {
		t.Errorf("other-directive errors = %v", l.errors)
	}
}

func TestConfigureThreeFieldAndMalformed(t *testing.T) {
	l := &svelteLang{}
	l.Before(context.Background())
	cfg := config.New()
	file, err := rule.LoadData("BUILD.bazel", "pkg", []byte("# gazelle:dx_ignore_import svelte svelte mydep\n"))
	if err != nil {
		t.Fatal(err)
	}
	l.Configure(cfg, "pkg", file)
	ignore := matchingIgnore(cfg, "mydep")
	if ignore == nil || ignore.value != "mydep" {
		t.Fatalf("three-field ignore = %+v, want mydep", ignore)
	}
	ignore.used = true

	malformed := &svelteLang{}
	malformed.Before(context.Background())
	badCfg := config.New()
	badFile, err := rule.LoadData("BUILD.bazel", "pkg", []byte("# gazelle:dx_ignore_import svelte\n"))
	if err != nil {
		t.Fatal(err)
	}
	malformed.Configure(badCfg, "pkg", badFile)
	if len(malformed.errors) != 1 || !strings.Contains(malformed.errors[0], "malformed") {
		t.Errorf("malformed errors = %v", malformed.errors)
	}
	l.AfterResolvingDeps(context.Background())
}

func TestMatchingIgnoreMiss(t *testing.T) {
	l := &svelteLang{}
	l.Before(context.Background())
	cfg := config.New()
	file, err := rule.LoadData("BUILD.bazel", "app", []byte("# gazelle:dx_ignore_import svelte other\n"))
	if err != nil {
		t.Fatal(err)
	}
	l.Configure(cfg, "app", file)
	if got := matchingIgnore(cfg, "missing"); got != nil {
		t.Errorf("ignore miss = %+v, want nil", got)
	}
	if got := matchingIgnore(config.New(), "missing"); got != nil {
		t.Errorf("no-exts ignore = %+v, want nil", got)
	}
}
