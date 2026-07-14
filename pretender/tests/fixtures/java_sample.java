// expected_complexity: simple=1, withBranch=2, complexFunc=5
public class Sample {
    public int simple(int x) {
        return x + 1;
    }

    public int withBranch(int x) {
        if (x > 0) {
            return x;
        }
        return -x;
    }

    public int complexFunc(int a, int b, int[] items) {
        int total = 0;
        if (a > 0) {
            if (b > 0) {
                total += a + b;
            }
        }
        for (int item : items) {
            if (item % 2 == 0) {
                total += item;
            }
        }
        return total;
    }
}
