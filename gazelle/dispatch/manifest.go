package dispatch

import (
	"bytes"
	"encoding/json"
	"fmt"
	"os"
	"path"
	"path/filepath"
	"sort"
	"strings"

	"github.com/bazelbuild/bazel-gazelle/config"
	"github.com/bazelbuild/bazel-gazelle/language"
	"github.com/bazelbuild/bazel-gazelle/merger"
	"github.com/bazelbuild/bazel-gazelle/rule"
	difflib "github.com/pmezard/go-difflib/difflib"
)

const (
	intendedManifestSchemaMajor = 1
	intendedManifestSchemaMinor = 0

	envIntendedManifest = "DX_GENERATE_INTENDED"
	envGenerateScope    = "DX_GENERATE_SCOPE"
	envGenerateMode     = "DX_GENERATE_MODE"
)

type scopeElement struct {
	Element string   `json:"element"`
	Dirs    []string `json:"dirs"`
}

type intendedEdit struct {
	Start       uint64 `json:"start_byte"`
	End         uint64 `json:"end_byte"`
	Replacement []byte `json:"replacement"`
}

type intendedFile struct {
	Path            string         `json:"path"`
	ScopeIndex      int            `json:"scope_index"`
	CreateContent   []byte         `json:"create_content,omitempty"`
	OriginalContent []byte         `json:"original_content,omitempty"`
	Edits           []intendedEdit `json:"edits,omitempty"`
}

type intendedIgnoredImport struct {
	Path       string `json:"path"`
	Language   string `json:"language"`
	Import     string `json:"import"`
	ScopeIndex int    `json:"scope_index"`
}

type intendedScope struct {
	Value           string `json:"value"`
	ResultsComplete bool   `json:"results_complete"`
}

type intendedManifest struct {
	SchemaMajor    int                     `json:"schema_major"`
	SchemaMinor    int                     `json:"schema_minor"`
	Mode           string                  `json:"mode"`
	Scopes         []intendedScope         `json:"scopes"`
	Files          []intendedFile          `json:"files"`
	IgnoredImports []intendedIgnoredImport `json:"ignored_imports"`
}

type collectedIgnore struct {
	path     string
	language string
	value    string
}

type packageRecord struct {
	rel      string
	dir      string
	file     *rule.File
	gen      []*rule.Rule
	genKinds []string
	oldKinds []string
	cfg      *config.Config
}

type manifestRecorder struct {
	outPath    string
	mode       string
	scopes     []scopeElement
	unionLoads func(func(string) string) []rule.LoadInfo
	visited    []packageRecord
}

func loadManifestRecorder() (*manifestRecorder, error) {
	outPath := os.Getenv(envIntendedManifest)
	if outPath == "" {
		return nil, nil
	}
	mode := os.Getenv(envGenerateMode)
	if mode == "" {
		mode = "default"
	}
	if mode != "check" && mode != "default" {
		return nil, fmt.Errorf("dispatch: %s must be \"check\" or \"default\", got %q", envGenerateMode, mode)
	}
	scopes := []scopeElement{{Element: "//...", Dirs: []string{""}}}
	if raw := os.Getenv(envGenerateScope); raw != "" {
		if err := json.Unmarshal([]byte(raw), &scopes); err != nil {
			return nil, fmt.Errorf("dispatch: malformed %s: %v", envGenerateScope, err)
		}
		if len(scopes) == 0 {
			return nil, fmt.Errorf("dispatch: %s must list at least one scope element", envGenerateScope)
		}
	}
	return &manifestRecorder{outPath: outPath, mode: mode, scopes: scopes}, nil
}

func (r *manifestRecorder) record(args language.GenerateArgs, res language.GenerateResult) {
	rec := packageRecord{
		rel:  args.Rel,
		dir:  args.Dir,
		file: args.File,
		cfg:  args.Config,
	}
	seen := map[*rule.Rule]bool{}
	for _, g := range args.OtherGen {
		if g == nil || seen[g] {
			continue
		}
		seen[g] = true
		rec.gen = append(rec.gen, g)
	}
	for _, g := range res.Gen {
		if g == nil || seen[g] {
			continue
		}
		seen[g] = true
		rec.gen = append(rec.gen, g)
	}
	for _, g := range rec.gen {
		rec.genKinds = append(rec.genKinds, g.Kind())
	}
	if args.File != nil {
		for _, old := range args.File.Rules {
			rec.oldKinds = append(rec.oldKinds, old.Kind())
		}
	}
	r.visited = append(r.visited, rec)
}

