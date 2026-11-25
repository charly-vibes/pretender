## MODIFIED Requirements

### Requirement: Branch Model

The system SHALL represent control-flow points as `Branch` values with kind, span, nesting depth at capture time, and optional logical sequence ID. Branch kinds SHALL include if, else-if, switch case, loop, catch, ternary, logical-and, logical-or, comprehension-filter, match-case, cond-clause, and recur.

Python adapters SHALL capture match arms as `match-case` branches (one per `case` clause; the enclosing `match` adds no branch) and comprehension `for` clauses as `loop` branches with each trailing `if` filter as a `comprehension-filter` branch. Clojure adapters SHALL capture `cond`/`case`/`condp` as one `cond-clause` branch per test/result pair, `recur` as a `recur` branch, and exactly one `catch` branch per `catch` clause (`try` and `finally` SHALL NOT be branches).

#### Scenario: Python match arms are branches
- **WHEN** a Python function contains a `match` with 3 `case` clauses and no other branching
- **THEN** the unit's cyclomatic complexity is 4 (1 + 3 match-case branches)

#### Scenario: Python comprehension filters are branches
- **WHEN** a Python comprehension has one `for` clause and two `if` filters
- **THEN** the unit captures one loop branch and two comprehension-filter branches

#### Scenario: Clojure cond counts per clause
- **WHEN** a Clojure function contains a 3-clause `cond` and no other branching
- **THEN** the unit's cyclomatic complexity is 4 (1 + 3 cond-clause branches)

#### Scenario: Clojure try is not a branch
- **WHEN** a Clojure form is `(try body (catch E e handler) (finally cleanup))`
- **THEN** exactly one catch branch is captured and cyclomatic complexity of the enclosing unit increases by 1

#### Scenario: Logical sequence IDs are scoped to code unit
- **WHEN** logical operators are captured in two different code units
- **THEN** their sequence IDs are unique only within each enclosing code unit

### Requirement: Tree-Sitter Query Contract

Each implemented language adapter SHALL provide a `.scm` query file whose captures populate the universal model. The current MVP query engine requires captures for `@function.definition`, `@function.name`, `@function.parameters`, and `@function.body`; it also consumes optional branch, `@call`, `@call.callee`, `@assign`, and `@assert.*` captures. Assertion captures SHALL contribute to the per-code-unit assertion count used by role-specific threshold checks.

The Clojure query SHALL NOT capture a form as `@call` when the same form is captured as a branch, assignment, function definition, or assertion: control forms (`if`, `when`, `cond`, `case`, `loop`, `for`, `doseq`, `recur`, `try`, `catch`, `let`, `and`, `or`) SHALL contribute to ABC's B-count or be excluded, never to its C-count.

#### Scenario: Clojure control form is not a call
- **WHEN** a Clojure unit body contains `(if (pos? x) x (- x))`
- **THEN** `pos?` and `-` are calls (C-count) and `if` contributes only a branch (B-count)

#### Scenario: Function captures populate code unit
- **WHEN** a query captures `@function.definition`, `@function.name`, `@function.parameters`, and `@function.body`
- **THEN** the adapter creates the corresponding `CodeUnit`

### Requirement: Code Unit Model

The system SHALL represent each parsed function-like definition as a `CodeUnit` with name, kind, span, parameters, body block, and metrics. Kinds SHALL include Function, Method, Ctor, and Test where the language expresses them.

