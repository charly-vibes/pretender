// Purpose: Round-trip integration test for the Python parser against its real
//          metrics.scm query and the python_types_sample.py fixture.
// Responsibilities: Assert TypeDefinition count/kind/span/ordered fields for
//                   Person and Account, decorator-free dataclass capture, and
//                   that CodeUnits stay separate from type members.
// Rationale: Moved verbatim from pretender/src/python.rs unit tests (slice
//            5zo.40) so the source file passes pretender's own structural
//            gate; assertions are unchanged. The fixture round trip is
//            split into focused tests (person fields, account fields, units
//            separation) sharing the parse helper; every assertion is
//            unchanged.

use std::path::Path;

use pretender::metrics::cyclomatic;
use pretender::model::{
    BranchKind, CodeUnit, Language, Module, Node, Parser, Span, TypeDefinition, TypeKind, UnitKind,
};
use pretender::registry::{lookup, GenericParser};

/// The production record for py, through the public registry API.
fn parser_for_py() -> GenericParser {
    lookup("py")
        .map(|record| GenericParser { record })
        .unwrap_or_else(|| panic!("py must be registered"))
}

/// Reusable harness pattern (tasks 6.2): the language's real metrics.scm
/// parses the committed source; asserts a clean parse and returns the module.
fn parse_py(path: &str, source: &'static str) -> Module {
    let (module, diags) = parser_for_py()
        .parse(Path::new(path), source)
        .unwrap_or_else(|e| panic!("fixture must parse: {e:?}"));
    assert!(diags.is_empty(), "fixture must parse cleanly: {diags:?}");
    module
}

/// Types fixture through the production parser (task 6.2 harness).
fn parse_python_types_fixture() -> Module {
    parse_py(
        "python_types_sample.py",
        include_str!("fixtures/python_types_sample.py"),
    )
}

/// Forms fixture through the production parser (1v8k.4 assert-guard).
fn parse_python_forms_fixture() -> Module {
    parse_py(
        "python_forms_sample.py",
        include_str!("fixtures/python_forms_sample.py"),
    )
}

/// Finds the type definition named `name` or panics naming it.
fn find_type<'a>(module: &'a Module, name: &str) -> &'a TypeDefinition {
    module
        .type_definitions
        .iter()
        .find(|t| t.name == name)
        .unwrap_or_else(|| panic!("{name} captured"))
}

#[test]
fn python_types_fixture_round_trip() {
    let module = parse_python_types_fixture();

    assert_eq!(
        module.type_definitions.len(),
        2,
        "both classes captured; methods are not type definitions"
    );

    let person = find_type(&module, "Person");
    // Design D3: @dataclass ships as Class — no decorator refinement.
    assert_eq!(person.kind, TypeKind::Class);
    assert_eq!(person.language, Language::Python);
    assert_eq!(
        person.span,
        Span {
            start_line: 7,
            end_line: 16
        }
    );
    assert!(
        person.name.chars().all(|c| c.is_alphanumeric() || c == '_'),
        "captured name must be bare identifier, no grammar syntax: {:?}",
        person.name
    );
    let person_fields: Vec<(&str, Option<&str>)> = person
        .fields
        .iter()
        .map(|f| (f.name.as_str(), f.type_name.as_deref()))
        .collect();
    assert_eq!(
        person_fields,
        vec![
            ("name", Some("str")),
            ("age", Some("int")),
            ("email", Some("str")),
        ],
        "annotated attributes only, in declaration order"
    );
    assert!(
        person.fields.iter().all(|f| f.name != "greeting"),
        "method-local annotated variables must not become fields"
    );
}

#[test]
fn python_account_class_round_trip() {
    let module = parse_python_types_fixture();

    // Second half of the fixture round trip: the Account class has its own
    // span, and __init__ self.x assignments are not duplicated as fields.
    let account = find_type(&module, "Account");
    assert_eq!(account.kind, TypeKind::Class);
    assert_eq!(
        account.span,
        Span {
            start_line: 19,
            end_line: 27
        }
    );
    let account_fields: Vec<(&str, Option<&str>)> = account
        .fields
        .iter()
        .map(|f| (f.name.as_str(), f.type_name.as_deref()))
        .collect();
    assert_eq!(
        account_fields,
        vec![("balance", Some("float")), ("owner", Some("str"))],
        "__init__ self.x assignments must not be duplicated as fields"
    );
}

#[test]
fn python_units_separate_from_type_members() {
    let module = parse_python_types_fixture();

    // CodeUnits remain separate from type members.
    let mut unit_names: Vec<&str> = module.units.iter().map(|u| u.name.as_str()).collect();
    unit_names.sort_unstable();
    assert_eq!(unit_names, vec!["__init__", "greet"]);
    let greet = module
        .units
        .iter()
        .find(|u| u.name == "greet")
        .unwrap_or_else(|| panic!("greet captured"));
    assert_eq!(greet.kind, UnitKind::Method);
    let ctor = module
        .units
        .iter()
        .find(|u| u.name == "__init__")
        .unwrap_or_else(|| panic!("__init__ captured"));
    assert_eq!(ctor.kind, UnitKind::Ctor);
}