func (r *manifestRecorder) scopeIndex(rel string) int {
	best := -1
	bestLen := -1
	for i, scope := range r.scopes {
		for _, dir := range scope.Dirs {
			length := -1
			switch {
			case dir == "":
				length = 0
			case rel == dir:
				length = len(dir)
			case strings.HasPrefix(rel, dir+"/"):
				length = len(dir)
			}
			if length > bestLen {
				bestLen = length
				best = i
			}
		}
	}
	return best
}

func (r *manifestRecorder) emit(ignores []collectedIgnore) error {
	manifest := intendedManifest{
		SchemaMajor:    intendedManifestSchemaMajor,
		SchemaMinor:    intendedManifestSchemaMinor,
		Mode:           r.mode,
		Scopes:         []intendedScope{},
		Files:          []intendedFile{},
		IgnoredImports: []intendedIgnoredImport{},
	}
	for _, scope := range r.scopes {
		manifest.Scopes = append(manifest.Scopes, intendedScope{
			Value:           scope.Element,
			ResultsComplete: true,
		})
	}
	for _, rec := range r.visited {
		index := r.scopeIndex(rec.rel)
		if index < 0 {
			return fmt.Errorf("dispatch: package %q matches no %s scope element", rec.rel, envGenerateScope)
		}
		file, changed, err := r.witness(rec)
		if err != nil {
			return err
		}
		if !changed {
			continue
		}
		file.ScopeIndex = index
		manifest.Files = append(manifest.Files, file)
	}
	seenIgnore := map[intendedIgnoredImport]bool{}
	for _, ignore := range ignores {
		index := r.scopeIndex(ignore.path)
		if index < 0 {
			return fmt.Errorf("dispatch: ignored import %q matches no %s scope element", ignore.value, envGenerateScope)
		}
		entry := intendedIgnoredImport{
			Path:       ignore.path,
			Language:   ignore.language,
			Import:     ignore.value,
			ScopeIndex: index,
		}
		if seenIgnore[entry] {
			continue
		}
		seenIgnore[entry] = true
		manifest.IgnoredImports = append(manifest.IgnoredImports, entry)
	}
	sort.SliceStable(manifest.IgnoredImports, func(i, j int) bool {
		a, b := manifest.IgnoredImports[i], manifest.IgnoredImports[j]
		if a.Path != b.Path {
			return a.Path < b.Path
		}
		if a.Language != b.Language {
			return a.Language < b.Language
		}
		return a.Import < b.Import
	})
	data, err := json.Marshal(manifest)
	if err != nil {
		return fmt.Errorf("dispatch: cannot encode intended manifest: %v", err) // LCOV_EXCL_LINE - reason: unreachable encode, issue: 1055, policy: docs/cli/commands/build-test-coverage.md
	}
	if err := os.WriteFile(r.outPath, data, 0o600); err != nil {
		return fmt.Errorf("dispatch: cannot write intended manifest: %v", err)
	}
	return nil
}

func (r *manifestRecorder) witness(rec packageRecord) (intendedFile, bool, error) {
	var file intendedFile
	if rec.file == nil {
		if len(rec.gen) == 0 {
			return file, false, nil
		}
		built := rule.EmptyFile(filepath.Join(rec.dir, rec.cfg.DefaultBuildFileName()), rec.rel)
		for _, g := range rec.gen {
			g.Insert(built)
		}
		loads, err := r.knownLoads(rec)
		if err != nil {
			return file, false, err
		}
		merger.FixLoads(built, loads)
		file.Path = manifestPath(rec.rel, rec.cfg.DefaultBuildFileName())
		file.CreateContent = built.Format()
		return file, true, nil
	}
	original := rec.file.Content
	loads, err := r.knownLoads(rec)
	if err != nil {
		return file, false, err
	}
	merger.FixLoads(rec.file, loads)
	intended := rec.file.Format()
	if bytes.Equal(original, intended) {
		return file, false, nil
	}
	file.Path = manifestPath(rec.rel, filepath.Base(rec.file.Path))
	file.OriginalContent = original
	file.Edits = diffLines(original, intended)
	if len(file.Edits) == 0 {
		return file, false, fmt.Errorf("dispatch: changed package %q produced no edits", rec.rel) // LCOV_EXCL_LINE - reason: unreachable encode, issue: 1055, policy: docs/cli/commands/build-test-coverage.md
	}
	return file, true, nil
}

