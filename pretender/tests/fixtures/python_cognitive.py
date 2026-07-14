def deeply_nested(x):
    if x > 0:
        for i in range(x):
            while i > 0:
                if i % 2 == 0:
                    if i % 3 == 0:
                        return i
                    elif i % 5 == 0:
                        return i - 1
    return 0
