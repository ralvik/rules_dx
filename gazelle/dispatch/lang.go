package dispatch

import (
	"context"
	"flag"
	"fmt"
	"os"
	"sort"
	"strings"

	"github.com/bazelbuild/bazel-gazelle/config"
	"github.com/bazelbuild/bazel-gazelle/language"
	"github.com/bazelbuild/bazel-gazelle/rule"

	"github.com/ralvik/rules_dx/gazelle/astro"
	"github.com/ralvik/rules_dx/gazelle/cc"
	"github.com/ralvik/rules_dx/gazelle/csharp"
	"github.com/ralvik/rules_dx/gazelle/fsharp"
	"github.com/ralvik/rules_dx/gazelle/go"
	"github.com/ralvik/rules_dx/gazelle/java"
	"github.com/ralvik/rules_dx/gazelle/javascript"
	"github.com/ralvik/rules_dx/gazelle/kotlin"
	"github.com/ralvik/rules_dx/gazelle/mdx"
	"github.com/ralvik/rules_dx/gazelle/python"
	"github.com/ralvik/rules_dx/gazelle/ruby"
	"github.com/ralvik/rules_dx/gazelle/rust"
	"github.com/ralvik/rules_dx/gazelle/scala"
	"github.com/ralvik/rules_dx/gazelle/svelte"
	"github.com/ralvik/rules_dx/gazelle/typescript"
	"github.com/ralvik/rules_dx/gazelle/vue"
)

const languageName = "dx_dispatch"

// composedLanguages instantiates every first-party extension once so the
// witness replays the exact union loads the framework uses for the
// composed run. The instances carry no per-run state: ApparentLoads only
// reads the module mapping.
func composedLanguages() []language.Language {
	return []language.Language{
		astro.NewLanguage(),
		cc.NewLanguage(),
		csharp.NewLanguage(),
		fsharp.NewLanguage(),
		golang.NewLanguage(),
		java.NewLanguage(),
		javascript.NewLanguage(),
		kotlin.NewLanguage(),
		mdx.NewLanguage(),
		python.NewLanguage(),
		ruby.NewLanguage(),
		rust.NewLanguage(),
		scala.NewLanguage(),
		svelte.NewLanguage(),
		typescript.NewLanguage(),
		vue.NewLanguage(),
	}
}

// unionApparentLoads returns the union loads over every first-party
// extension with the run's own module mapping, mirroring the framework's
// load collection for the composed binary.
func unionApparentLoads(moduleToApparentName func(string) string) []rule.LoadInfo {
	var loads []rule.LoadInfo
	for _, lang := range composedLanguages() {
		if aware, ok := lang.(language.ModuleAwareLanguage); ok {
			loads = append(loads, aware.ApparentLoads(moduleToApparentName)...)
			continue
		}
		loads = append(loads, lang.Loads()...)
	}
	return loads
}

type dispatchLang struct {
	language.BaseLang
	errors   []string
	manifest *manifestRecorder
	configs  []*config.Config
}

// NewLanguage returns the composed-run witness extension. It must run
// last in the composed binary's language order so GenerateRules observes
// every sibling extension's rules as OtherGen.
func NewLanguage() language.Language { return &dispatchLang{} }

// exitProcess ends the Gazelle run when the witness cannot be recorded.
// It is a variable so unit tests can observe the fail-closed decision
// without exiting the test process.
var exitProcess = os.Exit

func (l *dispatchLang) Before(context.Context) {
	l.errors = nil
	l.configs = nil
	l.manifest = nil
	if rec, err := loadManifestRecorder(); err != nil {
		l.fail("%v", err)
	} else {
		l.manifest = rec
	}
	if l.manifest != nil {
		l.manifest.unionLoads = unionApparentLoads
	}
}

func (*dispatchLang) DoneGeneratingRules() {}

func (l *dispatchLang) RegisterFlags(*flag.FlagSet, string, *config.Config) {}

func (l *dispatchLang) CheckFlags(*flag.FlagSet, *config.Config) error { return nil }

func (*dispatchLang) KnownDirectives() []string { return nil }

func (l *dispatchLang) Configure(*config.Config, string, *rule.File) {}

func (l *dispatchLang) fail(format string, args ...interface{}) {
	l.errors = append(l.errors, fmt.Sprintf(format, args...))
}

// collectUsedIgnores gathers the used dx_ignore_import entries visible in
// every visited directory config across all first-party extensions. Each
// extension owns its entries; this witness only unions them for the
// manifest notices. Stale entries stay owned by their extension, which
// fails the run in its own AfterResolvingDeps before this witness emits.
func (l *dispatchLang) collectUsedIgnores() []collectedIgnore {
	var out []collectedIgnore
	seen := map[collectedIgnore]bool{}
	emit := func(language string, pairs [][2]string) {
		for _, pair := range pairs {
			entry := collectedIgnore{path: pair[0], language: language, value: pair[1]}
			if seen[entry] {
				continue
			}
			seen[entry] = true
			out = append(out, entry)
		}
	}
	for _, cfg := range l.configs {
		if cfg == nil {
			continue
		}
		emit("astro", astro.CollectUsedIgnores(cfg))
		emit("cc", cc.CollectUsedIgnores(cfg))
		emit("csharp", csharp.CollectUsedIgnores(cfg))
		emit("fsharp", fsharp.CollectUsedIgnores(cfg))
		emit("go", golang.CollectUsedIgnores(cfg))
		emit("java", java.CollectUsedIgnores(cfg))
		emit("javascript", javascript.CollectUsedIgnores(cfg))
		emit("kotlin", kotlin.CollectUsedIgnores(cfg))
		emit("mdx", mdx.CollectUsedIgnores(cfg))
		emit("python", python.CollectUsedIgnores(cfg))
		emit("ruby", ruby.CollectUsedIgnores(cfg))
		emit("rust", rust.CollectUsedIgnores(cfg))
		emit("scala", scala.CollectUsedIgnores(cfg))
		emit("svelte", svelte.CollectUsedIgnores(cfg))
		emit("typescript", typescript.CollectUsedIgnores(cfg))
		emit("vue", vue.CollectUsedIgnores(cfg))
	}
	sort.Slice(out, func(i, j int) bool {
		if out[i].path != out[j].path {
			return out[i].path < out[j].path
		}
		if out[i].language != out[j].language {
			return out[i].language < out[j].language
		}
		return out[i].value < out[j].value
	})
	return out
}

func (l *dispatchLang) AfterResolvingDeps(context.Context) {
	if len(l.errors) == 0 && l.manifest != nil {
		if err := l.manifest.emit(l.collectUsedIgnores()); err != nil {
			l.fail("%v", err)
		}
	}
	// The framework's Language interface offers no error return here, so a
	// fatal exit — not a panic and its stack trace — is the only way to
	// fail the run before BUILD emission. A zero-length error list returns
	// normally.
	if len(l.errors) > 0 {
		sort.Strings(l.errors)
		fmt.Fprintln(os.Stderr, "Dispatch generation failed before BUILD emission:\n"+strings.Join(l.errors, "\n"))
		exitProcess(1)
	}
}

func (*dispatchLang) Name() string { return languageName }

func (*dispatchLang) Kinds() map[string]rule.KindInfo { return nil }

func (l *dispatchLang) GenerateRules(args language.GenerateArgs) language.GenerateResult {
	if args.Config != nil {
		l.configs = append(l.configs, args.Config)
	}
	if l.manifest != nil {
		l.manifest.record(args, language.GenerateResult{})
	}
	return language.GenerateResult{}
}
