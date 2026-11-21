; Function definitions (free functions and methods)
(function_definition
  declarator: (function_declarator
    declarator: (identifier) @function.name
    parameters: (parameter_list) @function.parameters)
  body: (compound_statement) @function.body) @function.definition

(function_definition
  declarator: (function_declarator
    declarator: (qualified_identifier
      name: (identifier) @function.name)
    parameters: (parameter_list) @function.parameters)
  body: (compound_statement) @function.body) @function.definition

; Member functions defined inside a class body: the declarator is a
; field_identifier there (and may carry trailing qualifiers like const).
(function_definition
  declarator: (function_declarator
    declarator: (field_identifier) @function.name
    parameters: (parameter_list) @function.parameters)
  body: (compound_statement) @function.body) @function.definition

; Branches
(if_statement) @branch.if
(for_statement) @branch.loop
(for_range_loop) @branch.loop
(while_statement) @branch.loop
(do_statement) @branch.loop
(catch_clause) @branch.catch
(case_statement) @branch.case

; Logical operators
(binary_expression operator: "&&") @branch.logical.and
(binary_expression operator: "||") @branch.logical.or

; Calls (ABC C-count)
(call_expression
  function: (_) @call.callee) @call

; Assignments (ABC A-count)
(assignment_expression) @assign
(init_declarator
  value: (_)) @assign

; Type definitions (universal code model)
; Unions, enums, and typedefs are not captured (design D3):
; struct_specifier → Struct, class_specifier → Class (fallback).
; Requiring name + body skips forward declarations.
(struct_specifier
  name: (type_identifier) @type.name
  body: (field_declaration_list)) @type.definition

(class_specifier
  name: (type_identifier) @type.name
  body: (field_declaration_list)) @type.definition

; Fields: both bare and pointer declarators reach a field_identifier.
(field_declaration
  type: (_) @type.field.type
  declarator: (field_identifier) @type.field.name) @type.field

(field_declaration
  type: (_) @type.field.type
  declarator: (pointer_declarator
    declarator: (field_identifier) @type.field.name)) @type.field
