// Stdlib lists the Node.js builtin module identities treated as standard
// library by the JavaScript Gazelle extension (M16, ADR 0012/0013).
// Generation treats these imports as standard library: they resolve without
// an edge, a manifest, or a lockfile. Both bare (`fs`) and `node:`-prefixed
// (`node:fs`) forms are recognized; the list follows the Node release
// backing the pinned aspect_rules_js toolchain.
package javascript

// stdlibModules is the exact builtin identity set (without the `node:`
// prefix; callers strip it before lookup).
var stdlibModules = map[string]struct{}{
	"assert": {}, "async_hooks": {}, "buffer": {}, "child_process": {},
	"cluster": {}, "console": {}, "constants": {}, "crypto": {},
	"dgram": {}, "diagnostics_channel": {}, "dns": {}, "domain": {},
	"events": {}, "fs": {}, "fs/promises": {}, "http": {}, "http2": {},
	"https": {}, "inspector": {}, "inspector/promises": {}, "module": {},
	"net": {}, "os": {}, "path": {}, "path/posix": {}, "path/win32": {},
	"perf_hooks": {}, "process": {}, "punycode": {}, "querystring": {},
	"readline": {}, "readline/promises": {}, "repl": {}, "sea": {},
	"sqlite": {}, "stream": {}, "stream/consumers": {}, "stream/promises": {},
	"stream/web": {}, "string_decoder": {}, "sys": {}, "test": {},
	"test/reporters": {}, "timers": {}, "timers/promises": {}, "tls": {},
	"trace_events": {}, "tty": {}, "url": {}, "util": {},
	"util/types": {}, "v8": {}, "vm": {}, "wasi": {},
	"worker_threads": {}, "zlib": {},
}

// IsStdLib reports whether a parsed import root is Node standard library.
// The `node:` prefix is stripped before lookup; subpaths (`fs/promises`)
// match exactly, never by prefix.
func IsStdLib(name string) bool {
	if len(name) > 5 && name[:5] == "node:" {
		name = name[5:]
	}
	if name == "" {
		return false
	}
	_, ok := stdlibModules[name]
	return ok
}
