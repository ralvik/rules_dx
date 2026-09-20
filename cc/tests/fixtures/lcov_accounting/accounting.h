// LCOV accounting fixture header; inline plus declarations (issue #501).
#pragma once

// Add returns the sum of a and b.
int Add(int a, int b);

// IsPositive reports whether value is positive.
bool IsPositive(int value);

// ClampNegative maps negatives to -1; the defensive return stays
// LCOV-ignored in accounting.cc with a nearby reason.
int ClampNegative(int value);

// Double is header-executable: its LCOV DA lives on the header.
inline int Double(int value) {
  return value * 2;
}
