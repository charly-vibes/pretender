//! Clojure parser integration tests.
//!
//! # Responsibilities
//! Pin Clojure-specific parse behavior through the shared engine.
//!
//! # Rationale
//! The round-trip unit tests moved out of the `#[cfg(test)]` module in
//! `src/clojure.rs` (pretender-948q registry migration): they pin the
//! Clojure adapter's zero-capture contract, and this is their matching
//! integration target. The query is included from the adapter's data
//! directory so the pin rides the same bytes the production registry uses.

use pretender::engine::QueryEngine;
use pretender::metrics::{abc, cognitive, cyclomatic, function_lines, nesting_max};
use pretender::model::{CodeUnit, Language, Module, Parser, UnitKind};
use pretender::registry::{lookup, GenericParser};
use std::path::Path;

fn clojure_engine() -> QueryEngine {
    QueryEngine::new(
        tree_sitter_clojure::LANGUAGE.into(),
        Language::Clojure,
        include_str!("../languages/clojure/metrics.scm"),
    )
    .unwrap_or_else(|e| panic!("clojure query engine: {e:?}"))
}

const DEFRECORD_DEFTYPE_SOURCE: &str =
    include_str!("fixtures/clojure_types_sample.clj");

/// The zero-capture contract: a clean parse with an empty
/// `type_definitions` list.
#[test]
fn clojure_defrecord_deftype_ship_zero_type_captures() {
    // Zero-capture decision (tasks 6.6, design D2/D3): the pinned
    // grammar tree-sitter-clojure 0.1.0 exposes only generic
    // `list_lit`/`sym_lit` nodes — there is no distinct defrecord or
    // deftype node to anchor `@type.definition` on, and capturing
    // `list_lit` would map to `Class`, never the `Record` kind D3
    // reserves for defrecord. Per D2/D3 the adapter ships with zero
    // type captures; parsing must stay valid (spec: "Adapter without
    // type captures stays valid").
    let engine = clojure_engine();
    let (module, diags) = engine
        .parse(
            Path::new("clojure_types_sample.clj"),
            DEFRECORD_DEFTYPE_SOURCE,
        )
        .unwrap_or_else(|e| panic!("clojure sample must parse: {e:?}"));
    assert!(diags.is_empty(), "fixture must parse cleanly: {diags:?}");
    assert!(
        module.type_definitions.is_empty(),
        "pinned grammar has no distinct type nodes; captures must stay empty, got: {:?}",
        module.type_definitions
    );
}

/// Red gate (pretender-krou): the engine must not surface phantom
/// function units from query matches whose `#match?` predicate fails.
/// The shipped clojure query structurally matches `(foo x [y] z)` against
/// the `defn` pattern (head + name + vector + body); the failing
/// predicate used to leak a phantom unit named `x`.
#[test]
fn clojure_call_shaped_like_defn_is_not_a_unit() {
    let module = parse_clojure_source("(defn f [x] (inc x))\n\n(foo x [y] z)\n");
    let names = module
        .units
        .iter()
        .map(|u| u.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(names, vec!["f"], "phantom units present");
}

fn find_unit<'a>(module: &'a Module, name: &str) -> &'a CodeUnit {
    module
        .units
        .iter()
        .find(|u| u.name == name)
        .unwrap_or_else(|| panic!("{name} not captured"))
}

fn param_names(unit: &CodeUnit) -> Vec<&str> {
    unit.parameters.iter().map(|p| p.name.as_str()).collect()
}

/// Red gate (1v8k.8, task 2.5): a deftest form is a Test unit and its
/// direct `is` form counts as one assertion.
#[test]
fn clojure_deftest_is_a_test_unit_with_assertions() {
    let module = control_module();
    let unit = find_unit(&module, "arith-test");
    assert_eq!(unit.kind, UnitKind::Test);
    assert_eq!(unit.assertions, 1);
}

/// Red gate (1v8k.8, task 2.5): is/are/thrown-with-msg? all count as
/// assertions inside a deftest, and assertions inside the consumed
/// `testing` scaffolding are still reached by the body walk.
#[test]
fn clojure_is_family_assertions_count_inside_testing() {
    let module = control_module();
    let unit = find_unit(&module, "throws-test");
    assert_eq!(unit.kind, UnitKind::Test);
    assert_eq!(unit.assertions, 3);
}

