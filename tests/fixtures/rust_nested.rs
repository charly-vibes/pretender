// expected_complexity: outer=2, inner=2

fn outer(x: i32) -> i32 {
    fn inner(y: i32) -> i32 {
        if y > 0 {
            y
        } else {
            -y
        }
    }
    if x > 0 {
        inner(x)
    } else {
        0
    }
}