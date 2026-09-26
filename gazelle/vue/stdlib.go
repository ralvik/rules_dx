package vue

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
