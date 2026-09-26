package dispatch

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/bazelbuild/bazel-gazelle/config"
	"github.com/bazelbuild/bazel-gazelle/language"
	"github.com/bazelbuild/bazel-gazelle/rule"
)

func TestManifestRecorderEnvironment(t *testing.T) {
	t.Setenv(envIntendedManifest, "")
	if rec, err := loadManifestRecorder(); rec != nil || err != nil {
		t.Fatalf("disabled recorder: %v, %v", rec, err)
	}
	t.Setenv(envIntendedManifest, filepath.Join(t.TempDir(), "intended.json"))
	t.Setenv(envGenerateMode, "")
	t.Setenv(envGenerateScope, "")
	rec, err := loadManifestRecorder()
	if err != nil || rec.mode != "default" || len(rec.scopes) != 1 || rec.scopes[0].Element != "//..." {
		t.Fatalf("default recorder: %+v, %v", rec, err)
	}
	t.Setenv(envGenerateMode, "invalid")
	if _, err := loadManifestRecorder(); err == nil {
		t.Fatal("invalid mode accepted")
	}
	t.Setenv(envGenerateMode, "check")
	for _, raw := range []string{"{", "[]", "null"} {
		t.Setenv(envGenerateScope, raw)
		if _, err := loadManifestRecorder(); err == nil {
			t.Fatalf("invalid scopes accepted: %s", raw)
		}
	}
	t.Setenv(envGenerateScope, `[{"element":"//a/...","dirs":["a"]}]`)
	rec, err = loadManifestRecorder()
	if err != nil || rec.mode != "check" || rec.scopeIndex("a/b") != 0 || rec.scopeIndex("b") != -1 {
		t.Fatalf("scoped recorder: %+v, %v", rec, err)
	}
}

func TestManifestEmissionPreservesScopeAndDeduplicatesIgnores(t *testing.T) {
	dir := t.TempDir()
	rec := &manifestRecorder{outPath: filepath.Join(dir, "intended.json"), mode: "check", scopes: []scopeElement{{Element: "//...", Dirs: []string{""}}}, unionLoads: unionApparentLoads}
	gen := rule.NewRule("python_library", "demo")
	gen.SetAttr("srcs", []string{"demo.py"})
	rec.record(genArgs(testConfig(), dir, "pkg", nil, []*rule.Rule{nil, gen, gen}), genRes([]*rule.Rule{nil, gen}))
	if len(rec.visited[0].gen) != 1 {
		t.Fatal("duplicate rules retained")
	}
	rec.record(genArgs(testConfig(), dir, "empty", nil, nil), genRes(nil))
	ignores := []collectedIgnore{{"pkg/z.py", "python", "z"}, {"pkg/a.py", "rust", "z"}, {"pkg/a.py", "python", "z"}, {"pkg/a.py", "python", "a"}, {"pkg/a.py", "python", "a"}}
	if err := rec.emit(ignores); err != nil {
		t.Fatal(err)
	}
	data, err := os.ReadFile(rec.outPath)
	if err != nil {
		t.Fatal(err)
	}
	var manifest intendedManifest
	if err := json.Unmarshal(data, &manifest); err != nil {
		t.Fatal(err)
	}
	if manifest.SchemaMajor != 1 || manifest.Mode != "check" || len(manifest.Files) != 1 || manifest.Files[0].Path != "pkg/BUILD.bazel" || manifest.Files[0].ScopeIndex != 0 || !manifest.Scopes[0].ResultsComplete {
		t.Fatalf("manifest: %+v", manifest)
	}
	if len(manifest.IgnoredImports) != 4 || manifest.IgnoredImports[0].Import != "a" || manifest.IgnoredImports[1].Language != "python" || manifest.IgnoredImports[2].Language != "rust" || manifest.IgnoredImports[3].Path != "pkg/z.py" {
		t.Fatalf("ignores: %+v", manifest.IgnoredImports)
	}
	rec.outPath = dir
	if err := rec.emit(nil); err == nil || !strings.Contains(err.Error(), "cannot write") {
		t.Fatalf("write error: %v", err)
	}
	rec.scopes = []scopeElement{{Element: "//other/...", Dirs: []string{"other"}}}
	if err := rec.emit(nil); err == nil || !strings.Contains(err.Error(), "matches no") {
		t.Fatalf("scope error: %v", err)
	}
	rec.visited = nil
	if err := rec.emit(ignores); err == nil || !strings.Contains(err.Error(), "ignored import") {
		t.Fatalf("ignore scope error: %v", err)
	}
}

