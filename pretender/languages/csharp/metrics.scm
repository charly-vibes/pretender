; Function definitions
(method_declaration
  name: (identifier) @function.name
  parameters: (parameter_list) @function.parameters
  body: (block) @function.body) @function.definition

(constructor_declaration
  name: (identifier) @function.name
  parameters: (parameter_list) @function.parameters
  body: (block) @function.body) @function.definition

(destructor_declaration
  body: (block) @function.body) @function.definition

; Local functions (C# 7+)
(local_function_statement
  name: (identifier) @function.name
  parameters: (parameter_list) @function.parameters
  body: (block) @function.body) @function.definition

; Branches
(if_statement) @branch.if
(for_statement) @branch.loop
(foreach_statement) @branch.loop
(while_statement) @branch.loop
(do_statement) @branch.loop
(catch_clause) @branch.catch
(conditional_expression) @branch.ternary
(switch_section) @branch.case

; Logical operators — match binary_expression with "&&" or "||" operator token
(binary_expression "&&") @branch.logical.and
(binary_expression "||") @branch.logical.or

; Calls (ABC C-count) — invocation via member access or direct name
(invocation_expression
  function: (member_access_expression
    name: (identifier) @call.callee)) @call
(invocation_expression
  function: (identifier) @call.callee) @call
(object_creation_expression) @call

; Assignments (ABC A-count)
(assignment_expression) @assign
(variable_declarator
  (identifier)
  .
  (_)) @assign