/// Collects branch kinds from a unit's body tree, depth-first.
fn branch_kinds(unit: &CodeUnit) -> Vec<BranchKind> {
    fn walk(block: &pretender::model::Block, out: &mut Vec<BranchKind>) {
        for node in &block.children {
            match node {
                Node::Branch(b) => out.push(b.kind),
                Node::NestedBlock(b) => walk(b, out),
                _ => {}
            }
        }
    }
    let mut out = Vec::new();
    walk(&unit.body, &mut out);
    out
}

fn find_unit<'a>(module: &'a Module, name: &str) -> &'a CodeUnit {
    module
        .units
        .iter()
        .find(|u| u.name == name)
        .unwrap_or_else(|| panic!("{name} not captured"))
}

const MATCH_SAMPLE: &str = "\
def sort_key(item):
    match item:
        case {\"kind\": \"circle\", \"radius\": r}:
            return 3.14 * r * r
        case [x, y] if x > y:
            return x - y
        case _:
            return 0
";

/// Task 3.1: each `case` clause of a Python match statement is one branch
/// (kind match-case). The wildcard and guarded clauses count too; a guard
/// is part of its clause and must not double-count.
#[test]
fn python_match_statement_counts_one_branch_per_case_clause() {
    let module = parse_py("python_branches.py", MATCH_SAMPLE);
    let unit = find_unit(&module, "sort_key");
    assert_eq!(cyclomatic(unit), 4, "3 case clauses + base");
    assert_eq!(
        branch_kinds(unit),
        vec![BranchKind::MatchCase; 3],
        "one match-case branch per case clause"
    );
}

const COMPREHENSION_SAMPLE: &str = "\
def collect(items):
    kept = [x for x in items if x > 0 if x < 100]
    return kept
";

/// Task 3.2: a comprehension's `for` clause is one loop branch and each
/// `if` filter is one comprehension-filter branch; filters never wrap a
/// body (no nesting contribution).
#[test]
fn python_comprehension_counts_for_clause_and_if_filters() {
    let module = parse_py("python_branches.py", COMPREHENSION_SAMPLE);
    let unit = find_unit(&module, "collect");
    assert_eq!(cyclomatic(unit), 4, "1 for clause + 2 if filters + base");
    assert_eq!(
        branch_kinds(unit),
        vec![
            BranchKind::Loop,
            BranchKind::ComprehensionFilter,
            BranchKind::ComprehensionFilter
        ],
        "loop for the for clause, one comprehension-filter branch per if"
    );
}

/// Every Python form in the fixture is captured as a unit with the pinned
/// kind and cyclomatic complexity: async def as a unit, decorated defs with
/// no decorator complexity, @property/@staticmethod as Method, and nested
/// defs as their own unit.
#[test]
fn python_forms_fixture_kinds_and_cyclomatic() {
    let module = parse_python_forms_fixture();
    let expected: Vec<(&str, UnitKind, u32)> = vec![
        ("fetch", UnitKind::Function, 1),
        ("make_greeter", UnitKind::Function, 1),
        ("greet", UnitKind::Function, 2),
        ("size", UnitKind::Method, 1),
        ("tag", UnitKind::Method, 1),
        ("pulse", UnitKind::Method, 1),
        ("shield", UnitKind::Method, 3),
        ("sieve", UnitKind::Method, 6),
    ];
    let got: Vec<(&str, UnitKind, u32)> = module
        .units
        .iter()
        .map(|u| (u.name.as_str(), u.kind, cyclomatic(u)))
        .collect();
    assert_eq!(
        got, expected,
        "each form captured with pinned kind/complexity"
    );
}

/// The nested def's branch and call stay out of the parent's body metrics
/// while being reported on the nested unit itself.
#[test]
fn python_nested_def_excluded_from_parent_body() {
    let module = parse_python_forms_fixture();
    let parent = find_unit(&module, "make_greeter");
    assert_eq!(cyclomatic(parent), 1, "greet's if must not leak in");
    assert!(
        branch_kinds(parent).is_empty(),
        "greet's branch must not appear in the parent body tree"
    );
    let greet = find_unit(&module, "greet");
    assert!(greet.span.start_line > parent.span.start_line);
    assert!(greet.span.end_line < parent.span.end_line);
}

/// except* groups count one catch branch per group; comprehension fors are loops, filters are filters.
#[test]
fn python_except_groups_and_comprehension_forms() {
    let module = parse_python_forms_fixture();
    assert_eq!(
        branch_kinds(find_unit(&module, "shield")),
        vec![BranchKind::Catch; 2],
        "one catch branch per except* group"
    );
    assert_eq!(
        branch_kinds(find_unit(&module, "sieve")),
        vec![
            BranchKind::Loop,
            BranchKind::ComprehensionFilter,
            BranchKind::ComprehensionFilter,
            BranchKind::Loop,
            BranchKind::Loop,
        ]
    );
}