Python adapters SHALL resolve Method and Ctor kinds from enclosing `class_definition` scopes. Clojure adapters SHALL produce one unit per arity for multi-arity `defn` (each with that arity's parameter list, named `name`), resolve kind `Method` for `defn` inside `defrecord`/`deftype`/`defprotocol` bodies, and capture `deftest` forms as units of kind `Test`.

#### Scenario: Clojure multi-arity defn produces one unit per arity
- **WHEN** a Clojure file defines `(defn f ([x] x) ([x y] (+ x y)))`
- **THEN** two units named `f` are captured, with 1 and 2 parameters respectively

#### Scenario: Clojure deftest is a Test unit
- **WHEN** a Clojure file defines `(deftest arith-test (is (= 4 (+ 2 2))))`
- **THEN** a unit named `arith-test` of kind `Test` is captured with assertions count 1

#### Scenario: Python method resolves kind
- **WHEN** a `def` appears inside a Python `class` body
- **THEN** the unit kind is Method (Ctor for `__init__`)

#### Scenario: Function maps to code unit
- **WHEN** a query captures `@function.definition`, `@function.name`, `@function.parameters`, and `@function.body`
- **THEN** the adapter creates the corresponding `CodeUnit` with name, kind, span, parameters, and body

### Requirement: Call Site Weighting

In the current MVP, the parser SHALL assign every parsed `CallSite` a default `smell_weight` of `1.0`, with smell-specific weighting applied later during metric evaluation. The application of `[smell_weights]` from `plugin.toml` to the ABC C-count versus smell detection SHALL be pinned by an adapter unit test so the semantics are explicit and regression-guarded.

#### Scenario: Parsed call site gets default weight
- **WHEN** the parser extracts a `CallSite` from source
- **THEN** it carries the default `smell_weight` of `1.0` until smell-specific weighting applies during metric evaluation

#### Scenario: Clojure smell weight semantics are pinned
- **WHEN** the Clojure adapter round-trip test runs on the standard fixture
- **THEN** the expected ABC values in the test document whether `[smell_weights]` entries (e.g. `swap!` at 1.5) contribute to the C-count

## ADDED Requirements

### Requirement: Parameter Extraction

The system SHALL extract unit parameters from the language's parameter capture: Python from the `parameters` node (including self/cls), Clojure from the arity vector's direct symbol bindings. Destructuring bindings SHALL count as one parameter per binding form.

#### Scenario: Clojure parameters extracted
- **WHEN** a Clojure unit is `(defn complex-func [a b items] ...)`
- **THEN** the unit's parameter count is 3 with names `a`, `b`, `items`

#### Scenario: Destructured binding counts once
- **WHEN** a Clojure arity vector is `[x {:keys [a b]}]`
- **THEN** the unit's parameter count is 2 (`x` and the map binding)

### Requirement: Clojure Nesting Model

Clojure adapters SHALL increment block nesting for forms nested inside branch, loop, or recur bodies within the same unit, matching the shared engine's nesting model. `let`, `binding`, `do`, `testing`, and function-body-adjacent binding forms SHALL NOT increment nesting.

#### Scenario: Clojure nesting is no longer flat
- **WHEN** the `complex-func` fixture (nested `if` inside `if`, `doseq` containing `if`) is parsed
- **THEN** the unit's nesting_max is 2

#### Scenario: Let does not nest
- **WHEN** a Clojure unit body is a single `(let [x 1] x)`
- **THEN** the unit's nesting_max is 0

### Requirement: Branch Weight Table Completeness

Every branch capture kind in a language adapter's `metrics.scm` SHALL have a corresponding weight row in that adapter's `plugin.toml [branches]` table, and a test SHALL guard the invariant by diffing capture kinds against table rows.

#### Scenario: Capture without weight row fails the guard test
- **WHEN** a `metrics.scm` introduces a new `@branch.<kind>` capture with no `[branches]` entry in `plugin.toml`
- **THEN** the weight-table guard test fails naming the missing kind

### Requirement: Declared Unsupported Metrics

Each language adapter's README SHALL list the metrics the language cannot support, so absence is intentional rather than accidental. For Clojure: type definitions (no nominal type system), per-element destructured parameter counts, and constructor unit kinds. For Python: none beyond the already-documented surface.

#### Scenario: Clojure README declares unsupported metrics
- **WHEN** a user reads `languages/clojure/README.md`
- **THEN** the unsupported-metrics section names type definitions, destructured-parameter granularity, and constructor kinds
