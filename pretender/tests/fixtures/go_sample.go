// expected_complexity: simple=1, with_branch=2, complex_func=5
package main

func simple(x int) int {
	return x + 1
}

func with_branch(x int) int {
	if x > 0 {
		return x
	}
	return -x
}

func complex_func(a int, b int, items []int) int {
	total := 0
	if a > 0 {
		if b > 0 {
			total += a + b
		}
	}
	for _, item := range items {
		if item%2 == 0 {
			total += item
		}
	}
	return total
}
