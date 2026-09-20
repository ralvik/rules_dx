// Strict generation fixture test; test-owned shape (issue #503).
// Test-only references ride this handwritten cc_test, never the production
// library: *_test sources never enter the generated library.
#include <cassert>

#include "cc/tests/fixtures/strict_generation/strict.h"

int main() {
  assert(StrictGreet("world") == "strict hello world");
  return 0;
}
