package rust

import (
	"context"
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"unicode/utf8"

	"github.com/bazelbuild/bazel-gazelle/config"
	"github.com/bazelbuild/bazel-gazelle/language"
	"github.com/bazelbuild/bazel-gazelle/merger"
	"github.com/bazelbuild/bazel-gazelle/rule"
)

func testConfig() *config.Config {
	return &config.Config{
		RepoRoot:             "/repo",
		ValidBuildFileNames:  []string{"BUILD.bazel"},
		ModuleToApparentName: func(string) string { return "" },
	}
}

func testApparentLoads() func(func(string) string) []rule.LoadInfo {
	return (&rustLang{}).ApparentLoads
}

func mustLoad(t *testing.T, path, pkg, content string) *rule.File {
	t.Helper()
	f, err := rule.LoadData(path, pkg, []byte(content))
	if err != nil {
		t.Fatalf("LoadData: %v", err)
	}
	return f
}

func assertPanics(t *testing.T, message string, f func()) {
	t.Helper()
	defer func() {
		if recover() == nil {
			t.Errorf("%s: expected panic, got none", message)
		}
	}()
	f()
}

// applyEdits replays validated edits against the original bytes.
func applyEdits(t *testing.T, original []byte, edits []intendedEdit) []byte {
	t.Helper()
	var out []byte
	cursor := uint64(0)
	for _, edit := range edits {
		if edit.Start < cursor || edit.Start > edit.End || edit.End > uint64(len(original)) {
			t.Fatalf("edit %+v out of order or bounds for %d bytes", edit, len(original))
		}
		out = append(out, original[cursor:edit.Start]...)
		out = append(out, edit.Replacement...)
		cursor = edit.End
	}
	return append(out, original[cursor:]...)
}

// checkEditsValid enforces the validated edit form: ordered,
// non-overlapping, within bounds, boundary-aligned, no no-ops.
func checkEditsValid(t *testing.T, original []byte, edits []intendedEdit) {
	t.Helper()
	previous := uint64(0)
	for _, edit := range edits {
		if edit.Start < previous || edit.Start > edit.End || edit.End > uint64(len(original)) {
			t.Fatalf("edits not ordered within bounds: %+v", edits)
		}
		for _, at := range []uint64{edit.Start, edit.End} {
			if !utf8.ValidString(string(original[:at])) {
				t.Fatalf("edit boundary %d splits a UTF-8 sequence", at)
			}
		}
		if string(original[edit.Start:edit.End]) == string(edit.Replacement) {
			t.Fatalf("no-op edit %+v", edit)
		}
		previous = edit.Start
	}
}

// stableContent witnesses a seed until the bytes stop changing and returns
// the fixed point. FixLoads converges and Format is idempotent, so one pass
// normally suffices; the loop keeps tests independent of the exact
// canonical load rendering.
func stableContent(t *testing.T, path, pkg, seed string) string {
	t.Helper()
	rec := &manifestRecorder{apparentLoads: testApparentLoads()}
	content := []byte(seed)
	for i := 0; i < 3; i++ {
		f := mustLoad(t, path, pkg, string(content))
		file, changed := rec.witness(packageRecord{rel: pkg, file: f, cfg: testConfig()})
		if !changed {
			return string(content)
		}
		content = applyEdits(t, content, file.Edits)
	}
	t.Fatalf("witness did not stabilize for %s", path)
	return ""
}

func TestLoadManifestRecorderDisabled(t *testing.T) {
	t.Setenv(envIntendedManifest, "")
	if rec := loadManifestRecorder(); rec != nil {
		t.Fatalf("loadManifestRecorder without %s = %+v, want nil", envIntendedManifest, rec)
	}
}

