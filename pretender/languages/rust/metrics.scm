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

; Imports (for coupling analysis)
(use_declaration) @import

; Assertions (test role min_assertions)
; Assertion macros — capture the invocation node, match the macro name.
(macro_invocation
  macro: (identifier) @assertion.macro_name
  (#match? @assertion.macro_name "^(assert|assert_eq|assert_ne|debug_assert|debug_assert_eq|debug_assert_ne|panic|unreachable)$")) @assert.macro_invocation
