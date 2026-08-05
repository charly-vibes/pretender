// expected_complexity: outer=2, inner=2

function outer(x) {
    function inner(y) {
        if (y > 0) {
            return y;
        } else {
            return -y;
        }
    }
    if (x > 0) {
        return inner(x);
    } else {
        return 0;
    }
}