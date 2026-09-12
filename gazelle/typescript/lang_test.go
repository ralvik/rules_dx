package typescript

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

func TestGenerateSourceOnlyPackage(t *testing.T) {
	regular := []string{"demo.ts", "helper.tsx", "helper_test.mts", "notes.txt", "widget.d.ts", "orphan.d.mts"}
	result := generateFixture(t, map[string]string{
		"pkg/demo/demo.ts":        "import helper from \"./helper.tsx\";\nimport fs from \"fs\";\n",
		"pkg/demo/helper.tsx":     "export const suffix = (t: string) => t;\n",
		"pkg/demo/helper_test.mts": "import helper from \"./helper.tsx\";\n",
		"pkg/demo/notes.txt":      "not a source\n",
		"pkg/demo/widget.d.ts":    "export declare const x: number;\n",
		"pkg/demo/orphan.d.mts":   "export declare const y: number;\n",
	}, regular)
	if len(result.Gen) != 3 || len(result.Imports) != 3 {
		t.Fatalf("generated %d rules and %d import sets, want 3 each", len(result.Gen), len(result.Imports))
	}
	demo := result.Gen[0]
	if demo.Kind() != projectKind || demo.Name() != "demo" {
		t.Fatalf("demo = %s(%s)", demo.Kind(), demo.Name())
	}
	if got := strings.Join(demo.AttrStrings("srcs"), ","); got != "demo.ts" {
		t.Errorf("demo srcs = %q", got)
	}
	helper := result.Gen[1]
	if helper.Kind() != projectKind || helper.Name() != "helper" {
		t.Fatalf("helper = %s(%s)", helper.Kind(), helper.Name())
	}
	test := result.Gen[2]
	if test.Kind() != projectKind || test.Name() != "helper_test" {
		t.Fatalf("test = %s(%s)", test.Kind(), test.Name())
	}
	demoImports := result.Imports[0].(targetImports)
	if strings.Join(demoImports.imports, ",") != "helper" {
		t.Errorf("demo imports = %+v", demoImports)
	}
	helperImports := result.Imports[1].(targetImports)
	if len(helperImports.imports) != 0 {
		t.Errorf("helper imports = %+v, want empty", helperImports)
	}
}