func genArgs(cfg *config.Config, dir, rel string, f *rule.File, other []*rule.Rule) language.GenerateArgs {
	return language.GenerateArgs{
		Config:   cfg,
		Dir:      dir,
		Rel:      rel,
		File:     f,
		OtherGen: other,
	}
}

func genRes(rules []*rule.Rule) language.GenerateResult {
	res := language.GenerateResult{Gen: rules}
	for range rules {
		res.Imports = append(res.Imports, struct{}{})
	}
	return res
}

func applyFileEdits(t *testing.T, original []byte, file intendedFile) []byte {
	t.Helper()
	var out []byte
	cursor := uint64(0)
	for _, edit := range file.Edits {
		if edit.Start < cursor || edit.Start > edit.End || edit.End > uint64(len(original)) {
			t.Fatalf("edit %+v out of order or bounds", edit)
		}
		out = append(out, original[cursor:edit.Start]...)
		out = append(out, edit.Replacement...)
		cursor = edit.End
	}
	return append(out, original[cursor:]...)
}

func TestWitnessNewFileUsesUnionGen(t *testing.T) {
	rec := &manifestRecorder{unionLoads: unionApparentLoads}
	dir := t.TempDir()
	gen := rule.NewRule("python_library", "handlers")
	gen.SetAttr("srcs", []string{"handlers.py"})
	other := rule.NewRule("rust_library", "native")
	other.SetAttr("srcs", []string{"lib.rs"})
	cfg := testConfig()
	rec.record(
		genArgs(cfg, dir, "mixed", nil, []*rule.Rule{other}),
		genRes([]*rule.Rule{gen}),
	)
	if len(rec.visited) != 1 {
		t.Fatalf("visited = %d, want 1", len(rec.visited))
	}
	file, changed, err := rec.witness(rec.visited[0])
	if err != nil {
		t.Fatalf("witness: %v", err)
	}
	if !changed {
		t.Fatal("new mixed file must read as changed")
	}
	content := string(file.CreateContent)
	if !strings.Contains(content, "handlers") || !strings.Contains(content, "native") {
		t.Fatalf("union content must carry both rules, got:\n%s", content)
	}
	if file.Path != "mixed/BUILD.bazel" {
		t.Fatalf("path = %q, want mixed/BUILD.bazel", file.Path)
	}
}

func TestWitnessExistingFileSeesMergedState(t *testing.T) {
	rec := &manifestRecorder{unionLoads: unionApparentLoads}
	seed := "load(\"@rules_dx//python/rules:defs.bzl\", \"python_library\")\n\npython_library(\n    name = \"handlers\",\n    srcs = [\"handlers.py\"],\n)\n"
	f := mustLoad(t, "BUILD.bazel", "a", seed)
	cfg := testConfig()
	extra := rule.NewRule("rust_library", "native")
	extra.SetAttr("srcs", []string{"lib.rs"})
	extra.Insert(f)
	rec.record(genArgs(cfg, "/repo", "a", f, nil), genRes(nil))
	file, changed, err := rec.witness(rec.visited[0])
	if err != nil {
		t.Fatalf("witness: %v", err)
	}
	if !changed {
		t.Fatal("merged sibling rule must read as changed")
	}
	if len(file.Edits) == 0 {
		t.Fatal("merged change must produce edits")
	}
	if len(file.OriginalContent) == 0 {
		t.Fatal("modification must carry original content")
	}
}

func TestWitnessCleanFileIsUnchanged(t *testing.T) {
	seed := "load(\"@rules_dx//python/rules:defs.bzl\", \"python_library\")\n\npython_library(\n    name = \"handlers\",\n    srcs = [\"handlers.py\"],\n)\n"
	content := seed
	for i := 0; i < 3; i++ {
		f := mustLoad(t, "BUILD.bazel", "a", content)
		r := &manifestRecorder{unionLoads: unionApparentLoads}
		r.record(genArgs(testConfig(), "/repo", "a", f, nil), genRes(nil))
		file, changed, err := r.witness(r.visited[0])
		if err != nil {
			t.Fatalf("witness: %v", err)
		}
		if !changed {
			return
		}
		content = string(applyFileEdits(t, []byte(content), file))
	}
	t.Fatal("witness did not converge")
}

