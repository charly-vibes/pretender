; Function definitions
(function_declaration
  name: (identifier) @function.name
  parameters: (parameter_list) @function.parameters
  body: (block) @function.body) @function.definition

(method_declaration
  name: (field_identifier) @function.name
  parameters: (parameter_list) @function.parameters
  body: (block) @function.body) @function.definition

; Branches
(if_statement) @branch.if
(for_statement) @branch.loop
(expression_switch_statement
  (expression_case) @branch.case)
(type_switch_statement
  (type_case) @branch.case)
(communication_case) @branch.case

; Logical operators
(binary_expression operator: "&&") @branch.logical.and
(binary_expression operator: "||") @branch.logical.or

; Calls (ABC C-count)
(call_expression function: (_) @call.callee) @call

; Assignments (ABC A-count)
(assignment_statement) @assign
(short_var_declaration) @assign
(var_declaration) @assign
