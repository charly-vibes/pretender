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

; Type definitions
(class_declaration
  name: (type_identifier) @type.name) @type.definition

; Typed class properties -> fields; type text comes from the annotation.
; Function-valued properties (arrow functions, function expressions)
; are CodeUnits, not data fields — members are not duplicated, so the
; value is only paired with non-function expression kinds.
(public_field_definition
  !value
  name: (property_identifier) @type.field.name
  type: (type_annotation
          (_) @type.field.type)?) @type.field

(public_field_definition
  name: (property_identifier) @type.field.name
  type: (type_annotation
          (_) @type.field.type)?
  value: [
    (assignment_expression) (augmented_assignment_expression)
    (await_expression) (binary_expression) (call_expression)
    (identifier) (member_expression) (new_expression)
    (parenthesized_expression) (sequence_expression) (subscript_expression)
    (template_string) (string) (number) (regex)
    (true) (false) (null) (undefined) (object) (array) (class)
    (unary_expression) (update_expression)
    (as_expression) (non_null_expression)
  ]) @type.field

; Function-valued class properties are CodeUnits (methods).
(public_field_definition
  name: (property_identifier) @function.name
  value: [
    (arrow_function
      parameters: (_) @function.parameters
      body: (_) @function.body)
    (function_expression
      parameters: (_) @function.parameters
      body: (statement_block) @function.body)
  ]) @function.definition

; Declared property signatures (ambient/declare class members)
(property_signature
  name: (property_identifier) @type.field.name
  type: (type_annotation
          (_) @type.field.type)?) @type.field
