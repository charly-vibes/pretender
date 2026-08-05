// expected_complexity: outer=2, inner=2

package main

func outer(x int) int {
    inner := func(y int) int {
        if y > 0 {
            return y
        } else {
            return -y
        }
    }
    if x > 0 {
        return inner(x)
    } else {
        return 0
    }
}