def smell_eval(code):
    result = eval(code)
    return result


def smell_exec(code):
    exec(code)


def smell_compile(code):
    result = compile(code, "<string>", "exec")
    return result
