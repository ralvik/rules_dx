// Seed C++ lint fixture.
int greet(int value) {
  int *slot = nullptr;
  if (value > 0) {
    slot = &value;
  }
  return *slot;
}