func TestLoadManifestRecorderDefaults(t *testing.T) {
	t.Setenv(envIntendedManifest, filepath.Join(t.TempDir(), "intended.json"))
	t.Setenv(envGenerateMode, "")
	t.Setenv(envGenerateScope, "")
	rec := loadManifestRecorder()
	if rec == nil {
		t.Fatal("loadManifestRecorder = nil, want recorder")
	}
	if rec.mode != "default" {
		t.Errorf("mode = %q, want default", rec.mode)
	}
	if len(rec.scopes) != 1 || rec.scopes[0].Element != "//..." || len(rec.scopes[0].Dirs) != 1 || rec.scopes[0].Dirs[0] != "" {
		t.Errorf("scopes = %+v, want repo-wide default", rec.scopes)
	}
}

func TestLoadManifestRecorderExplicit(t *testing.T) {
	t.Setenv(envIntendedManifest, filepath.Join(t.TempDir(), "intended.json"))
	t.Setenv(envGenerateMode, "check")
	t.Setenv(envGenerateScope, `[{"element":"//pkg/...","dirs":["pkg"]}]`)
	rec := loadManifestRecorder()
	if rec.mode != "check" {
		t.Errorf("mode = %q, want check", rec.mode)
	}
	if len(rec.scopes) != 1 || rec.scopes[0].Element != "//pkg/..." || len(rec.scopes[0].Dirs) != 1 || rec.scopes[0].Dirs[0] != "pkg" {
		t.Errorf("scopes = %+v, want explicit scope", rec.scopes)
	}
}

func TestLoadManifestRecorderBadMode(t *testing.T) {
	t.Setenv(envIntendedManifest, filepath.Join(t.TempDir(), "intended.json"))
	t.Setenv(envGenerateMode, "print")
	assertPanics(t, "bad mode", func() { loadManifestRecorder() })
}

func TestLoadManifestRecorderMalformedScope(t *testing.T) {
	t.Setenv(envIntendedManifest, filepath.Join(t.TempDir(), "intended.json"))
	t.Setenv(envGenerateScope, `[{"element":`)
	assertPanics(t, "malformed scope", func() { loadManifestRecorder() })
}

func TestLoadManifestRecorderEmptyScopes(t *testing.T) {
	t.Setenv(envIntendedManifest, filepath.Join(t.TempDir(), "intended.json"))
	t.Setenv(envGenerateScope, `[]`)
	assertPanics(t, "empty scopes", func() { loadManifestRecorder() })
}

func TestScopeIndex(t *testing.T) {
	rec := &manifestRecorder{scopes: []scopeElement{
		{Element: "//a/b:target", Dirs: []string{"a/b"}},
		{Element: "//a/...", Dirs: []string{"a"}},
		{Element: "//...", Dirs: []string{""}},
	}}
	cases := []struct {
		rel  string
		want int
	}{
		{"a/b", 0},
		{"a/b/c", 0},
		{"a", 1},
		{"a/other", 1},
		{"", 2},
		{"unrelated", 2},
	}
	for _, c := range cases {
		if got := rec.scopeIndex(c.rel); got != c.want {
			t.Errorf("scopeIndex(%q) = %d, want %d", c.rel, got, c.want)
		}
	}
	narrow := &manifestRecorder{scopes: []scopeElement{{Element: "//a/...", Dirs: []string{"a"}}}}
	if got := narrow.scopeIndex("b"); got != -1 {
		t.Errorf("scopeIndex outside narrow scope = %d, want -1", got)
	}
}

func TestSplitLines(t *testing.T) {
	for _, c := range []string{"", "a", "a\n", "a\nb\nc", "a\nb", "é\nx\n"} {
		var rebuilt strings.Builder
		for _, line := range splitLines([]byte(c)) {
			rebuilt.Write(line)
		}
		if rebuilt.String() != c {
			t.Errorf("splitLines(%q) rebuilds to %q", c, rebuilt.String())
		}
	}
	if splitLines(nil) != nil {
		t.Errorf("splitLines(nil) = %q, want nil", splitLines(nil))
	}
}

