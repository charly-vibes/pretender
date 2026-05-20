; Function definitions (free functions and methods)
(function_definition
  declarator: (function_declarator
    declarator: (identifier) @function.name
    parameters: (parameter_list) @function.parameters)
  body: (compound_statement) @function.body) @function.definition

(function_definition
  declarator: (function_declarator
    declarator: (qualified_identifier
      name: (identifier) @function.name)
    parameters: (parameter_list) @function.parameters)
  body: (compound_statement) @function.body) @function.definition

; Branches
(if_statement) @branch.if
(for_statement) @branch.loop
(for_range_loop) @branch.loop
(while_statement) @branch.loop
(do_statement) @branch.loop
(catch_clause) @branch.catch
(case_statement) @branch.case

; Logical operators
(binary_expression operator: "&&") @branch.logical.and
(binary_expression operator: "||") @branch.logical.or

; Calls (ABC C-count)
(call_expression
  function: (_) @call.callee) @call

; Assignments (ABC A-count)
(assignment_expression) @assign
(init_declarator
  value: (_)) @assign
