; Function definitions
(function_declaration
  name: (identifier) @function.name
  parameters: (formal_parameters) @function.parameters
  body: (statement_block) @function.body) @function.definition

; Arrow functions assigned to variables
(variable_declarator
  name: (identifier) @function.name
  value: (arrow_function
    parameters: (_) @function.parameters
    body: (_) @function.body)) @function.definition

; Method definitions in classes
(method_definition
  name: (property_identifier) @function.name
  parameters: (formal_parameters) @function.parameters
  body: (statement_block) @function.body) @function.definition

; Branches
(if_statement) @branch.if
(for_statement) @branch.loop
(for_in_statement) @branch.loop
(while_statement) @branch.loop
(do_statement) @branch.loop
(catch_clause) @branch.catch
(ternary_expression) @branch.ternary
(switch_case) @branch.if

; Logical operators
(binary_expression
  operator: "&&") @branch.logical.and
(binary_expression
  operator: "||") @branch.logical.or

; Calls (ABC C-count)
(call_expression
  function: (_) @call.callee) @call

; Assignments (ABC A-count)
(assignment_expression) @assign
(augmented_assignment_expression) @assign
(variable_declarator
  value: (_)) @assign

; TypeScript-specific: type annotations (captured for completeness, no metric impact)
(type_annotation) @type.annotation

; TypeScript-specific: generic types (no metric impact)
(type_arguments) @type.generic
