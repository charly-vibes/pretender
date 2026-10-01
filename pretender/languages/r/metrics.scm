; Function definitions
; R functions are anonymous; the name is on the LHS of <- or =
(binary_operator
    lhs: (identifier) @function.name
    operator: "<-"
    rhs: (function_definition
        parameters: (parameters) @function.parameters
        body: (_) @function.body) @function.definition)

(binary_operator
    lhs: (identifier) @function.name
    operator: "="
    rhs: (function_definition
        parameters: (parameters) @function.parameters
        body: (_) @function.body) @function.definition)

(binary_operator
    lhs: (identifier) @function.name
    operator: "<<-"
    rhs: (function_definition
        parameters: (parameters) @function.parameters
        body: (_) @function.body) @function.definition)

; Branches
(if_statement) @branch.if
(for_statement) @branch.loop
(while_statement) @branch.loop
(repeat_statement) @branch.loop

; Logical operators
(binary_operator operator: "&&") @branch.logical.and
(binary_operator operator: "||") @branch.logical.or

; Calls (ABC C-count)
(call function: (_) @call.callee) @call

; Assignments (ABC A-count)
(binary_operator operator: "<-") @assign
(binary_operator operator: "<<-") @assign
(binary_operator operator: "=") @assign
(binary_operator operator: "->") @assign
(binary_operator operator: "->>") @assign
(binary_operator operator: ":=") @assign