// expected_complexity: simple=1, with_branch=2, complex_func=5
int simple(int x) {
    return x + 1;
}

int with_branch(int x) {
    if (x > 0) {
        return x;
    }
    return -x;
}

int complex_func(int a, int b, int *items, int n) {
    int total = 0;
    if (a > 0) {
        if (b > 0) {
            total += a + b;
        }
    }
    for (int i = 0; i < n; i++) {
        if (items[i] % 2 == 0) {
            total += items[i];
        }
    }
    return total;
}