func TestScopeIndexLongestDirWins(t *testing.T) {
	rec := &manifestRecorder{scopes: []scopeElement{
		{Element: "//...", Dirs: []string{""}},
		{Element: "//a:one", Dirs: []string{"a"}},
	}}
	if got := rec.scopeIndex("a/b"); got != 1 {
		t.Fatalf("scopeIndex(a/b) = %d, want 1", got)
	}
	if got := rec.scopeIndex("other"); got != 0 {
		t.Fatalf("scopeIndex(other) = %d, want 0", got)
	}
}

func TestDiffLinesProduceValidatedEdits(t *testing.T) {
	original := []byte("line1\nline2\nline3\n")
	intended := []byte("line1\nchanged\nline3\n")
	edits := diffLines(original, intended)
	if len(edits) == 0 {
		t.Fatal("diff must produce edits")
	}
	out := applyFileEdits(t, original, intendedFile{Edits: edits})
	if string(out) != string(intended) {
		t.Fatalf("replay = %q, want %q", out, intended)
	}
}

func TestKindMappingsFollowChainsAndRejectCycles(t *testing.T) {
	cfg := testConfig()
	cfg.KindMap = map[string]config.MappedKind{
		"python_library": {KindName: "middle", KindLoad: "//:middle.bzl"},
		"middle":         {KindName: "final", KindLoad: "//:final.bzl"},
	}
	loads := []rule.LoadInfo{{Name: "//:final.bzl", Symbols: []string{"existing"}}}
	rec := packageRecord{cfg: cfg, genKinds: []string{"python_library"}}
	got, err := applyKindMappings(rec, loads)
	if err != nil || len(got) != 1 || strings.Join(got[0].Symbols, ",") != "existing,final" {
		t.Fatalf("mapping: %v, %v", got, err)
	}
	got, err = applyKindMappings(rec, nil)
	if err != nil || len(got) != 1 || got[0].Name != "//:final.bzl" {
		t.Fatalf("new load: %v, %v", got, err)
	}
	if mapped, err := replacementKind(map[string]config.MappedKind{"self": {KindName: "self", KindLoad: "//:self.bzl"}}, "self"); err != nil || mapped.KindName != "self" {
		t.Fatalf("self mapping: %v, %v", mapped, err)
	}
	cfg.KindMap["final"] = config.MappedKind{KindName: "middle", KindLoad: "//:middle.bzl"}
	if _, err := applyKindMappings(rec, loads); err == nil {
		t.Fatal("cycle accepted")
	}
	gen := rule.NewRule("python_library", "demo")
	rec.gen = []*rule.Rule{gen}
	rec.dir = t.TempDir()
	recorder := &manifestRecorder{}
	if _, _, err := recorder.witness(rec); err == nil {
		t.Fatal("new-file cycle accepted")
	}
	rec.file = rule.EmptyFile(filepath.Join(rec.dir, "BUILD.bazel"), "")
	if _, _, err := recorder.witness(rec); err == nil {
		t.Fatal("existing-file cycle accepted")
	}
}

func TestDiffEditsReplayInsertionsDeletionsAndUnterminatedLines(t *testing.T) {
	for _, pair := range [][2]string{{"", "new"}, {"old", ""}, {"one\ntwo", "one\nthree"}, {"one\n", "one\ntwo\n"}} {
		edits := diffLines([]byte(pair[0]), []byte(pair[1]))
		got := applyFileEdits(t, []byte(pair[0]), intendedFile{Edits: edits})
		if string(got) != pair[1] {
			t.Fatalf("replay %q -> %q: %q", pair[0], pair[1], got)
		}
		for _, edit := range edits {
			if edit.Replacement == nil {
				t.Fatal("nil replacement serializes as null")
			}
		}
	}
	if got := manifestPath("", "BUILD.bazel"); got != "BUILD.bazel" {
		t.Fatalf("root path: %s", got)
	}
}
