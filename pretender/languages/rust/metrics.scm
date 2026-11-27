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

; Assertion method calls — assert_cmd / predicates-style chains
; (pretender-jg9y). Every call in the chain is its own call_expression
; node, so each link counts: `.assert().success().stdout(...)` counts 3.
(call_expression
  function: (field_expression
    field: (field_identifier) @assertion.method_name)
  (#match? @assertion.method_name "^(assert|success|failure|code|stdout|stderr|predicate)$")) @assert.method_call

; Type definitions (universal code model)
(struct_item
  name: (type_identifier) @type.name) @type.definition

(field_declaration
  name: (field_identifier) @type.field.name
  type: (_) @type.field.type) @type.field
