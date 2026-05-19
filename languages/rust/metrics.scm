; Function definitions
(function_item
  name: (identifier) @function.name
  parameters: (parameters) @function.parameters
  body: (block) @function.body) @function.definition

; Branches
(if_expression) @branch.if
(for_expression) @branch.loop
(while_expression) @branch.loop
(loop_expression) @branch.loop
(match_arm) @branch.match_arm

; Calls (ABC C-count)
(call_expression
  function: (_) @call.callee) @call

; Assignments (ABC A-count)
(assignment_expression) @assign
(compound_assignment_expr) @assign
(let_declaration
  value: (_)) @assign
