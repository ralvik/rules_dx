// Linux corpus test; SQLite plus OpenSSL plus ring shapes (issue #499).
#include <cassert>
#include <string>

#include "cc/tests/fixtures/linux_corpus/openssl.h"
#include "cc/tests/fixtures/linux_corpus/ring.h"
#include "cc/tests/fixtures/linux_corpus/sqlite.h"

int main() {
  dx_sqlite_db db = 0;
  assert(dx_sqlite_open(":memory:", &db) == 0);
  assert(dx_sqlite_exec(db, "CREATE TABLE t(x)") == 0);
  assert(dx_sqlite_close(db) == 0);
  assert(dx_sqlite_open(NULL, &db) != 0);

  assert(dx_openssl_version() == 30000000);
  char digest[65];
  assert(dx_openssl_digest("abc", digest) == 0);
  assert(std::string(digest).size() == 64);
  assert(dx_openssl_digest(NULL, digest) != 0);

  char ring_out[65];
  assert(dx_ring_hash("abc", ring_out) == 0);
  assert(std::string(ring_out).size() == 64);
  assert(std::string(digest) != std::string(ring_out));
  assert(dx_ring_asm_add(2, 3) == 5);
  return 0;
}