func TestDiffLinesRoundtrip(t *testing.T) {
	cases := [][2]string{
		{"a\n", "a\n"},
		{"a\n", "b\n"},
		{"a\nb\n", "a\nx\nb\n"},
		{"x\na\nb\n", "a\nb\n"},
		{"", "load(\"x\")\n"},
		{"load(\"x\")\n", ""},
		{"", ""},
		{"no trailing\nnewline", "no trailing\nnewline\n"},
		{"café\nnaïve\n", "café\nnaive\n"},
		{"é\n", "ex\n"},
		{"line1\nline2\nline3\n", "line1\nline3\n"},
		{"a\n", "a\nb\nc\n"},
	}
	for _, c := range cases {
		edits := diffLines([]byte(c[0]), []byte(c[1]))
		if c[0] == c[1] {
			if len(edits) != 0 {
				t.Errorf("diffLines(%q, equal) = %+v, want none", c[0], edits)
			}
			continue
		}
		if len(edits) == 0 {
			t.Errorf("diffLines(%q, %q) yields no edits", c[0], c[1])
			continue
		}
		checkEditsValid(t, []byte(c[0]), edits)
		if got := string(applyEdits(t, []byte(c[0]), edits)); got != c[1] {
			t.Errorf("applyEdits(%q) = %q, want %q", c[0], got, c[1])
		}
	}
}

func TestReplacementKind(t *testing.T) {
	kindMap := map[string]config.MappedKind{
		"old_kind": {FromKind: "old_kind", KindName: "new_kind", KindLoad: "@x//:defs.bzl"},
		"new_kind": {FromKind: "new_kind", KindName: "final_kind", KindLoad: "@x//:defs.bzl"},
		"same":     {FromKind: "same", KindName: "same", KindLoad: "@x//:defs.bzl"},
	}
	if got := replacementKind(kindMap, "plain"); got != nil {
		t.Errorf("replacementKind(plain) = %+v, want nil", got)
	}
	if got := replacementKind(kindMap, "old_kind"); got == nil || got.KindName != "final_kind" {
		t.Errorf("replacementKind(old_kind) = %+v, want transitive final_kind", got)
	}
	if got := replacementKind(kindMap, "same"); got == nil || got.KindName != "same" {
		t.Errorf("replacementKind(same) = %+v, want same", got)
	}
	loop := map[string]config.MappedKind{
		"a": {FromKind: "a", KindName: "b", KindLoad: "@x//:defs.bzl"},
		"b": {FromKind: "b", KindName: "a", KindLoad: "@x//:defs.bzl"},
	}
	assertPanics(t, "kind map loop", func() { replacementKind(loop, "a") })
}

func TestAppendOrMergeKindMapping(t *testing.T) {
	loads := []rule.LoadInfo{{Name: "@x//:defs.bzl", Symbols: []string{"old_kind"}}}
	merged := appendOrMergeKindMapping(loads, config.MappedKind{KindName: "new_kind", KindLoad: "@x//:defs.bzl"})
	if len(merged) != 1 || len(merged[0].Symbols) != 2 || merged[0].Symbols[1] != "new_kind" {
		t.Fatalf("merged loads = %+v, want symbols appended", merged)
	}
	grown := appendOrMergeKindMapping(merged, config.MappedKind{KindName: "other", KindLoad: "@y//:defs.bzl"})
	if len(grown) != 2 || grown[1].Name != "@y//:defs.bzl" {
		t.Fatalf("grown loads = %+v, want appended load", grown)
	}
}

func TestApplyKindMappings(t *testing.T) {
	loads := []rule.LoadInfo{{Name: "@rules_dx//rust/rules:defs.bzl", Symbols: []string{"dx_rust_library"}}}
	plain := packageRecord{genKinds: []string{"dx_rust_library"}, cfg: testConfig()}
	if got := applyKindMappings(plain, loads); len(got) != 1 {
		t.Fatalf("applyKindMappings without map = %+v", got)
	}
	mappedCfg := testConfig()
	mappedCfg.KindMap = map[string]config.MappedKind{
		"dx_rust_library": {FromKind: "dx_rust_library", KindName: "custom_library", KindLoad: "@custom//:defs.bzl"},
	}
	mapped := packageRecord{genKinds: []string{"dx_rust_library"}, cfg: mappedCfg}
	if got := applyKindMappings(mapped, loads); len(got) != 2 || got[1].Name != "@custom//:defs.bzl" {
		t.Fatalf("applyKindMappings with map = %+v", got)
	}
}

