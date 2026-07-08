// expected_complexity: Simple=1, WithBranch=2, ComplexFunc=5
public class Sample {
    public int Simple(int x) {
        return x + 1;
    }

    public int WithBranch(int x) {
        if (x > 0) {
            return x;
        }
        return -x;
    }

    public int ComplexFunc(int a, int b, int[] items) {
        int total = 0;
        if (a > 0) {
            if (b > 0) {
                total += a + b;
            }
        }
        for (int i = 0; i < items.Length; i++) {
            if (items[i] % 2 == 0) {
                total += items[i];
            }
        }
        return total;
    }
}