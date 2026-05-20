# expected_complexity: simple=1, with_branch=2, complex_func=5
def simple(x)
  x + 1
end

def with_branch(x)
  if x > 0
    x
  else
    -x
  end
end

def complex_func(a, b, items)
  total = 0
  if a > 0
    if b > 0
      total += a + b
    end
  end
  i = 0
  while i < items.length
    if items[i] % 2 == 0
      total += items[i]
    end
    i += 1
  end
  total
end
