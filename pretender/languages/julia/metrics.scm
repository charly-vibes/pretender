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
; Assertions (test role min_assertions)
; Test/assertion macros — capture the macrocall node, match the macro name.
(macrocall_expression
  (macro_identifier) @assertion.macro_name
  (#match? @assertion.macro_name "^@(test|test_throws|test_broken|test_logs|assert)")) @assert.macrocall

; Type definitions (universal code model)
; `struct` and `mutable struct` share the struct_definition node, so both
; map to Struct. The type name lives inside type_head; parametric heads
; (`struct Pair{T}`) nest the identifier one node deeper.
(struct_definition
  (type_head
    (identifier) @type.name)) @type.definition

(struct_definition
  (type_head
    (parametrized_type_expression
      (identifier) @type.name))) @type.definition

; Fields — typed declarations `name::Type` (simple and parametric types)
(typed_expression
  (identifier) @type.field.name
  (identifier) @type.field.type) @type.field

(typed_expression
  (identifier) @type.field.name
  (parametrized_type_expression) @type.field.type) @type.field

; Fields — plain assignments `name = default` (untyped fields are common
; in Julia; type_name stays None).
(assignment
  (identifier) @type.field.name) @type.field