/// Red gate (1v8k.8, task 2.5): a defn inside a defrecord/deftype
/// protocol implementation body resolves to Method; the defn inside a
/// deftest body stays Function (deftest is not a type scope).
#[test]
fn clojure_record_and_type_methods_are_methods() {
    let module = parse_clojure_source(
        "(defrecord Point [x y]\n  Area\n  (defn area [this] 1))\n\n(deftype Cell [held]\n  Proto\n  (defn held? [this] true))\n\n(deftest nested-test\n  (is (= 1 (helper 1))))\n\n(defn helper [x]\n  (inc x))\n",
    );
    assert_eq!(find_unit(&module, "area").kind, UnitKind::Method);
    assert_eq!(find_unit(&module, "held?").kind, UnitKind::Method);
    // defn at top level stays Function; defn inside deftest too.
    assert_eq!(find_unit(&module, "helper").kind, UnitKind::Function);
}

fn parse_clojure_source(source: &'static str) -> Module {
    let record = lookup("clj").unwrap_or_else(|| panic!("clj must be registered"));
    let parser = GenericParser { record };
    let (module, diags) = parser
        .parse(Path::new("clojure_params.clj"), source)
        .unwrap_or_else(|e| panic!("clojure source must parse: {e:?}"));
    assert!(diags.is_empty(), "fixture must parse cleanly: {diags:?}");
    module
}

/// Red gate (1v8k.6): a 3-parameter defn reports params: 3.
#[test]
fn clojure_single_arity_defn_names_params() {
    let module = parse_clojure_source(SAMPLE_SOURCE);
    let unit = find_unit(&module, "complex-func");
    assert_eq!(
        param_names(unit),
        vec!["a", "b", "items"],
        "arity vector symbols become parameters in declaration order"
    );
}

/// Variadic and destructuring shapes follow the documented rule:
/// `&` marks the variadic boundary (not a parameter); each destructuring
/// binding form counts as one parameter named by its source text.
#[test]
fn clojure_variadic_and_destructuring_bind_shapes() {
    let module = parse_clojure_source(include_str!("fixtures/clojure_params_sample.clj"));
    assert_eq!(
        param_names(find_unit(&module, "variadic")),
        vec!["a", "rest"],
        "& rest counts as one parameter, matching Python's *rest"
    );
    assert_eq!(
        param_names(find_unit(&module, "deep-bind")),
        vec!["a", "[b c]"],
        "one parameter per destructuring binding, not per inner symbol"
    );
    assert_eq!(
        param_names(find_unit(&module, "destr-map")),
        vec!["x", "{:keys [a b]}"],
        "map bindings count as one parameter"
    );
}

const CONTROL_SAMPLE: &str = include_str!("fixtures/clojure_control_sample.clj");
const SAMPLE_SOURCE: &str = include_str!("fixtures/clojure_sample.clj");
const MULTI_ARITY_SAMPLE: &str = include_str!("fixtures/clojure_multi_arity_sample.clj");

fn control_module() -> Module {
    parse_clojure_source(CONTROL_SAMPLE)
}

/// Per-clause branches (proposal D7): cond/case/condp count one branch
/// per test/result pair, not one per form.
#[test]
fn clojure_cond_case_count_per_clause() {
    let module = control_module();
    assert_eq!(cyclomatic(find_unit(&module, "condy")), 4, "3 clauses");
    assert_eq!(
        cyclomatic(find_unit(&module, "caser")),
        3,
        "2 pairs + default dangling"
    );
    assert_eq!(
        cyclomatic(find_unit(&module, "condpr")),
        3,
        "pred+expr are dispatch head"
    );
}

/// recur is a loop back-edge and counts as its own branch kind.
#[test]
fn clojure_recur_counts_as_branch() {
    assert_eq!(
        cyclomatic(find_unit(&control_module(), "looper")),
        4,
        "loop + if + recur"
    );
}

/// try/finally no longer branch: one catch clause = one branch.
#[test]
fn clojure_try_counts_one_catch() {
    assert_eq!(
        cyclomatic(find_unit(&control_module(), "catcher")),
        2,
        "catch only"
    );
}

