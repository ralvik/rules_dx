package dispatch

import (
	"strings"
	"testing"

	"github.com/bazelbuild/bazel-gazelle/config"
	"github.com/bazelbuild/bazel-gazelle/language"
	"github.com/bazelbuild/bazel-gazelle/rule"
)

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
	// Simulate the framework's PostResolve merge: a sibling extension's
	// rule already merged into the shared file pointer.
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
	// Stabilize: witness until the bytes stop changing, then assert the
	// fixed point reads clean.
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
