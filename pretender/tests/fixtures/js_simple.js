// expected_complexity: simple=1, withBranch=2, complexFunc=5

function simple(x) {
  return x + 1;
}

function withBranch(x) {
  if (x > 0) {
    return x;
  }
  return -x;
}

function complexFunc(a, b, items) {
  let total = 0;
  if (a > 0) {
    if (b > 0) {
      total += a + b;
    }
  }
  for (const item of items) {
    if (item % 2 === 0) {
      total += item;
    }
  }
  return total;
}
