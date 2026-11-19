; Function definitions — matches all (top-level, methods, decorated, nested)
(function_definition
  name: (identifier) @function.name
  parameters: (parameters) @function.parameters
  body: (block) @function.body) @function.definition

; Branches
(if_statement) @branch.if
(elif_clause) @branch.elif
(for_statement) @branch.loop
(while_statement) @branch.loop
(except_clause) @branch.catch
(conditional_expression) @branch.ternary

; Logical operators
(boolean_operator
  operator: "and") @branch.logical.and
(boolean_operator
  operator: "or") @branch.logical.or

; Calls (ABC C-count)
(call function: (_) @call.callee) @call

; Assignments (ABC A-count) — assignment statements and augmented assignment
(assignment) @assign
(augmented_assignment) @assign

; Assertions (test role min_assertions)
(assert_statement) @assert.statement
(call
  function: (identifier) @assert.fn
  (#match? @assert.fn "^assert"))
(call
  function: (attribute
    object: (identifier) @meta.assert_obj
    attribute: (identifier) @meta.assert_attr)
  (#match? @meta.assert_obj "^(self|pytest)$")
  (#match? @meta.assert_attr "^(assert[A-Z].*|raises|warns)$")) @assert.call

; Type definitions (universal model): class declarations → Class via
; determine_type_kind; @dataclass ships as Class per design D3.
(class_definition
  name: (identifier) @type.name) @type.definition

; Annotated attributes on the class body become typed fields. Direct
; children of the class body block only: annotated locals inside method
; bodies and self.x assignments must not leak into the field list.
(class_definition
  body: (block
    (expression_statement
      (assignment
        left: (identifier) @type.field.name
        type: (type) @type.field.type) @type.field)))
