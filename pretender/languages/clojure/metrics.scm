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

; Function definitions — multi-arity defn/defn-: the quantifier repeats
; @function.parameters/@function.body once per arity clause within ONE
; match, pairwise and in order; the engine splits them into one unit per
; arity (pretender-1v8k.5, design D2).
(list_lit
  value: (sym_lit) @_defn_multi
  .
  value: (sym_lit) @function.name
  .
  value: (list_lit
    .
    value: (vec_lit) @function.parameters
    value: (_) @function.body)+) @function.definition
  (#match? @_defn_multi "^(defn|defn-)$")

; Function definitions — deftest (no parameter vector, pretender-1v8k.8)
(list_lit
  value: (sym_lit) @_deftest
  .
  value: (sym_lit) @function.name
  .
  value: (_) @function.body) @function.definition
  (#match? @_deftest "^deftest$")

; Assertions (test role min_assertions) — is-family forms
(list_lit
  value: (sym_lit) @_assert_callee
  (#match? @_assert_callee "^(is|is-not|are|assert|thrown\\?|thrown-with-msg\\?|with-test)$")) @assert.statement

; Branches — single-decision conditionals
(list_lit
  value: (sym_lit) @_cond
  (#match? @_cond "^(if|when|when-not|if-not|if-let|when-let)$")) @branch.if

; cond — one branch per test/result pair (proposal D7)
(list_lit
  value: (sym_lit) @_clause
  (#match? @_clause "^cond$")) @branch.clause

; case — pairs start after the dispatch expression
(list_lit
  value: (sym_lit) @_case
  .
  value: (_) @clause.dispatch
  (#match? @_case "^case$")) @branch.switch_case

; condp — pairs start after the predicate and the value
(list_lit
  value: (sym_lit) @_condp
  .
  value: (_) @clause.dispatch
  .
  value: (_) @clause.dispatch
  (#match? @_condp "^condp$")) @branch.clause

; recur — a loop back-edge is its own branch
(list_lit
  value: (sym_lit) @_recur
  (#match? @_recur "^recur$")) @branch.recur

; Loops
(list_lit
  value: (sym_lit) @_loop
  (#match? @_loop "^(loop|for|doseq)$")) @branch.loop

; catch — one branch per clause; try/finally are consumed scaffolding
(list_lit
  value: (sym_lit) @_catch
  (#match? @_catch "^catch$")) @branch.catch

; Consumed forms — claimed so they never fall through to @call (D3):
; binding/sequence forms and error scaffolding contribute no C-count
(list_lit
  value: (sym_lit) @_consumed
  (#match? @_consumed "^(let|binding|do|try|finally|testing)$")) @consumed

; Logical operators
(list_lit
  value: (sym_lit) @_and
  (#eq? @_and "and")) @branch.logical.and

(list_lit
  value: (sym_lit) @_or
  (#eq? @_or "or")) @branch.logical.or

; Calls (ABC C-count): the `.` anchor pins @call.callee to the list's
; head symbol. Without it the unanchored repeated `value:` field matches
; at every sym_lit position — `(swap! total + a b)` yields one match per
; argument symbol, and the last insert into `CaptureMap.calls` overwrites
; the head, so `[smell_weights]` lookups keyed on `swap!` never hit
; (python's fixed `function:` field never had this ambiguity).
(list_lit
  .
  value: (sym_lit) @call.callee) @call

; Assignments (ABC A-count)
(list_lit
  value: (sym_lit) @_def
  .
  value: (sym_lit) @assign
  (#match? @_def "^(def|defonce)$"))