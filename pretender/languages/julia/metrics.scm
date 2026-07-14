; Function definitions
; Julia: function foo(x, y); body; end
; The body expressions are direct children of function_definition (no wrapping block).
; Capture the whole node as both @function.definition and @function.body
; so walk_block iterates child expressions from body.
(function_definition
  (signature
    (call_expression
      (identifier) @function.name
      (argument_list) @function.parameters))) @function.definition @function.body

; Macro definitions — treat as functions
(macro_definition
  (signature
    (call_expression
      (identifier) @function.name
      (argument_list) @function.parameters))) @function.definition @function.body

; Branches
(if_statement) @branch.if
(elseif_clause) @branch.if

(for_statement) @branch.loop
(while_statement) @branch.loop

(try_statement) @branch.catch

(ternary_expression) @branch.ternary

; Logical operators — match by child operator text
(binary_expression
  (_)
  (_) @_op
  (_)
  (#eq? @_op "&&")) @branch.logical.and
(binary_expression
  (_)
  (_) @_op
  (_)
  (#eq? @_op "||")) @branch.logical.or

; Calls (ABC C-count) — first child anchored with .
(call_expression
  .
  (_) @call.callee
  (argument_list)) @call

; Assignments (ABC A-count)
(assignment) @assign
(compound_assignment_expression) @assign