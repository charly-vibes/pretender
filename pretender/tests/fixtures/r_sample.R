# expected_complexity: simple=1, with_branch=2, complex_func=5
simple <- function(x) {
  x + 1
}

with_branch <- function(x) {
  if (x > 0) {
    x
  } else {
    -x
  }
}

complex_func <- function(a, b, items) {
  total <- 0
  if (a > 0) {
    if (b > 0) {
      total <- total + a + b
    }
  }
  for (item in items) {
    if (item %% 2 == 0) {
      total <- total + item
    }
  }
  total
}