func TestWitnessNewFile(t *testing.T) {
	rec := &manifestRecorder{apparentLoads: testApparentLoads()}
	cfg := testConfig()
	gen := []*rule.Rule{rule.NewRule("dx_rust_library", "demo")}
	file, changed := rec.witness(packageRecord{rel: "pkg", dir: "/repo/pkg", cfg: cfg, gen: gen})
	if !changed {
		t.Fatal("witness new file changed = false")
	}
	if file.Path != "pkg/BUILD.bazel" {
		t.Errorf("path = %q, want pkg/BUILD.bazel", file.Path)
	}
	if len(file.CreateContent) == 0 || len(file.Edits) != 0 || len(file.OriginalContent) != 0 {
		t.Errorf("new file record = %+v, want only create content", file)
	}
	if !strings.Contains(string(file.CreateContent), `name = "demo"`) {
		t.Errorf("create content = %q, want generated rule", file.CreateContent)
	}
	root, changed := rec.witness(packageRecord{rel: "", dir: "/repo", cfg: cfg, gen: gen})
	if !changed || root.Path != "BUILD.bazel" {
		t.Errorf("root new file = %+v, changed = %v, want path BUILD.bazel", root, changed)
	}
	empty, changed := rec.witness(packageRecord{rel: "pkg", dir: "/repo/pkg", cfg: cfg})
	if changed {
		t.Errorf("witness empty new file changed = true: %+v", empty)
	}
}

func TestWitnessModification(t *testing.T) {
	old := "load(\"@rules_dx//rust/rules:defs.bzl\", \"dx_rust_library\")\n\ndx_rust_library(\n    name = \"demo\",\n)\n"
	f := mustLoad(t, "/repo/pkg/BUILD.bazel", "pkg", old)
	rule.NewRule("dx_rust_library", "extra").Insert(f)
	rec := &manifestRecorder{apparentLoads: testApparentLoads()}
	file, changed := rec.witness(packageRecord{rel: "pkg", file: f, cfg: testConfig()})
	if !changed {
		t.Fatal("witness modification changed = false")
	}
	if file.Path != "pkg/BUILD.bazel" {
		t.Errorf("path = %q", file.Path)
	}
	if string(file.OriginalContent) != old {
		t.Errorf("original content = %q, want input bytes", file.OriginalContent)
	}
	checkEditsValid(t, []byte(old), file.Edits)
	if got := string(applyEdits(t, []byte(old), file.Edits)); got != string(f.Format()) {
		t.Errorf("edits apply to %q, want formatted %q", got, f.Format())
	}
}

func TestWitnessUnchanged(t *testing.T) {
	path := "/repo/pkg/BUILD.bazel"
	stable := stableContent(t, path, "pkg", "load(\"@rules_dx//rust/rules:defs.bzl\", \"dx_rust_library\")\n")
	f := mustLoad(t, path, "pkg", stable)
	rec := &manifestRecorder{apparentLoads: testApparentLoads()}
	if file, changed := rec.witness(packageRecord{rel: "pkg", file: f, cfg: testConfig()}); changed {
		t.Fatalf("witness stable file changed = true: %+v", file)
	}
}

func TestFixLoadsConverges(t *testing.T) {
	f := mustLoad(t, "/repo/pkg/BUILD.bazel", "pkg", "dx_rust_library(\n    name = \"demo\",\n)\n")
	loads := testApparentLoads()(func(string) string { return "" })
	merger.FixLoads(f, loads)
	first := string(f.Format())
	merger.FixLoads(f, loads)
	if got := string(f.Format()); got != first {
		t.Fatalf("second FixLoads changed bytes:\n%s\nvs\n%s", got, first)
	}
}