/// Control forms leave the C-count (D3): `let` is consumed, so the
/// calls are exactly pos?, inc, println. Pins abc = sqrt(1 + 9).
#[test]
fn clojure_control_forms_leave_call_count() {
    let module = control_module();
    let unit = find_unit(&module, "g");
    assert!(
        (abc(unit) - 10f64.sqrt()).abs() < 1e-9,
        "abc was {}",
        abc(unit)
    );
}

/// D4 nesting pins per control-fixture annotations: each branch-bearing
/// body wraps one level deeper; a single conditional nests exactly once
/// (like a Python if-block); doseq -> when -> if reaches depth 3
/// (cyclomatic 4, cognitive 6). complex-func's wrapping `let` adds no
/// level (its roots sit at depth 0 while bodies nest) — same shape in
/// Python reports nesting_max 2, cognitive 6. `g` (control fixture):
/// the if wraps once; the let inside its body adds no level.
#[test]
fn clojure_nesting_levels_match_control_fixture() {
    for (source, name, nest) in [
        (parse_clojure_source(SAMPLE_SOURCE), "complex-func", 2),
        (control_module(), "g", 1),
        (control_module(), "branchy", 1),
    ] {
        let unit = find_unit(&source, name);
        assert_eq!(nesting_max(unit), nest, "{name}");
    }
    let control = control_module();
    let deeply = find_unit(&control, "deeply-nested");
    assert_eq!(nesting_max(deeply), 3);
    assert_eq!(cyclomatic(deeply), 4);
    assert_eq!(cognitive(deeply), 6);
    let sample = parse_clojure_source(SAMPLE_SOURCE);
    let unit = find_unit(&sample, "complex-func");
    assert_eq!(cognitive(unit), 6);
}

/// D6 semantics pin (verified at HEAD), through the production registry
/// parser: complex-func has A=0, B=4 (if, if, doseq, if), C = 6 calls
/// (pos?, pos?, atom, swap!, even?, swap!) x 1.0 = 6.0. Production
/// engines currently do NOT apply [smell_weights] to the C-count:
/// `QueryEngine` never wires `WeightMaps.call` into `call_weights`
/// (pre-existing dead plumbing, tracked under the smell-weights
/// ticket). This pin freezes that truth; it must be revisited when the
/// wiring lands.
#[test]
fn clojure_control_forms_abc_semantics_pin() {
    let record = lookup("clj").unwrap_or_else(|| panic!("clj must be registered"));
    let parser = GenericParser { record };
    let (module, diags) = parser
        .parse(Path::new("clojure_sample.clj"), SAMPLE_SOURCE)
        .unwrap_or_else(|e| panic!("clojure sample must parse: {e:?}"));
    assert!(diags.is_empty(), "fixture must parse cleanly: {diags:?}");
    let expected = (4.0f64 * 4.0 + 6.0 * 6.0).sqrt();
    let measured = abc(find_unit(&module, "complex-func"));
    assert!((measured - expected).abs() < 1e-9, "abc was {measured}");
}

/// 1v8k.5 (D2): one unit per arity clause, bare names, per-clause metrics.
#[test]
fn clojure_multi_arity_defn_one_unit_per_arity() {
    let module = parse_clojure_source(MULTI_ARITY_SAMPLE);
    let units = &module.units;
    let arities = |name: &str, want: &[&[&str]]| {
        let got: Vec<_> = units.iter().filter(|u| u.name == name).collect();
        assert_eq!(got.len(), want.len(), "{name} arity count");
        for (u, params) in got.iter().zip(want) {
            assert_eq!(param_names(u), *params, "{name} params");
        }
    };
    arities("multi-arity", &[&["x"], &["x", "y"]]);
    arities("private-multi", &[&["a"], &["a", "b"]]);
    arities("single-arity", &[&["n"]]);
    let multi: Vec<_> = units.iter().filter(|u| u.name == "multi-arity").collect();
    assert_eq!(
        (cyclomatic(multi[0]), cyclomatic(multi[1])),
        (1, 2),
        "if counts only in its own clause"
    );
    assert_eq!(
        (function_lines(multi[0]), function_lines(multi[1])),
        (2, 3),
        "per-clause spans"
    );
}
