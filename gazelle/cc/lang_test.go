package cc

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

func TestIgnoreWitnessPreservesUsedInheritedEntries(t *testing.T) {
	cfg := config.New()
	if got := CollectUsedIgnores(cfg); got != nil {
		t.Fatalf("absent: %v", got)
	}
	cfg.Exts[languageName] = "foreign"
	if got := CollectUsedIgnores(cfg); got != nil {
		t.Fatalf("foreign: %v", got)
	}
	delete(cfg.Exts, languageName)
	lang := &ccLang{}
	lang.Configure(cfg, "parent", &rule.File{Directives: []rule.Directive{{Key: "other"}, {Key: "dx_ignore_import", Value: "cc cc widget.h"}}})
	lang.Configure(cfg, "child", nil)
	entry := matchingIgnore(cfg, "widget.h")
	if entry == nil {
		t.Fatal("missing inherited ignore")
	}
	entry.used = true
	conf := cfg.Exts[languageName].(*ccConfig)
	conf.ignores = append(conf.ignores, nil, entry, &ignoreEntry{value: "Unused"})
	got := CollectUsedIgnores(cfg)
	if len(got) != 1 || got[0] != [2]string{"parent", "widget.h"} {
		t.Fatalf("witness: %v", got)
	}
	lang.AfterResolvingDeps(context.Background())
	lang.Configure(cfg, "parent", &rule.File{Directives: []rule.Directive{{Key: "dx_ignore_import", Value: "cc"}}})
	if len(lang.errors) != 1 {
		t.Fatal("malformed directive accepted")
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

func TestGeneratePackageLevelLibrary(t *testing.T) {
	regular := []string{"demo.cc", "helper.cc", "helper.h", "notes.txt", "demo_test.cc"}
	result := generateFixture(t, map[string]string{
		"pkg/demo/demo.cc":      "#include <string>\n#include \"stuff/thing.h\"\n",
		"pkg/demo/helper.cc":    "#include \"pkg/demo/helper.h\"\n",
		"pkg/demo/helper.h":     "#pragma once\n#include <string>\n",
		"pkg/demo/notes.txt":    "not a source\n",
		"pkg/demo/demo_test.cc": "#include \"stuff/only_by_test.h\"\nint main() { return 0; }\n",
	}, regular)
	if len(result.Gen) != 1 || len(result.Imports) != 1 {
		t.Fatalf("generated %d rules and %d import sets, want 1 each", len(result.Gen), len(result.Imports))
	}
	lib := result.Gen[0]
	if lib.Kind() != LibraryKind || lib.Name() != "demo" {
		t.Fatalf("library = %s(%s), want cc_library(demo)", lib.Kind(), lib.Name())
	}
	if got := strings.Join(lib.AttrStrings("srcs"), ","); got != "demo.cc,helper.cc" {
		t.Errorf("library srcs = %q, want demo.cc,helper.cc", got)
	}
	if got := strings.Join(lib.AttrStrings("hdrs"), ","); got != "helper.h" {
		t.Errorf("library hdrs = %q, want helper.h", got)
	}
	got := result.Imports[0].(targetImports)
	if strings.Join(got.imports, ",") != "helper.h,thing.h" {
		t.Errorf("library imports = %+v, want [helper.h thing.h]", got.imports)
	}
}

func TestGenerateTestSourcesExcluded(t *testing.T) {
	regular := []string{"demo.cc", "demo_test.cc"}
	result := generateFixture(t, map[string]string{
		"pkg/demo/demo.cc":      "#include <string>\n",
		"pkg/demo/demo_test.cc": "#include \"stuff/only_by_test.h\"\nint main() { return 0; }\n",
	}, regular)
	if len(result.Gen) != 1 || len(result.Imports) != 1 {
		t.Fatalf("generated %d rules and %d import sets, want 1 each", len(result.Gen), len(result.Imports))
	}
	if got := strings.Join(result.Gen[0].AttrStrings("srcs"), ","); got != "demo.cc" {
		t.Errorf("library srcs = %q, want demo.cc", got)
	}
	if got := result.Imports[0].(targetImports); len(got.imports) != 0 {
		t.Errorf("library imports = %+v, want empty", got.imports)
	}
}

func TestGenerateHeaderOnly(t *testing.T) {
	regular := []string{"helper.h"}
	result := generateFixture(t, map[string]string{
		"pkg/demo/helper.h": "#pragma once\n#include <string>\n",
	}, regular)
	if len(result.Gen) != 1 || len(result.Imports) != 1 {
		t.Fatalf("generated %d rules and %d import sets, want 1 each", len(result.Gen), len(result.Imports))
	}
	if attr := result.Gen[0].Attr("srcs"); attr != nil {
		t.Errorf("header-only srcs = %v, want absent", result.Gen[0].AttrStrings("srcs"))
	}
	if got := strings.Join(result.Gen[0].AttrStrings("hdrs"), ","); got != "helper.h" {
		t.Errorf("header-only hdrs = %q, want helper.h", got)
	}
}

func TestGenerateNoSourcesStaleSweep(t *testing.T) {
	root := t.TempDir()
	dir := filepath.Join(root, "pkg", "demo")
	if err := os.MkdirAll(dir, 0o755); err != nil {
		t.Fatal(err)
	}
	result := NewLanguage().GenerateRules(language.GenerateArgs{
		Config:       &config.Config{RepoRoot: root},
		Dir:          dir,
		Rel:          "pkg/demo",
		RegularFiles: []string{"notes.txt"},
	})
	if len(result.Gen) != 0 {
		t.Fatalf("generated %d rules, want 0", len(result.Gen))
	}
}

func TestGenerateMainFails(t *testing.T) {
	regular := []string{"demo.cc", "main.cc"}
	result := generateFixture(t, map[string]string{
		"pkg/demo/demo.cc": "#include <string>\n",
		"pkg/demo/main.cc": "#include <iostream>\nint main() { return 0; }\n",
	}, regular)
	if len(result.Gen) != 0 {
		t.Fatalf("generated %d rules for a main mix, want 0 with a recorded failure", len(result.Gen))
	}
}

func TestGenerateKindMismatchFails(t *testing.T) {
	root := t.TempDir()
	for name, content := range map[string]string{
		"pkg/demo/demo.cc": "#include <string>\n",
	} {
		writeFixture(t, root, name, content)
	}
	existing := &rule.File{}
	fg := rule.NewRule("filegroup", "demo")
	existing.Rules = append(existing.Rules, fg)
	result := NewLanguage().GenerateRules(language.GenerateArgs{
		Config:       &config.Config{RepoRoot: root},
		Dir:          filepath.Join(root, "pkg", "demo"),
		Rel:          "pkg/demo",
		RegularFiles: []string{"demo.cc"},
		File:         existing,
	})
	if len(result.Gen) != 0 {
		t.Fatalf("generated %d rules over a kind mismatch, want 0 with a recorded failure", len(result.Gen))
	}
}

func TestImportsIndexNonTestHeaders(t *testing.T) {
	lang := NewLanguage()
	lib := rule.NewRule(LibraryKind, "demo")
	lib.SetAttr("srcs", []string{"demo.cc", "helper.cc", "demo_test.cc", "notes.txt"})
	lib.SetAttr("hdrs", []string{"demo.h", "helper.h", "demo_test.h"})
	imports := lang.Imports(&config.Config{}, lib, nil)
	if len(imports) != 2 || imports[0].Lang != languageName || imports[0].Imp != "demo.h" || imports[1].Imp != "helper.h" {
		t.Errorf("imports = %+v, want demo.h+helper.h", imports)
	}
	if got := lang.Imports(&config.Config{}, rule.NewRule("filegroup", "demo"), nil); got != nil {
		t.Errorf("other-kind imports = %+v, want nil", got)
	}
	if got := lang.Imports(&config.Config{}, rule.NewRule(LibraryKind, "empty"), nil); got != nil {
		t.Errorf("headerless imports = %+v, want nil", got)
	}
}

func TestAfterResolvingDepsReportsFailures(t *testing.T) {
	l := &ccLang{}
	l.Before(context.Background())
	l.fail("cc: boom")
	defer func() {
		r := recover()
		if r == nil {
			t.Fatal("AfterResolvingDeps did not panic with recorded errors")
		}
		if msg, ok := r.(string); !ok || !strings.Contains(msg, "CC generation failed") {
			t.Fatalf("panic = %v, want CC generation failure", r)
		}
	}()
	l.AfterResolvingDeps(context.Background())
}

func TestAfterResolvingDepsClean(t *testing.T) {
	l := &ccLang{}
	l.Before(context.Background())
	l.AfterResolvingDeps(context.Background())
	if len(l.errors) != 0 {
		t.Errorf("errors = %v", l.errors)
	}
}

func TestLanguageMetadata(t *testing.T) {
	l := &ccLang{}
	if l.Name() != "cc" || len(l.Kinds()) != 1 || l.CheckFlags(flag.NewFlagSet("test", flag.ContinueOnError), config.New()) != nil {
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
	if len(loads) != 1 || loads[0].Name != "@renamed_dx//cc/rules:defs.bzl" || strings.Join(loads[0].Symbols, ",") != "cc_library" {
		t.Errorf("apparent loads = %+v", loads)
	}
	if defaults := l.Loads(); len(defaults) != 1 || defaults[0].Name != "@rules_dx//cc/rules:defs.bzl" {
		t.Errorf("default loads = %+v", defaults)
	}
	defaultApparent := l.ApparentLoads(func(string) string { return "" })
	if defaultApparent[0].Name != "@rules_dx//cc/rules:defs.bzl" {
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

func resolverIndex(lang *ccLang, entries ...struct {
	pkg  string
	name string
}) *resolve.RuleIndex {
	index := resolve.NewRuleIndex(func(r *rule.Rule, _ string) resolve.Resolver {
		if _, ok := ccKinds[r.Kind()]; ok {
			return lang
		}
		return nil
	})
	for _, entry := range entries {
		r := rule.NewRule(LibraryKind, entry.name)
		r.SetAttr("hdrs", []string{entry.name + ".h"})
		index.AddRule(config.New(), r, rule.EmptyFile(filepath.Join(entry.pkg, "BUILD.bazel"), entry.pkg))
	}
	index.Finish()
	return index
}

func TestResolveBranches(t *testing.T) {
	l := &ccLang{}
	index := resolverIndex(l,
		struct{ pkg, name string }{"lib/b", "b"},
		struct{ pkg, name string }{"lib/a", "a"},
	)
	cfg := resolverConfig(t, nil)
	r := rule.NewRule(LibraryKind, "app")
	l.Resolve(cfg, index, nil, r, targetImports{imports: []string{"b.h", "a.h"}}, label.New("", "app", "app"))
	if got := strings.Join(r.AttrStrings("deps"), ","); got != "//lib/a,//lib/b" {
		t.Errorf("resolved deps = %q", got)
	}

	self := rule.NewRule(LibraryKind, "a")
	l.Resolve(cfg, index, nil, self, targetImports{imports: []string{"a.h"}}, label.New("", "lib/a", "a"))
	if self.Attr("deps") != nil {
		t.Errorf("self dependency was emitted: %v", self.AttrStrings("deps"))
	}

	otherKind := rule.NewRule("filegroup", "other")
	l.Resolve(cfg, index, nil, otherKind, targetImports{imports: []string{"a.h"}}, label.New("", "app", "other"))
	if otherKind.Attr("deps") != nil {
		t.Errorf("non-library resolution emitted deps = %v", otherKind.AttrStrings("deps"))
	}

	hand := rule.NewRule(LibraryKind, "hand")
	hand.SetAttr("deps", []string{":kept"})
	l.Resolve(cfg, index, nil, hand, targetImports{imports: []string{"a.h"}}, label.New("", "app", "hand"))
	if got := strings.Join(hand.AttrStrings("deps"), ","); got != "//lib/a,:kept" {
		t.Errorf("merged deps = %q", got)
	}

	l.Resolve(cfg, index, nil, rule.NewRule(LibraryKind, "unknown"), targetImports{imports: []string{"unknown.h"}}, label.New("", "app", "unknown"))
	if len(l.errors) != 1 || !strings.Contains(l.errors[0], "unresolved import") {
		t.Errorf("unresolved errors = %v", l.errors)
	}
	l.Resolve(cfg, index, nil, r, nil, label.New("", "app", "app"))
}

func TestResolveOverride(t *testing.T) {
	l := &ccLang{}
	cfg := resolverConfig(t, []rule.Directive{{Key: "resolve", Value: "cc mapped.h //mapped:dep"}})
	r := rule.NewRule(LibraryKind, "app")
	l.Resolve(cfg, resolverIndex(l), nil, r, targetImports{imports: []string{"mapped.h"}}, label.New("", "app", "app"))
	if got := strings.Join(r.AttrStrings("deps"), ","); got != "//mapped:dep" {
		t.Errorf("override deps = %q", got)
	}
}

func TestResolveIgnore(t *testing.T) {
	l := &ccLang{}
	l.Before(context.Background())
	cfg := resolverConfig(t, nil)
	file, err := rule.LoadData("BUILD.bazel", "app", []byte("# gazelle:dx_ignore_import cc dropped.h\n"))
	if err != nil {
		t.Fatal(err)
	}
	l.Configure(cfg, "app", file)
	r := rule.NewRule(LibraryKind, "app")
	l.Resolve(cfg, resolverIndex(l), nil, r, targetImports{imports: []string{"dropped.h"}}, label.New("", "app", "app"))
	if r.Attr("deps") != nil {
		t.Errorf("ignored import emitted deps = %v", r.AttrStrings("deps"))
	}
	l.AfterResolvingDeps(context.Background())
	if len(l.errors) != 0 {
		t.Errorf("ignore errors = %v", l.errors)
	}
}

func TestResolveStaleIgnoreFails(t *testing.T) {
	l := &ccLang{}
	l.Before(context.Background())
	cfg := resolverConfig(t, nil)
	file, err := rule.LoadData("BUILD.bazel", "app", []byte("# gazelle:dx_ignore_import cc unused.h\n"))
	if err != nil {
		t.Fatal(err)
	}
	l.Configure(cfg, "app", file)
	defer func() {
		r := recover()
		if r == nil {
			t.Fatal("stale ignore did not panic")
		}
		if msg, ok := r.(string); !ok || !strings.Contains(msg, "stale # gazelle:dx_ignore_import") {
			t.Fatalf("panic = %v, want stale ignore failure", r)
		}
	}()
	l.AfterResolvingDeps(context.Background())
}

func TestResolveAmbiguousFails(t *testing.T) {
	l := &ccLang{}
	index := resolverIndex(l,
		struct{ pkg, name string }{"lib/one", "dup"},
		struct{ pkg, name string }{"lib/two", "dup"},
	)
	cfg := resolverConfig(t, nil)
	l.Resolve(cfg, index, nil, rule.NewRule(LibraryKind, "app"), targetImports{imports: []string{"dup.h"}}, label.New("", "app", "app"))
	if len(l.errors) != 1 || !strings.Contains(l.errors[0], "ambiguous import") {
		t.Errorf("ambiguous errors = %v", l.errors)
	}
}
