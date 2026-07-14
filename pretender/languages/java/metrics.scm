; Function definitions
(method_declaration
  name: (identifier) @function.name
  parameters: (formal_parameters) @function.parameters
  body: (block) @function.body) @function.definition

(constructor_declaration
  name: (identifier) @function.name
  parameters: (formal_parameters) @function.parameters
  body: (constructor_body) @function.body) @function.definition

; Branches
(if_statement) @branch.if
(for_statement) @branch.loop
(enhanced_for_statement) @branch.loop
(while_statement) @branch.loop
(do_statement) @branch.loop
(catch_clause) @branch.catch
(ternary_expression) @branch.ternary
(switch_block_statement_group
  (switch_label) @branch.case)

; Logical operators
(binary_expression operator: "&&") @branch.logical.and
(binary_expression operator: "||") @branch.logical.or

; Calls (ABC C-count)
(method_invocation
  name: (identifier) @call.callee) @call
(object_creation_expression) @call

; Assignments (ABC A-count)
(assignment_expression) @assign
(variable_declarator
  value: (_)) @assign
