// LCOV accounting fixture library; consumer of cc_library.
#include "cc/tests/fixtures/lcov_accounting/accounting.h"

int Add(int a, int b) {
  return a + b;
}

bool IsPositive(int value) {
  if (value > 0) {
    return true;
  }
  return false;
}

int ClampNegative(int value) {
  if (value < 0) {
    return -1;  // LCOV_EXCL_LINE - reason: defensive fixture branch never exercised.
  }
  return value;
}
