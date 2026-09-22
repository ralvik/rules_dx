package dispatch

import (
	"context"
	"encoding/json"
	"os"
	"path/filepath"
	"testing"

	"github.com/bazelbuild/bazel-gazelle/config"
	"github.com/bazelbuild/bazel-gazelle/language"
	"github.com/bazelbuild/bazel-gazelle/rule"
)

func testConfig() *config.Config {
	return &config.Config{
		RepoRoot:             "/repo",
		ValidBuildFileNames:  []string{"BUILD.bazel"},
		ModuleToApparentName: func(string) string { return "" },
	}
}

func mustLoad(t *testing.T, path, pkg, content string) *rule.File {
	t.Helper()
	f, err := rule.LoadData(path, pkg, []byte(content))
	if err != nil {
		t.Fatalf("LoadData: %v", err)
	}
	return f
}

func TestNameIsDispatch(t *testing.T) {
	if got := NewLanguage().Name(); got != languageName {
		t.Fatalf("Name() = %q, want %q", got, languageName)
	}
	if languageName == "rust" || languageName == "python" {
		t.Fatal("dispatch name must not collide with a first-party extension")
	}
}

func TestGenerateRulesRecordsOtherGenUnion(t *testing.T) {
	l := &dispatchLang{}
	l.Before(context.Background())
	// Recording disabled without the witness env: visits are dropped.
	cfg := testConfig()
	other := rule.NewRule("python_library", "handlers")
	res := l.GenerateRules(language.GenerateArgs{
		Config:   cfg,
		Dir:      "/repo/a",
		Rel:      "a",
		OtherGen: []*rule.Rule{other},
	})
	if len(res.Gen) != 0 {
		t.Fatalf("dispatch generates no rules, got %v", res.Gen)
	}
	if l.manifest != nil {
		t.Fatal("manifest must stay nil without DX_GENERATE_INTENDED")
	}
	if len(l.configs) != 1 {
		t.Fatalf("configs = %d, want 1", len(l.configs))
	}

	// With the witness env, OtherGen is retained as the union for new files.
	out := filepath.Join(t.TempDir(), "intended.json")
	t.Setenv(envIntendedManifest, out)
	t.Setenv(envGenerateMode, "default")
	l2 := &dispatchLang{}
	l2.Before(context.Background())
	if l2.manifest == nil {
		t.Fatal("manifest must load with DX_GENERATE_INTENDED set")
	}
	l2.GenerateRules(language.GenerateArgs{
		Config:   cfg,
		Dir:      t.TempDir(),
		Rel:      "a",
		OtherGen: []*rule.Rule{other},
	})
	if len(l2.manifest.visited) != 1 {
		t.Fatalf("visited = %d, want 1", len(l2.manifest.visited))
	}
	if len(l2.manifest.visited[0].gen) != 1 {
		t.Fatalf("union gen = %d, want 1", len(l2.manifest.visited[0].gen))
	}
}

func TestUnionLoadsCoverAllExtensions(t *testing.T) {
	loads := unionApparentLoads(func(string) string { return "" })
	// Every first-party extension contributes at least one load file.
	if len(loads) < 15 {
		t.Fatalf("union loads = %d entries, want >= 15 for 15 extensions", len(loads))
	}
	seen := map[string]bool{}
	for _, load := range loads {
		seen[load.Name] = true
	}
	for _, want := range []string{
		"//python/rules:defs.bzl",
		"//rust/rules:defs.bzl",
	} {
		found := false
		for name := range seen {
			if len(name) >= len(want) && name[len(name)-len(want):] == want {
				found = true
				break
			}
		}
		if !found {
			t.Errorf("union loads miss %s (have %v)", want, seen)
		}
	}
}

func TestAfterResolvingDepsWritesManifest(t *testing.T) {
	out := filepath.Join(t.TempDir(), "intended.json")
	t.Setenv(envIntendedManifest, out)
	t.Setenv(envGenerateMode, "default")
	l := &dispatchLang{}
	l.Before(context.Background())
	cfg := testConfig()
	gen := rule.NewRule("python_library", "handlers")
	gen.SetAttr("srcs", []string{"handlers.py"})
	l.GenerateRules(language.GenerateArgs{
		Config:   cfg,
		Dir:      t.TempDir(),
		Rel:      "a",
		OtherGen: []*rule.Rule{gen},
	})
	l.AfterResolvingDeps(context.Background())
	data, err := os.ReadFile(out)
	if err != nil {
		t.Fatalf("read manifest: %v", err)
	}
	var manifest intendedManifest
	if err := json.Unmarshal(data, &manifest); err != nil {
		t.Fatalf("decode manifest: %v", err)
	}
	if manifest.Mode != "default" {
		t.Errorf("mode = %q, want default", manifest.Mode)
	}
	if len(manifest.Files) != 1 {
		t.Fatalf("files = %d, want 1 new file", len(manifest.Files))
	}
	if len(manifest.Files[0].CreateContent) == 0 {
		t.Error("new file must carry complete content")
	}
}

func TestAfterResolvingDepsFailsClosedOnBadScope(t *testing.T) {
	out := filepath.Join(t.TempDir(), "intended.json")
	t.Setenv(envIntendedManifest, out)
	t.Setenv(envGenerateScope, `[{"element":"//a:one","dirs":["a"]}]`)
	l := &dispatchLang{}
	l.Before(context.Background())
	cfg := testConfig()
	gen := rule.NewRule("python_library", "handlers")
	gen.SetAttr("srcs", []string{"handlers.py"})
	// Visit outside the declared scope: emit must fail.
	l.GenerateRules(language.GenerateArgs{
		Config:   cfg,
		Dir:      t.TempDir(),
		Rel:      "elsewhere",
		OtherGen: []*rule.Rule{gen},
	})
	oldExit := exitProcess
	exited := false
	exitProcess = func(int) { exited = true }
	defer func() { exitProcess = oldExit }()
	l.AfterResolvingDeps(context.Background())
	if !exited {
		t.Fatal("scope mismatch must fail the run before emission")
	}
	if len(l.errors) == 0 {
		t.Fatal("scope mismatch must record an error")
	}
}

func TestCollectUsedIgnoresEmptyWithoutConfigs(t *testing.T) {
	l := &dispatchLang{}
	if got := l.collectUsedIgnores(); len(got) != 0 {
		t.Fatalf("ignores = %v, want empty", got)
	}
}