func TestKnownLoadsWithKindMap(t *testing.T) {
	cfg := testConfig()
	cfg.KindMap = map[string]config.MappedKind{
		"dx_rust_library": {FromKind: "dx_rust_library", KindName: "custom_library", KindLoad: "@custom//:defs.bzl"},
	}
	rec := &manifestRecorder{apparentLoads: testApparentLoads()}
	loads := rec.knownLoads(packageRecord{genKinds: []string{"dx_rust_library"}, cfg: cfg})
	found := false
	for _, load := range loads {
		if load.Name == "@custom//:defs.bzl" {
			found = true
		}
	}
	if !found {
		t.Fatalf("known loads = %+v, want mapped load", loads)
	}
}

func readIntended(t *testing.T, path string) intendedManifest {
	t.Helper()
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("ReadFile: %v", err)
	}
	var manifest intendedManifest
	if err := json.Unmarshal(data, &manifest); err != nil {
		t.Fatalf("Unmarshal: %v", err)
	}
	return manifest
}

func TestEmitEndToEnd(t *testing.T) {
	out := filepath.Join(t.TempDir(), "intended.json")
	rec := &manifestRecorder{
		outPath:       out,
		mode:          "check",
		scopes:        []scopeElement{{Element: "//...", Dirs: []string{""}}},
		apparentLoads: testApparentLoads(),
	}
	cfg := testConfig()
	old := "load(\"@rules_dx//rust/rules:defs.bzl\", \"dx_rust_library\")\n\ndx_rust_library(\n    name = \"demo\",\n)\n"
	changed := mustLoad(t, "/repo/pkg/BUILD.bazel", "pkg", old)
	rule.NewRule("dx_rust_library", "extra").Insert(changed)
	steadyPath := "/repo/steady/BUILD.bazel"
	steady := mustLoad(t, steadyPath, "steady", stableContent(t, steadyPath, "steady", "load(\"@rules_dx//rust/rules:defs.bzl\", \"dx_rust_library\")\n"))
	rec.record(language.GenerateArgs{Rel: "pkg", Dir: "/repo/pkg", File: changed, Config: cfg}, language.GenerateResult{})
	rec.record(language.GenerateArgs{Rel: "steady", Dir: "/repo/steady", File: steady, Config: cfg}, language.GenerateResult{})
	rec.record(language.GenerateArgs{
		Rel:    "new",
		Dir:    "/repo/new",
		Config: cfg,
	}, language.GenerateResult{Gen: []*rule.Rule{rule.NewRule("dx_rust_library", "fresh")}})
	rec.emit([]*ignoreEntry{
		{value: "used_import", path: "pkg", used: true},
		{value: "stale_import", path: "pkg", used: false},
	})
	manifest := readIntended(t, out)
	if manifest.SchemaMajor != 1 || manifest.SchemaMinor != 0 || manifest.Mode != "check" {
		t.Errorf("header = %+v", manifest)
	}
	if len(manifest.Scopes) != 1 || manifest.Scopes[0].Value != "//..." || !manifest.Scopes[0].ResultsComplete {
		t.Errorf("scopes = %+v", manifest.Scopes)
	}
	if len(manifest.Files) != 2 {
		t.Fatalf("files = %+v, want changed + new only", manifest.Files)
	}
	if manifest.Files[0].Path != "pkg/BUILD.bazel" || len(manifest.Files[0].Edits) == 0 {
		t.Errorf("first file = %+v", manifest.Files[0])
	}
	if string(manifest.Files[0].OriginalContent) != old {
		t.Errorf("original content = %q", manifest.Files[0].OriginalContent)
	}
	checkEditsValid(t, manifest.Files[0].OriginalContent, manifest.Files[0].Edits)
	if got := string(applyEdits(t, manifest.Files[0].OriginalContent, manifest.Files[0].Edits)); got != string(changed.Format()) {
		t.Errorf("edits apply to %q, want %q", got, changed.Format())
	}
	if manifest.Files[1].Path != "new/BUILD.bazel" || !strings.Contains(string(manifest.Files[1].CreateContent), `name = "fresh"`) {
		t.Errorf("second file = %+v", manifest.Files[1])
	}
	if len(manifest.IgnoredImports) != 1 {
		t.Fatalf("ignored = %+v, want only the used import", manifest.IgnoredImports)
	}
	ignored := manifest.IgnoredImports[0]
	if ignored.Import != "used_import" || ignored.Path != "pkg" || ignored.Language != languageName || ignored.ScopeIndex != 0 {
		t.Errorf("ignored import = %+v", ignored)
	}
}

