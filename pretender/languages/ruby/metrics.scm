; Function definitions
(method
  name: (identifier) @function.name
  parameters: (method_parameters) @function.parameters
  body: (body_statement) @function.body) @function.definition

(singleton_method
  name: (identifier) @function.name
  parameters: (method_parameters) @function.parameters
  body: (body_statement) @function.body) @function.definition

; Branches
(if) @branch.if
(unless) @branch.if
(elsif) @branch.if
(for) @branch.loop
(while) @branch.loop
(until) @branch.loop
(rescue) @branch.rescue
(when) @branch.case

; Logical operators
(binary
  operator: "&&") @branch.logical.and
(binary
  operator: "||") @branch.logical.or
(binary
  operator: "and") @branch.logical.and
(binary
  operator: "or") @branch.logical.or

; Calls (ABC C-count)
(call
  method: (identifier) @call.callee) @call

; Assignments (ABC A-count)
(assignment) @assign
(operator_assignment) @assign
