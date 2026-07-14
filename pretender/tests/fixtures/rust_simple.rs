// expected_complexity: simple=1, with_branch=2, complex_func=5

fn simple(x: i32) -> i32 {
    x + 1
}

fn with_branch(x: i32) -> i32 {
    if x > 0 {
        x
    } else {
        -x
    }
}

fn complex_func(a: i32, b: i32, items: &[i32]) -> i32 {
    let mut total = 0;
    if a > 0 {
        if b > 0 {
            total += a + b;
        }
    }
    for item in items {
        if *item % 2 == 0 {
            total += item;
        }
    }
    total
}