func TestEmitSortsIgnoredImports(t *testing.T) {
	out := filepath.Join(t.TempDir(), "intended.json")
	rec := &manifestRecorder{
		outPath:       out,
		mode:          "default",
		scopes:        []scopeElement{{Element: "//...", Dirs: []string{""}}},
		apparentLoads: testApparentLoads(),
	}
	rec.emit([]*ignoreEntry{
		{value: "zebra", path: "b", used: true},
		{value: "apple", path: "b", used: true},
		{value: "mango", path: "a", used: true},
	})
	manifest := readIntended(t, out)
	want := [][2]string{{"a", "mango"}, {"b", "apple"}, {"b", "zebra"}}
	if len(manifest.IgnoredImports) != len(want) {
		t.Fatalf("ignored = %+v, want %v", manifest.IgnoredImports, want)
	}
	for i, w := range want {
		got := manifest.IgnoredImports[i]
		if got.Path != w[0] || got.Import != w[1] || got.Language != languageName || got.ScopeIndex != 0 {
			t.Errorf("ignored[%d] = %+v, want path %q import %q", i, got, w[0], w[1])
		}
	}
}

func TestEmitScopeMismatch(t *testing.T) {
	narrow := func() *manifestRecorder {
		return &manifestRecorder{
			outPath:       filepath.Join(t.TempDir(), "intended.json"),
			mode:          "default",
			scopes:        []scopeElement{{Element: "//a/...", Dirs: []string{"a"}}},
			apparentLoads: testApparentLoads(),
		}
	}
	outside := narrow()
	outside.record(language.GenerateArgs{Rel: "b", Dir: "/repo/b", Config: testConfig()}, language.GenerateResult{})
	assertPanics(t, "file scope mismatch", func() { outside.emit(nil) })

	matched := narrow()
	assertPanics(t, "ignore scope mismatch", func() {
		matched.emit([]*ignoreEntry{{value: "x", path: "b", used: true}})
	})
}

func TestEmitWriteError(t *testing.T) {
	rec := &manifestRecorder{
		outPath:       filepath.Join(t.TempDir(), "missing", "intended.json"),
		mode:          "default",
		scopes:        []scopeElement{{Element: "//...", Dirs: []string{""}}},
		apparentLoads: testApparentLoads(),
	}
	assertPanics(t, "write error", func() { rec.emit(nil) })
}

func TestAfterResolvingDepsEmits(t *testing.T) {
	out := filepath.Join(t.TempDir(), "intended.json")
	l := &rustLang{}
	l.manifest = &manifestRecorder{
		outPath:       out,
		mode:          "default",
		scopes:        []scopeElement{{Element: "//...", Dirs: []string{""}}},
		apparentLoads: l.ApparentLoads,
	}
	l.AfterResolvingDeps(context.Background())
	manifest := readIntended(t, out)
	if manifest.Mode != "default" || len(manifest.Files) != 0 || len(manifest.IgnoredImports) != 0 {
		t.Errorf("manifest = %+v", manifest)
	}
}

func TestBeforeInitializesRecorder(t *testing.T) {
	l := &rustLang{}
	l.Before(context.Background())
	if l.manifest != nil {
		t.Fatal("Before without env sets recorder")
	}
	out := filepath.Join(t.TempDir(), "intended.json")
	t.Setenv(envIntendedManifest, out)
	l.Before(context.Background())
	if l.manifest == nil {
		t.Fatal("Before with env leaves recorder nil")
	}
	l.GenerateRules(language.GenerateArgs{Config: testConfig(), Dir: "/missing", Rel: "gone"})
	if len(l.manifest.visited) != 1 || l.manifest.visited[0].rel != "gone" {
		t.Fatalf("visited = %+v, want one recorded visit", l.manifest.visited)
	}
}
