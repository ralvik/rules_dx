// LCOV accounting fixture test; fully covered with no ignores.
#include <cassert>

#include "cc/tests/fixtures/lcov_accounting/accounting.h"

int main() {
  assert(Add(2, 3) == 5);
  assert(Add(-1, 1) == 0);
  assert(Double(3) == 6);
  assert(IsPositive(1));
  assert(!IsPositive(-1));
  assert(!IsPositive(0));
  assert(ClampNegative(5) == 5);
  assert(ClampNegative(0) == 0);
  assert(ClampNegative(-3) == -1);
  return 0;
}
