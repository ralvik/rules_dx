// Source-built SQLite shape stub (issue #499).
#include "cc/tests/fixtures/linux_corpus/sqlite.h"

#include <stddef.h>

int dx_sqlite_open(const char *path, dx_sqlite_db *out) {
  if (path == NULL || out == NULL) {
    return 1;
  }
  if (path[0] == '\0') {
    return 1;
  }
  *out = 42;
  return 0;
}

int dx_sqlite_exec(dx_sqlite_db db, const char *sql) {
  if (db != 42 || sql == NULL) {
    return 1;
  }
  return 0;
}

int dx_sqlite_close(dx_sqlite_db db) {
  return db == 42 ? 0 : 1;
}
