; Function definitions — matches all (top-level, methods, decorated, nested)
(function_definition
  name: (identifier) @function.name
  parameters: (parameters) @function.parameters
  body: (block) @function.body) @function.definition

; Branches
(if_statement) @branch.if
(elif_clause) @branch.elif
(for_statement) @branch.loop
(while_statement) @branch.loop
(except_clause) @branch.catch
(except_group_clause) @branch.catch
(conditional_expression) @branch.ternary

; Logical operators
(boolean_operator
  operator: "and") @branch.logical.and
(boolean_operator
  operator: "or") @branch.logical.or

; Calls (ABC C-count)
(call function: (_) @call.callee) @call

; Assignments (ABC A-count) — assignment statements and augmented assignment
(assignment) @assign
(augmented_assignment) @assign

; Assertions (test role min_assertions)
(assert_statement) @assert.statement
(call
  function: (identifier) @assert.fn (#match? @assert.fn "^assert"))