func TestGenerateEmptySweepsStale(t *testing.T) {
	result := generateFixture(t, nil, []string{"notes.txt", "orphan.d.ts"})
	if len(result.Gen) != 0 || len(result.Empty) != 0 {
		t.Fatalf("empty generation = %+v", result)
	}
	f := rule.EmptyFile("BUILD.bazel", "pkg/demo")
	f.Rules = append(f.Rules,
		rule.NewRule(projectKind, "old"),
		rule.NewRule("filegroup", "keep"),
		rule.NewRule(projectKind, "demo"),
	)
	root := t.TempDir()
	writeFixture(t, root, "pkg/demo/demo.ts", "export const x = 1;\n")
	dir := filepath.Join(root, "pkg", "demo")
	result = NewLanguage().GenerateRules(language.GenerateArgs{
		Config:       &config.Config{RepoRoot: root},
		Dir:          dir,
		Rel:          "pkg/demo",
		RegularFiles: []string{"demo.ts"},
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
		{"readError", map[string]string{}, []string{"missing.ts"}, "read missing.ts"},
		{"emptyName", map[string]string{"pkg/demo/---.ts": "export const x = 1;\n"}, []string{"---.ts"}, "empty target name"},
		{"collision", map[string]string{
			"pkg/demo/a-b.ts": "export const x = 1;\n",
			"pkg/demo/a_b.tsx": "export const y = 2;\n",
		}, []string{"a-b.ts", "a_b.tsx"}, "claimed by a-b.ts, a_b.tsx"},
	}
	for _, tc := range cases {
		root := t.TempDir()
		for name, content := range tc.files {
			writeFixture(t, root, name, content)
		}
		l := &typescriptLang{}
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

func TestImportsIndexesNonTestOnly(t *testing.T) {
	lang := NewLanguage()
	lib := rule.NewRule(projectKind, "demo")
	lib.SetAttr("srcs", []string{"demo.ts", "notes.txt"})
	imports := lang.Imports(&config.Config{}, lib, nil)
	if len(imports) != 1 || imports[0].Lang != languageName || imports[0].Imp != "demo" {
		t.Errorf("imports = %+v", imports)
	}
	testRule := rule.NewRule(projectKind, "demo_test")
	testRule.SetAttr("srcs", []string{"demo_test.ts"})
	if got := lang.Imports(&config.Config{}, testRule, nil); got != nil {
		t.Errorf("test imports = %+v, want nil", got)
	}
	if got := lang.Imports(&config.Config{}, rule.NewRule(projectKind, "empty"), nil); got != nil {
		t.Errorf("srcless imports = %+v, want nil", got)
	}
}

func TestLanguageMetadata(t *testing.T) {
	l := &typescriptLang{}
	if l.Name() != "typescript" || len(l.Kinds()) != 1 || l.CheckFlags(flag.NewFlagSet("test", flag.ContinueOnError), config.New()) != nil {
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
	if len(loads) != 1 || loads[0].Name != "@renamed_dx//typescript/rules:defs.bzl" || strings.Join(loads[0].Symbols, ",") != "dx_ts_project" {
		t.Errorf("apparent loads = %+v", loads)
	}
	if defaults := l.Loads(); len(defaults) != 1 || defaults[0].Name != "@rules_dx//typescript/rules:defs.bzl" {
		t.Errorf("default loads = %+v", defaults)
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

func resolverIndex(lang *typescriptLang, entries ...struct {
	pkg  string
	name string
	ext  string
}) *resolve.RuleIndex {
	index := resolve.NewRuleIndex(func(r *rule.Rule, _ string) resolve.Resolver {
		if _, ok := typescriptKinds[r.Kind()]; ok {
			return lang
		}
		return nil
	})
	for _, entry := range entries {
		ext := entry.ext
		if ext == "" {
			ext = ".ts"
		}
		r := rule.NewRule(projectKind, entry.name)
		r.SetAttr("srcs", []string{entry.name + ext})
		// Test projects are leaves: only index non-test sources.
		if IsTestFile(entry.name + ext) {
			continue
		}
		index.AddRule(config.New(), r, rule.EmptyFile(filepath.Join(entry.pkg, "BUILD.bazel"), entry.pkg))
	}
	index.Finish()
	return index
}

func TestResolveBranches(t *testing.T) {
	l := &typescriptLang{}
	index := resolverIndex(l,
		struct{ pkg, name string; ext string }{"lib/b", "b", ".ts"},
		struct{ pkg, name string; ext string }{"lib/a", "a", ".tsx"},
	)
	cfg := resolverConfig(t, nil)
	r := rule.NewRule(projectKind, "app")
	l.Resolve(cfg, index, nil, r, targetImports{imports: []string{"b", "a"}}, label.New("", "app", "app"))
	if got := strings.Join(r.AttrStrings("deps"), ","); got != "//lib/a,//lib/b" {
		t.Errorf("resolved deps = %q", got)
	}

	self := rule.NewRule(projectKind, "a")
	self.SetAttr("srcs", []string{"a.ts"})
	l.Resolve(cfg, index, nil, self, targetImports{imports: []string{"a", "a"}}, label.New("", "lib/a", "a"))
	_ = self

	std := rule.NewRule(projectKind, "uses_std")
	l.Resolve(cfg, index, nil, std, targetImports{imports: []string{"fs"}}, label.New("", "app", "uses_std"))
	if std.Attr("deps") != nil || len(l.errors) != 0 {
		t.Errorf("stdlib resolution = %v, errors = %v", std.AttrStrings("deps"), l.errors)
	}

	hand := rule.NewRule(projectKind, "hand")
	hand.SetAttr("deps", []string{":kept"})
	l.Resolve(cfg, index, nil, hand, targetImports{imports: []string{"a"}}, label.New("", "app", "hand"))
	if got := strings.Join(hand.AttrStrings("deps"), ","); got != "//lib/a,:kept" {
		t.Errorf("merged deps = %q", got)
	}

	l.Resolve(cfg, index, nil, rule.NewRule(projectKind, "unknown"), targetImports{imports: []string{"unknown"}}, label.New("", "app", "unknown"))
	if len(l.errors) != 1 || !strings.Contains(l.errors[0], "unresolved import") {
		t.Errorf("unresolved errors = %v", l.errors)
	}
	l.Resolve(cfg, index, nil, r, nil, label.New("", "app", "app"))
}

func TestResolveOverride(t *testing.T) {
	l := &typescriptLang{}
	cfg := resolverConfig(t, []rule.Directive{{Key: "resolve", Value: "typescript typescript mapped //mapped:dep"}})
	r := rule.NewRule(projectKind, "app")
	l.Resolve(cfg, resolverIndex(l), nil, r, targetImports{imports: []string{"mapped"}}, label.New("", "app", "app"))
	if got := strings.Join(r.AttrStrings("deps"), ","); got != "//mapped:dep" {
		t.Errorf("override deps = %q", got)
	}
}

func TestResolveAmbiguous(t *testing.T) {
	l := &typescriptLang{}
	cfg := resolverConfig(t, nil)
	index := resolverIndex(l,
		struct{ pkg, name string; ext string }{"one", "same", ".ts"},
		struct{ pkg, name string; ext string }{"two", "same", ".tsx"},
	)
	l.Resolve(cfg, index, nil, rule.NewRule(projectKind, "app"), targetImports{imports: []string{"same"}}, label.New("", "app", "app"))
	if len(l.errors) != 1 || !strings.Contains(l.errors[0], "ambiguous") {
		t.Errorf("ambiguity = %v", l.errors)
	}
}

func TestUnionStrings(t *testing.T) {
	if got := unionStrings([]string{"b", "a", "a"}, []string{"c", "a"}); strings.Join(got, ",") != "a,b,c" {
		t.Errorf("union = %q", got)
	}
}

func TestErrorsAbortBeforeEmission(t *testing.T) {
	l := &typescriptLang{}
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

func TestResolveIgnore(t *testing.T) {
	l := &typescriptLang{}
	l.Before(context.Background())
	cfg := resolverConfig(t, nil)
	file, err := rule.LoadData("BUILD.bazel", "app", []byte("# gazelle:dx_ignore_import typescript external\n"))
	if err != nil {
		t.Fatal(err)
	}
	l.Configure(cfg, "app", file)
	r := rule.NewRule(projectKind, "app")
	l.Resolve(cfg, resolverIndex(l), nil, r, targetImports{imports: []string{"external"}}, label.New("", "app", "app"))
	if r.Attr("deps") != nil {
		t.Errorf("ignored import produced deps = %v", r.AttrStrings("deps"))
	}
	l.AfterResolvingDeps(context.Background())
}

func TestStaleIgnoreFails(t *testing.T) {
	l := &typescriptLang{}
	l.Before(context.Background())
	cfg := config.New()
	file, err := rule.LoadData("BUILD.bazel", "pkg", []byte("# gazelle:dx_ignore_import typescript stale\n"))
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
