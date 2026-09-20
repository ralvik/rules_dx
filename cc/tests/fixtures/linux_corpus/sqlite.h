// Source-built SQLite shape stub.
#pragma once

// Opaque handle stands in for sqlite3*; the shape proves source-built C
// amalgamation through declared Bazel inputs, not a prebuilt binary.
typedef int dx_sqlite_db;

#ifdef __cplusplus
extern "C" {
#endif

int dx_sqlite_open(const char *path, dx_sqlite_db *out);
int dx_sqlite_exec(dx_sqlite_db db, const char *sql);
int dx_sqlite_close(dx_sqlite_db db);

#ifdef __cplusplus
}
#endif
