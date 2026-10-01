; Function definitions — defn and defn- via regex
(list_lit
  value: (sym_lit) @_defn
  .
  value: (sym_lit) @function.name
  .
  value: (vec_lit) @function.parameters
  .
  value: (_) @function.body) @function.definition
  (#match? @_defn "^(defn|defn-)$")

; Branches — conditionals via regex
(list_lit
  value: (sym_lit) @_cond
  (#match? @_cond "^(if|when|cond|case|when-not|if-not|if-let|when-let)$")) @branch.if

; Loops
(list_lit
  value: (sym_lit) @_loop
  (#match? @_loop "^(loop|for|doseq)$")) @branch.loop

; try/catch
(list_lit
  value: (sym_lit) @_try
  (#match? @_try "^(try|catch)$")) @branch.catch

; Logical operators
(list_lit
  value: (sym_lit) @_and
  (#eq? @_and "and")) @branch.logical.and

(list_lit
  value: (sym_lit) @_or
  (#eq? @_or "or")) @branch.logical.or

; Calls (ABC C-count)
(list_lit
  value: (sym_lit) @call.callee) @call

; Assignments (ABC A-count)
(list_lit
  value: (sym_lit) @_def
  .
  value: (sym_lit) @assign
  (#match? @_def "^(def|defonce)$"))