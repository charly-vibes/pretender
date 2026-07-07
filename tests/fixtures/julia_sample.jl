# expected_complexity: simple=1, with_branch=2, complex_func=5
function simple(x)
    return x + 1
end

function with_branch(x)
    if x > 0
        return x
    else
        return -x
    end
end

function complex_func(a, b, items)
    total = 0
    if a > 0
        if b > 0
            total += a + b
        end
    end
    for item in items
        if item % 2 == 0
            total += item
        end
    end
    return total
end