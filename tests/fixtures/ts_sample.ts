// expected_complexity: greet=1, withBranch=2, complexFunc=5

function greet(name: string): string {
  return `Hello, ${name}`;
}

function withBranch(x: number): number {
  if (x > 0) {
    return x;
  }
  return -x;
}

function complexFunc(a: number, b: number, items: number[]): number {
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