func (r *manifestRecorder) knownLoads(rec packageRecord) ([]rule.LoadInfo, error) {
	var loads []rule.LoadInfo
	if r.unionLoads != nil {
		loads = r.unionLoads(rec.cfg.ModuleToApparentName)
	}
	return applyKindMappings(rec, loads)
}

func manifestPath(rel, base string) string {
	if rel == "" {
		return base
	}
	return path.Join(rel, base)
}

func applyKindMappings(rec packageRecord, loads []rule.LoadInfo) ([]rule.LoadInfo, error) {
	var mapped []config.MappedKind
	for _, kind := range append(append([]string{}, rec.genKinds...), rec.oldKinds...) {
		repl, err := replacementKind(rec.cfg.KindMap, kind)
		if err != nil {
			return nil, err
		}
		if repl != nil {
			mapped = append(mapped, *repl)
		}
	}
	if len(mapped) == 0 {
		return loads, nil
	}
	merged := make([]rule.LoadInfo, len(loads))
	copy(merged, loads)
	for _, repl := range mapped {
		merged = appendOrMergeKindMapping(merged, repl)
	}
	return merged, nil
}

func appendOrMergeKindMapping(loads []rule.LoadInfo, repl config.MappedKind) []rule.LoadInfo {
	for i, load := range loads {
		if load.Name == repl.KindLoad {
			loads[i].Symbols = append(loads[i].Symbols, repl.KindName)
			return loads
		}
	}
	return append(loads, rule.LoadInfo{
		Name:    repl.KindLoad,
		Symbols: []string{repl.KindName},
	})
}

func replacementKind(kindMap map[string]config.MappedKind, kind string) (*config.MappedKind, error) {
	var mapped *config.MappedKind
	seen := make(map[string]struct{})
	for {
		replacement, ok := kindMap[kind]
		if !ok {
			return mapped, nil
		}
		if _, dup := seen[replacement.KindName]; dup {
			return nil, fmt.Errorf("dispatch: kind map loop at %q", replacement.KindName)
		}
		seen[replacement.KindName] = struct{}{}
		current := replacement
		mapped = &current
		if kind == replacement.KindName {
			return mapped, nil
		}
		kind = replacement.KindName
	}
}

func nonNilBytes(b []byte) []byte {
	if b == nil {
		return []byte{}
	}
	return b
}

func splitLines(data []byte) [][]byte {
	if len(data) == 0 {
		return nil
	}
	var lines [][]byte
	for len(data) > 0 {
		i := bytes.IndexByte(data, '\n')
		if i < 0 {
			lines = append(lines, data)
			break
		}
		lines = append(lines, data[:i+1])
		data = data[i+1:]
	}
	return lines
}

func diffLines(original, intended []byte) []intendedEdit {
	oldLines := splitLines(original)
	newLines := splitLines(intended)
	oldOffsets := lineOffsets(oldLines)
	var edits []intendedEdit
	for _, code := range difflib.NewMatcher(toStrings(oldLines), toStrings(newLines)).GetOpCodes() {
		if code.Tag == 'e' {
			continue
		}
		edit := intendedEdit{
			Start:       oldOffsets[code.I1],
			End:         oldOffsets[code.I2],
			Replacement: nonNilBytes(bytes.Join(newLines[code.J1:code.J2], nil)),
		}
		if bytes.Equal(original[edit.Start:edit.End], edit.Replacement) {
			continue
		}
		edits = append(edits, edit)
	}
	return edits
}

func lineOffsets(lines [][]byte) []uint64 {
	offsets := make([]uint64, len(lines)+1)
	for i, line := range lines {
		offsets[i+1] = offsets[i] + uint64(len(line))
	}
	return offsets
}

func toStrings(lines [][]byte) []string {
	out := make([]string, len(lines))
	for i, line := range lines {
		out[i] = string(line)
	}
	return out
}
