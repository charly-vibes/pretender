use anyhow::{Context, Result};
use std::collections::{BTreeMap, HashMap};

use crate::model::{
    Block, Branch, BranchKind, CallSite, CodeUnit, Language, Parameter, Span, UnitKind,
};
use crate::plugin::BranchWeights;

pub struct QueryEngine {
    pub(crate) language: tree_sitter::Language,
    pub(crate) lang_kind: Language,
    pub(crate) query: tree_sitter::Query,
    pub(crate) fn_def_idx: u32,
    pub(crate) fn_name_idx: u32,
    pub(crate) fn_params_idx: u32,
    pub(crate) fn_body_idx: u32,
    pub(crate) branch_captures: Vec<(u32, BranchCaptureSpec)>,
    pub(crate) assert_capture_indices: Vec<u32>,
    pub(crate) call_idx: Option<u32>,
    pub(crate) call_callee_idx: Option<u32>,
    pub(crate) assign_idx: Option<u32>,
    pub(crate) import_idx: Option<u32>,
    pub(crate) type_def_idx: Option<u32>,
    pub(crate) type_name_idx: Option<u32>,
    pub(crate) type_field_idx: Option<u32>,
    pub(crate) type_field_name_idx: Option<u32>,
    pub(crate) type_field_type_idx: Option<u32>,
    pub(crate) call_weights: BTreeMap<String, f64>,
}

#[derive(Clone, Copy)]
pub(crate) struct BranchCaptureSpec {
    kind: BranchKind,
    cyclomatic_weight: u32,
    cognitive_weight: u32,
}

/// Weight overrides supplied by a language adapter, passed to
/// [`QueryEngine::new_with_branch_weights`] as one bundle.
pub struct WeightMaps<'a> {
    /// Per-branch-name cyclomatic/cognitive weight overrides.
    pub branch: &'a BTreeMap<String, BranchWeights>,
    /// Per-callee smell-weight overrides.
    pub call: &'a BTreeMap<String, f64>,
}

impl QueryEngine {
    #[allow(dead_code)]
    pub fn new(
        language: tree_sitter::Language,
        lang_kind: Language,
        query_source: &str,
    ) -> Result<Self> {
        let empty_branches = BTreeMap::new();
        let empty_calls = BTreeMap::new();
        Self::new_with_branch_weights(
            language,
            lang_kind,
            query_source,
            &WeightMaps {
                branch: &empty_branches,
                call: &empty_calls,
            },
        )
    }

    pub fn new_with_branch_weights(
        language: tree_sitter::Language,
        lang_kind: Language,
        query_source: &str,
        weights: &WeightMaps<'_>,
    ) -> Result<Self> {
        let query = tree_sitter::Query::new(&language, query_source)
            .map_err(|e| anyhow::anyhow!("failed to compile query: {e}"))?;

        let fn_def_idx = query
            .capture_index_for_name("function.definition")
            .context("query missing @function.definition capture")?;
        let fn_name_idx = query
            .capture_index_for_name("function.name")
            .context("query missing @function.name capture")?;
        let fn_params_idx = query
            .capture_index_for_name("function.parameters")
            .context("query missing @function.parameters capture")?;
        let fn_body_idx = query
            .capture_index_for_name("function.body")
            .context("query missing @function.body capture")?;

        let branch_mapping = [
            ("branch.if", BranchKind::If),
            ("branch.elif", BranchKind::ElseIf),
            ("branch.switch_case", BranchKind::SwitchCase),
            ("branch.loop", BranchKind::Loop),
            ("branch.catch", BranchKind::Catch),
            ("branch.ternary", BranchKind::Ternary),
            ("branch.logical.and", BranchKind::LogicalAnd),
            ("branch.logical.or", BranchKind::LogicalOr),
        ];

        let branch_captures: Vec<(u32, BranchCaptureSpec)> = branch_mapping
            .iter()
            .filter_map(|(name, kind)| {
                query.capture_index_for_name(name).map(|idx| {
                    let capture_name = format!("@{name}");
                    let defaults = BranchWeights {
                        cyclomatic: 1,
                        cognitive: 1,
                    };
                    let w = weights
                        .branch
                        .get(&capture_name)
                        .copied()
                        .unwrap_or(defaults);
                    (
                        idx,
                        BranchCaptureSpec {
                            kind: *kind,
                            cyclomatic_weight: w.cyclomatic,
                            cognitive_weight: w.cognitive,
                        },
                    )
                })
            })
            .collect();

        let assert_capture_indices = query
            .capture_names()
            .iter()
            .enumerate()
            .filter_map(|(idx, name)| name.starts_with("assert.").then_some(idx as u32))
            .collect();

        let call_idx = query.capture_index_for_name("call");
        let call_callee_idx = query.capture_index_for_name("call.callee");
        let assign_idx = query.capture_index_for_name("assign");
        let import_idx = query.capture_index_for_name("import");

        // Type captures are optional (D2): their absence leaves
        // type_definitions empty instead of failing the query,
        // mirroring how @import is handled.
        let type_def_idx = query.capture_index_for_name("type.definition");
        let type_name_idx = query.capture_index_for_name("type.name");
        let type_field_idx = query.capture_index_for_name("type.field");
        let type_field_name_idx = query.capture_index_for_name("type.field.name");
        let type_field_type_idx = query.capture_index_for_name("type.field.type");

        Ok(Self {
            language,
            lang_kind,
            query,
            fn_def_idx,
            fn_name_idx,
            fn_params_idx,
            fn_body_idx,
            branch_captures,
            assert_capture_indices,
            call_idx,
            call_callee_idx,
            assign_idx,
            import_idx,
            type_def_idx,
            type_name_idx,
            type_field_idx,
            type_field_name_idx,
            type_field_type_idx,
            call_weights: weights.call.clone(),
        })
    }

    pub(crate) fn branch_capture_for_index(&self, idx: u32) -> Option<BranchCaptureSpec> {
        self.branch_captures
            .iter()
            .find(|(capture_idx, _)| *capture_idx == idx)
            .map(|(_, spec)| *spec)
    }

    pub(crate) fn build_code_unit(
        &self,
        fc: &FunctionCapture,
        source: &[u8],
        captures: &CaptureMap,
    ) -> Result<CodeUnit, String> {
        let def_node = fc.def_node.ok_or("missing definition node")?;
        let name_node = fc.name_node.ok_or("missing name node")?;
        let body_node = fc.body_node.ok_or("missing body node")?;

        let name = unit_name(name_node, source)?;
        let parameters = unit_parameters(fc, source);
        let body = self.unit_body(body_node, captures);
        let kind = determine_unit_kind(def_node, &name);
        let is_exported = !name.starts_with('_');

        Ok(CodeUnit {
            name,
            kind,
            span: node_span(def_node),
            parameters,
            body,
            is_exported,
            assertions: count_captured_nodes(body_node, &captures.assertions),
            parent_class: None,
            has_test_attribute: has_test_marker(def_node, source),
        })
    }

    fn unit_body(&self, body_node: tree_sitter::Node, captures: &CaptureMap) -> Block {
        let walk_ctx = WalkCtx {
            captures,
            call_weights: &self.call_weights,
            nesting: 0,
        };
        build_block(&walk_ctx, body_node)
    }
}

#[derive(Default)]
pub(crate) struct CaptureMap {
    pub(crate) branches: HashMap<usize, BranchCaptureSpec>,
    pub(crate) calls: HashMap<usize, String>,
    pub(crate) assignments: std::collections::HashSet<usize>,
    pub(crate) assertions: std::collections::HashSet<usize>,
}

#[derive(Default)]
pub(crate) struct FunctionCapture<'a> {
    pub(crate) def_node: Option<tree_sitter::Node<'a>>,
    pub(crate) name_node: Option<tree_sitter::Node<'a>>,
    pub(crate) params_node: Option<tree_sitter::Node<'a>>,
    pub(crate) body_node: Option<tree_sitter::Node<'a>>,
}

fn determine_unit_kind(node: tree_sitter::Node, name: &str) -> UnitKind {
    let mut current = node.parent();
    while let Some(parent) = current {
        match parent.kind() {
            "class_definition" => {
                if name == "__init__" || name == "__new__" {
                    return UnitKind::Ctor;
                }
                return UnitKind::Method;
            }
            "function_definition" => return UnitKind::Function,
            "module" => break,
            _ => {
                current = parent.parent();
            }
        }
    }
    UnitKind::Function
}

fn extract_params(params_node: tree_sitter::Node, source: &[u8]) -> Vec<Parameter> {
    let mut params = Vec::new();
    let mut cursor = params_node.walk();
    for child in params_node.children(&mut cursor) {
        let param_name = match child.kind() {
            "identifier" => child.utf8_text(source).ok().map(str::to_string),
            "typed_parameter" | "default_parameter" | "typed_default_parameter" => child
                .child_by_field_name("name")
                .and_then(|n| n.utf8_text(source).ok())
                .map(str::to_string),
            "parameter" => {
                // Python/Go-style parameters expose a `name` field; Rust
                // (tree-sitter-rust) exposes a `pattern` field instead.
                child
                    .child_by_field_name("name")
                    .or_else(|| child.child_by_field_name("pattern"))
                    .and_then(|pattern| {
                        if pattern.kind() == "self" {
                            return Some("self".to_string());
                        }
                        let mut name = None;
                        let mut c = pattern.walk();
                        for n in pattern.children(&mut c) {
                            if n.kind() == "identifier" {
                                name = n.utf8_text(source).ok().map(str::to_string);
                                break;
                            }
                        }
                        name.or_else(|| pattern.utf8_text(source).ok().map(str::to_string))
                    })
            }
            // Rust `self` parameters (`fn f(&self)` / `fn f(&mut self)`).
            "self_parameter" | "self" => Some("self".to_string()),
            "list_splat_pattern" | "dictionary_splat_pattern" => {
                let prefix = if child.kind() == "list_splat_pattern" {
                    "*"
                } else {
                    "**"
                };
                let mut c = child.walk();
                let ident = child
                    .children(&mut c)
                    .find(|n| n.kind() == "identifier")
                    .and_then(|n| n.utf8_text(source).ok())
                    .map(|s| format!("{prefix}{s}"));
                ident
            }
            _ => None,
        };
        if let Some(name) = param_name {
            let type_name = child
                .child_by_field_name("type")
                .and_then(|n| n.utf8_text(source).ok())
                .map(str::to_string);

            params.push(Parameter {
                span: node_span(child),
                name,
                type_name,
            });
        }
    }
    params
}

/// Shared immutable context threaded through the block walk below:
/// everything the recursive visitors need except the mutable output list.
#[derive(Clone, Copy)]
struct WalkCtx<'a> {
    captures: &'a CaptureMap,
    call_weights: &'a BTreeMap<String, f64>,
    nesting: u32,
}

fn build_block(ctx: &WalkCtx, block_node: tree_sitter::Node) -> Block {
    let mut children = Vec::new();
    // Check the block_node itself against captures first.
    // This handles languages where the function body IS a branch node
    // (e.g. Clojure: (defn f [x] (if ...)) where the entire body is an if form).
    let visited = visit_child(ctx, block_node, block_node, &mut children);
    if !visited {
        walk_block(ctx, block_node, &mut children);
    }
    Block {
        span: node_span(block_node),
        nesting: ctx.nesting,
        children,
    }
}

fn visit_child(
    ctx: &WalkCtx,
    parent: tree_sitter::Node,
    child: tree_sitter::Node,
    out: &mut Vec<crate::model::Node>,
) -> bool {
    if let Some(&spec) = ctx.captures.branches.get(&child.id()) {
        let sequence_id = match spec.kind {
            BranchKind::LogicalAnd | BranchKind::LogicalOr => Some(parent.id() as u32),
            _ => None,
        };
        out.push(crate::model::Node::Branch(Branch {
            kind: spec.kind,
            span: node_span(child),
            nesting_at: ctx.nesting,
            sequence_id,
            cyclomatic_weight: spec.cyclomatic_weight,
            cognitive_weight: spec.cognitive_weight,
        }));
        collect_nested_blocks(ctx, child, out);
        true
    } else if ctx.captures.assignments.contains(&child.id()) {
        out.push(crate::model::Node::Assignment(node_span(child)));
        collect_nested_blocks(ctx, child, out);
        true
    } else if let Some(callee) = ctx.captures.calls.get(&child.id()) {
        let smell_weight = ctx
            .call_weights
            .get(callee.as_str())
            .copied()
            .unwrap_or(1.0);
        out.push(crate::model::Node::Call(CallSite {
            callee: callee.clone(),
            span: node_span(child),
            smell_weight,
        }));
        collect_nested_blocks(ctx, child, out);
        true
    } else {
        false
    }
}

fn walk_block(ctx: &WalkCtx, node: tree_sitter::Node, out: &mut Vec<crate::model::Node>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if is_nested_definition(child) {
            continue;
        }
        if !visit_child(ctx, node, child, out) {
            walk_block(ctx, child, out);
        }
    }
}

fn collect_nested_blocks(
    ctx: &WalkCtx,
    branch_node: tree_sitter::Node,
    out: &mut Vec<crate::model::Node>,
) {
    let mut cursor = branch_node.walk();
    for child in branch_node.children(&mut cursor) {
        if is_nested_definition(child) {
            continue;
        }
        if !visit_child(ctx, branch_node, child, out) {
            if child.kind() == "block" {
                let inner = WalkCtx {
                    nesting: ctx.nesting + 1,
                    ..*ctx
                };
                out.push(crate::model::Node::NestedBlock(build_block(&inner, child)));
            } else {
                collect_nested_blocks(ctx, child, out);
            }
        }
    }
}

fn count_captured_nodes(
    node: tree_sitter::Node,
    captured_ids: &std::collections::HashSet<usize>,
) -> u32 {
    let mut count = u32::from(captured_ids.contains(&node.id()));
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if is_nested_definition(child) {
            continue;
        }
        count += count_captured_nodes(child, captured_ids);
    }
    count
}

/// True when the definition carries a test-registration attribute
/// (e.g. Rust `#[test]`). Attributes precede the definition as siblings;
/// scanning stops at the first non-attribute sibling so an outer unit
/// never claims another unit's marker.
fn has_test_marker(def_node: tree_sitter::Node, source: &[u8]) -> bool {
    let mut sibling = def_node.prev_sibling();
    while let Some(node) = sibling {
        if node.kind() != "attribute_item" {
            return false;
        }
        if node
            .utf8_text(source)
            .map(|text| text.contains("test"))
            .unwrap_or(false)
        {
            return true;
        }
        sibling = node.prev_sibling();
    }
    false
}

fn is_nested_definition(node: tree_sitter::Node) -> bool {
    matches!(
        node.kind(),
        "function_definition"
            | "class_definition"
            | "function_declaration"
            | "method_definition"
            | "method_declaration"
            | "generator_function_declaration"
            | "arrow_function"
            | "function_item"
            | "class"
            // Anonymous function literals / closures across languages
            | "func_literal"           // Go
            | "closure_expression"     // Rust
            | "lambda"                 // Python
            | "lambda_expression"       // Java, C++, C#
            | "anonymous_method_expression" // C#
    )
}

pub(crate) fn node_span(node: tree_sitter::Node) -> Span {
    Span {
        start_line: node.start_position().row as u32 + 1,
        end_line: node.end_position().row as u32 + 1,
    }
}

fn unit_name(name_node: tree_sitter::Node, source: &[u8]) -> Result<String, String> {
    Ok(name_node
        .utf8_text(source)
        .map_err(|_| "name contains invalid UTF-8")?
        .to_string())
}

fn unit_parameters(fc: &FunctionCapture, source: &[u8]) -> Vec<Parameter> {
    fc.params_node
        .map(|p| extract_params(p, source))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;
    use crate::plugin::BranchWeights;
    use std::collections::BTreeMap;
    use std::path::Path;

    fn python_language() -> tree_sitter::Language {
        tree_sitter_python::LANGUAGE.into()
    }

    fn python_query_source() -> &'static str {
        include_str!("../languages/python/metrics.scm")
    }

    fn engine() -> QueryEngine {
        QueryEngine::new(python_language(), Language::Python, python_query_source()).unwrap()
    }

    fn rust_engine() -> QueryEngine {
        QueryEngine::new(
            tree_sitter_rust::LANGUAGE.into(),
            Language::Rust,
            include_str!("../languages/rust/metrics.scm"),
        )
        .unwrap()
    }

    fn go_engine() -> QueryEngine {
        QueryEngine::new(
            tree_sitter_go::LANGUAGE.into(),
            Language::Go,
            include_str!("../languages/go/metrics.scm"),
        )
        .unwrap()
    }

    fn c_engine() -> QueryEngine {
        QueryEngine::new(
            tree_sitter_c::LANGUAGE.into(),
            Language::C,
            include_str!("../languages/c/metrics.scm"),
        )
        .unwrap()
    }

    fn julia_engine() -> QueryEngine {
        QueryEngine::new(
            tree_sitter_julia::LANGUAGE.into(),
            Language::Julia,
            include_str!("../languages/julia/metrics.scm"),
        )
        .unwrap()
    }

    #[test]
    fn rust_assertion_macros_count_as_assertions() {
        let engine = rust_engine();
        let source = "#[test]\nfn adds() {\n    let a = 2;\n    assert_eq!(a, 2);\n    assert!(a > 0);\n    assert_ne!(a, 3);\n    debug_assert_eq!(a, 2);\n}\n";
        let (module, _) = engine.parse(Path::new("test.rs"), source).unwrap();
        assert_eq!(module.units[0].assertions, 4);
    }

    #[test]
    fn rust_non_assertion_macros_do_not_count() {
        let engine = rust_engine();
        let source = "#[test]\nfn formats() {\n    let s = format!(\"{}\", 1);\n    vec![1, 2];\n    panic_if_bad(s);\n}\n\nfn panic_if_bad<T>(_: T) {}\n";
        let (module, _) = engine.parse(Path::new("test.rs"), source).unwrap();
        let unit = module.units.iter().find(|u| u.name == "formats").unwrap();
        assert_eq!(unit.assertions, 0);
    }

    #[test]
    fn julia_test_macros_count_as_assertions() {
        let engine = julia_engine();
        let source = "function check(x)\n    @test x > 0\n    @test_throws DomainError sqrt(-1)\n    @test_broken x < 0\n    @test_logs (:info) log_it(x)\n    @assert x > 0\n    return x\nend\n";
        let (module, _) = engine.parse(Path::new("test.jl"), source).unwrap();
        assert_eq!(module.units[0].assertions, 5);
    }

    #[test]
    fn julia_non_test_macros_do_not_count() {
        let engine = julia_engine();
        let source = "function show_it(x)\n    @show x\n    @info \"hello\" x\n    return x\nend\n";
        let (module, _) = engine.parse(Path::new("test.jl"), source).unwrap();
        assert_eq!(module.units[0].assertions, 0);
    }

    #[test]
    fn underscore_prefixed_unit_is_not_exported() {
        let engine = rust_engine();
        let source = "fn _helper() {}\n\nfn helper() {}\n";
        let (module, _) = engine.parse(Path::new("test.rs"), source).unwrap();
        let hidden = module.units.iter().find(|u| u.name == "_helper").unwrap();
        let public = module.units.iter().find(|u| u.name == "helper").unwrap();
        assert!(!hidden.is_exported);
        assert!(public.is_exported);
    }

    #[test]
    fn parsed_call_site_gets_default_weight() {
        let engine = rust_engine();
        let source = "fn calls_helper() {\n    helper(1);\n}\n";
        let (module, _) = engine.parse(Path::new("test.rs"), source).unwrap();
        let unit = module
            .units
            .iter()
            .find(|u| u.name == "calls_helper")
            .unwrap();
        let call = unit
            .body
            .children
            .iter()
            .find_map(|n| match n {
                Node::Call(c) if c.callee == "helper" => Some(c),
                _ => None,
            })
            .expect("call site captured");
        assert_eq!(call.smell_weight, 1.0);
    }

    fn weighted_engine(weights: BTreeMap<String, BranchWeights>) -> QueryEngine {
        let empty_calls = BTreeMap::new();
        QueryEngine::new_with_branch_weights(
            python_language(),
            Language::Python,
            python_query_source(),
            &WeightMaps {
                branch: &weights,
                call: &empty_calls,
            },
        )
        .unwrap()
    }

    // --- Phase 1: Query engine scaffolding ---

    #[test]
    fn compiles_valid_query() {
        let result = QueryEngine::new(python_language(), Language::Python, python_query_source());
        assert!(result.is_ok(), "valid query should compile");
    }

    #[test]
    fn rejects_invalid_query() {
        let result = QueryEngine::new(
            python_language(),
            Language::Python,
            "(invalid_node_xxx) @bogus",
        );
        assert!(result.is_err(), "invalid query should fail");
    }

    // --- Phase 2: Function extraction via captures ---

    #[test]
    fn extracts_single_function() {
        let engine = engine();
        let source = "def hello(x):\n    return x\n";
        let (module, diags) = engine.parse(Path::new("test.py"), source).unwrap();

        assert!(diags.is_empty());
        assert_eq!(module.units.len(), 1);
        assert_eq!(module.units[0].name, "hello");
        assert_eq!(module.units[0].kind, UnitKind::Function);
        assert_eq!(module.units[0].span.start_line, 1);
        assert_eq!(module.units[0].span.end_line, 2);
    }

    #[test]
    fn extracts_multiple_functions() {
        let engine = engine();
        let source = "def foo():\n    pass\n\ndef bar():\n    pass\n";
        let (module, _) = engine.parse(Path::new("test.py"), source).unwrap();

        assert_eq!(module.units.len(), 2);
        let names: Vec<&str> = module.units.iter().map(|u| u.name.as_str()).collect();
        assert_eq!(names, vec!["foo", "bar"]);
    }

    #[test]
    fn extracts_function_parameters() {
        let engine = engine();
        let source = "def greet(name, age):\n    pass\n";
        let (module, _) = engine.parse(Path::new("test.py"), source).unwrap();

        assert_eq!(module.units[0].parameters.len(), 2);
        assert_eq!(module.units[0].parameters[0].name, "name");
        assert_eq!(module.units[0].parameters[1].name, "age");
    }

    #[test]
    fn detects_class_methods() {
        let engine = engine();
        let source = "class Foo:\n    def bar(self):\n        pass\n";
        let (module, _) = engine.parse(Path::new("test.py"), source).unwrap();

        assert_eq!(module.units.len(), 1);
        assert_eq!(module.units[0].name, "bar");
        assert_eq!(module.units[0].kind, UnitKind::Method);
    }

    #[test]
    fn detects_constructor() {
        let engine = engine();
        let source = "class Foo:\n    def __init__(self):\n        pass\n";
        let (module, _) = engine.parse(Path::new("test.py"), source).unwrap();

        assert_eq!(module.units[0].name, "__init__");
        assert_eq!(module.units[0].kind, UnitKind::Ctor);
    }

    #[test]
    fn private_functions_not_exported() {
        let engine = engine();
        let source = "def _private():\n    pass\n\ndef public():\n    pass\n";
        let (module, _) = engine.parse(Path::new("test.py"), source).unwrap();

        assert!(!module.units[0].is_exported);
        assert!(module.units[1].is_exported);
    }

    // --- Phase 3: Branch extraction via captures ---

    #[test]
    fn captures_if_branch() {
        let engine = engine();
        let source = "def f():\n    if True:\n        pass\n";
        let (module, _) = engine.parse(Path::new("test.py"), source).unwrap();

        let branches = collect_branches(&module.units[0].body);
        assert_eq!(branches.len(), 1);
        assert_eq!(branches[0].kind, BranchKind::If);
    }

    #[test]
    fn captures_elif_branch() {
        let engine = engine();
        let source = "def f():\n    if True:\n        pass\n    elif False:\n        pass\n";
        let (module, _) = engine.parse(Path::new("test.py"), source).unwrap();

        let branches = collect_branches_recursive(&module.units[0].body);
        let kinds: Vec<BranchKind> = branches.iter().map(|b| b.kind).collect();
        assert!(kinds.contains(&BranchKind::If));
        assert!(kinds.contains(&BranchKind::ElseIf));
    }

    #[test]
    fn captures_loop_branch() {
        let engine = engine();
        let source = "def f():\n    for i in range(10):\n        pass\n";
        let (module, _) = engine.parse(Path::new("test.py"), source).unwrap();

        let branches = collect_branches(&module.units[0].body);
        assert_eq!(branches.len(), 1);
        assert_eq!(branches[0].kind, BranchKind::Loop);
    }

    #[test]
    fn captures_while_loop() {
        let engine = engine();
        let source = "def f():\n    while True:\n        pass\n";
        let (module, _) = engine.parse(Path::new("test.py"), source).unwrap();

        let branches = collect_branches(&module.units[0].body);
        assert_eq!(branches.len(), 1);
        assert_eq!(branches[0].kind, BranchKind::Loop);
    }

    #[test]
    fn captures_except_as_catch() {
        let engine = engine();
        let source = "def f():\n    try:\n        pass\n    except ValueError:\n        pass\n";
        let (module, _) = engine.parse(Path::new("test.py"), source).unwrap();

        let branches = collect_branches_recursive(&module.units[0].body);
        assert_eq!(branches.len(), 1);
        assert_eq!(branches[0].kind, BranchKind::Catch);
    }

    #[test]
    fn captures_logical_and() {
        let engine = engine();
        let source = "def f(a, b):\n    if a and b:\n        pass\n";
        let (module, _) = engine.parse(Path::new("test.py"), source).unwrap();

        let branches = collect_branches_recursive(&module.units[0].body);
        let kinds: Vec<BranchKind> = branches.iter().map(|b| b.kind).collect();
        assert!(kinds.contains(&BranchKind::If));
        assert!(kinds.contains(&BranchKind::LogicalAnd));
    }

    #[test]
    fn captures_logical_or() {
        let engine = engine();
        let source = "def f(a, b):\n    if a or b:\n        pass\n";
        let (module, _) = engine.parse(Path::new("test.py"), source).unwrap();

        let branches = collect_branches_recursive(&module.units[0].body);
        let kinds: Vec<BranchKind> = branches.iter().map(|b| b.kind).collect();
        assert!(kinds.contains(&BranchKind::LogicalOr));
    }

    #[test]
    fn captures_ternary() {
        let engine = engine();
        let source = "def f(x):\n    return x if x > 0 else -x\n";
        let (module, _) = engine.parse(Path::new("test.py"), source).unwrap();

        let branches = collect_branches_recursive(&module.units[0].body);
        assert_eq!(branches.len(), 1);
        assert_eq!(branches[0].kind, BranchKind::Ternary);
    }

    #[test]
    fn nested_branches_have_correct_nesting() {
        let engine = engine();
        let source = "def f():\n    if True:\n        if False:\n            pass\n";
        let (module, _) = engine.parse(Path::new("test.py"), source).unwrap();

        let branches = collect_branches_recursive(&module.units[0].body);
        assert_eq!(branches.len(), 2);
        assert_eq!(branches[0].nesting_at, 0);
        assert_eq!(branches[1].nesting_at, 1);
    }

    #[test]
    fn custom_branch_weights_override_cognitive_scores() {
        let engine = weighted_engine(BTreeMap::from([(
            "@branch.if".to_string(),
            BranchWeights {
                cyclomatic: 1,
                cognitive: 3,
            },
        )]));
        let source = "def f():\n    if True:\n        pass\n";
        let (module, _) = engine.parse(Path::new("test.py"), source).unwrap();

        assert_eq!(crate::metrics::cognitive(&module.units[0]), 3);
    }

    #[test]
    fn counts_assertions_per_code_unit() {
        let engine = engine();
        let source = "def test_assertions():\n    assert ok\n    self.assertEqual(a, b)\n    pytest.raises(ValueError)\n";
        let (module, _) = engine.parse(Path::new("test.py"), source).unwrap();

        assert_eq!(module.units[0].assertions, 3);
    }

    #[test]
    fn nested_function_assertions_do_not_count_toward_parent() {
        let engine = engine();
        let source = "def test_outer():\n    def helper():\n        assert True\n    helper()\n";
        let (module, _) = engine.parse(Path::new("test.py"), source).unwrap();

        let outer = module
            .units
            .iter()
            .find(|unit| unit.name == "test_outer")
            .unwrap();
        let inner = module
            .units
            .iter()
            .find(|unit| unit.name == "helper")
            .unwrap();
        assert_eq!(outer.assertions, 0);
        assert_eq!(inner.assertions, 1);
    }

    // --- Phase 4: Full fixture integration ---

    #[test]
    fn parse_python_fixture() {
        let engine = engine();
        let fixture = include_str!("../tests/fixtures/python_simple.py");
        let (module, diags) = engine
            .parse(Path::new("python_simple.py"), fixture)
            .unwrap();

        assert!(diags.is_empty(), "fixture should parse without diagnostics");
        assert!(
            module.units.len() >= 2,
            "fixture must produce at least 2 CodeUnits, got {}",
            module.units.len()
        );

        let names: Vec<&str> = module.units.iter().map(|u| u.name.as_str()).collect();
        assert!(names.contains(&"simple"));
        assert!(names.contains(&"with_branch"));
        assert!(names.contains(&"complex_func"));
        assert!(names.contains(&"__init__"));
        assert!(names.contains(&"method_simple"));
        assert!(names.contains(&"method_with_loop"));
    }

    #[test]
    fn fixture_metrics_match_expected() {
        let engine = engine();
        let fixture = include_str!("../tests/fixtures/python_simple.py");
        let (module, _) = engine
            .parse(Path::new("python_simple.py"), fixture)
            .unwrap();

        let cc = crate::metrics::CyclomaticComplexity;
        let expected = vec![
            ("simple", 1),
            ("with_branch", 2),
            ("complex_func", 6),
            ("__init__", 1),
            ("method_simple", 1),
            ("method_with_loop", 3),
        ];

        for (name, expected_cc) in expected {
            let unit = module
                .units
                .iter()
                .find(|u| u.name == name)
                .unwrap_or_else(|| panic!("missing CodeUnit: {name}"));
            let actual = cc.calculate(unit);
            assert_eq!(
                actual, expected_cc,
                "{name}: expected CC={expected_cc}, got CC={actual}"
            );
        }
    }

    // --- ABC: assignments + calls are captured ---

    #[test]
    fn captures_assignments_and_calls_for_abc() {
        let engine = engine();
        let source = "def f(x):\n    y = x + 1\n    z = foo(y)\n    return bar(z)\n";
        let (module, _) = engine.parse(Path::new("test.py"), source).unwrap();

        let unit = &module.units[0];
        let abc = crate::metrics::abc(unit);
        assert!(
            abc > 0.0,
            "ABC must reflect assignment + call captures; got {abc}",
        );

        let (mut assigns, mut calls) = (0, 0);
        collect_assigns_and_calls(&unit.body, &mut assigns, &mut calls);
        assert!(assigns >= 2, "expected ≥2 assignments, got {assigns}");
        assert!(calls >= 2, "expected ≥2 calls, got {calls}");
    }

    fn collect_assigns_and_calls(block: &Block, assigns: &mut u32, calls: &mut u32) {
        for child in &block.children {
            match child {
                crate::model::Node::Assignment(_) => *assigns += 1,
                crate::model::Node::Call(_) => *calls += 1,
                crate::model::Node::NestedBlock(b) => collect_assigns_and_calls(b, assigns, calls),
                _ => {}
            }
        }
    }

    // --- Phase 5: Error handling ---

    #[test]
    fn parse_error_emits_diagnostic_and_skips() {
        let engine = engine();
        let bad_source = "def f(\n    # unclosed paren, malformed\n";
        let (module, diags) = engine.parse(Path::new("bad.py"), bad_source).unwrap();

        assert!(!diags.is_empty(), "should emit at least one diagnostic");
        assert!(
            module.units.is_empty(),
            "should skip file with parse errors (no partial Module)"
        );
    }

    // --- Helpers ---

    fn collect_branches(block: &Block) -> Vec<&Branch> {
        block
            .children
            .iter()
            .filter_map(|n| match n {
                crate::model::Node::Branch(b) => Some(b),
                _ => None,
            })
            .collect()
    }

    fn collect_branches_recursive(block: &Block) -> Vec<&Branch> {
        let mut branches = Vec::new();
        for child in &block.children {
            match child {
                crate::model::Node::Branch(b) => branches.push(b),
                crate::model::Node::NestedBlock(b) => {
                    branches.extend(collect_branches_recursive(b))
                }
                _ => {}
            }
        }
        branches
    }

    // --- Phase 6: Type captures ---

    #[test]
    fn type_captures_populate_type_definition() {
        // Contract: universal-code-model / type-captures-populate-type-definition —
        // definition and field captures arrive in separate matches, so fields
        // must bind to the innermost enclosing @type.definition. The tuple
        // field type is an anonymous inner type: captured as declared text,
        // never as its own TypeDefinition. The method stays a CodeUnit.
        let query_source = r#"
(function_item
  name: (identifier) @function.name
  parameters: (parameters) @function.parameters
  body: (block) @function.body) @function.definition

(struct_item
  name: (type_identifier) @type.name) @type.definition

(field_declaration
  name: (field_identifier) @type.field.name
  type: [(type_identifier) (tuple_type)] @type.field.type) @type.field
"#;
        let engine = QueryEngine::new(
            tree_sitter_rust::LANGUAGE.into(),
            Language::Rust,
            query_source,
        )
        .unwrap();
        let source = "struct Outer {\n    name: String,\n    inner: (i32, bool),\n}\n\nimpl Outer {\n    fn method(&self) -> u32 {\n        self.name.len() as u32\n    }\n}\n";
        let (module, diags) = engine.parse(Path::new("test.rs"), source).unwrap();

        assert!(diags.is_empty(), "type captures must not emit diagnostics");

        assert_eq!(
            module.type_definitions.len(),
            1,
            "exactly one TypeDefinition"
        );
        let def = &module.type_definitions[0];
        assert_eq!(def.name, "Outer");
        assert_eq!(def.kind, TypeKind::Struct);
        assert_eq!(def.language, Language::Rust);
        assert_eq!(
            def.span,
            Span {
                start_line: 1,
                end_line: 4
            }
        );

        let field_names: Vec<&str> = def.fields.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(
            field_names,
            vec!["name", "inner"],
            "only struct-level fields, in declaration order"
        );
        assert_eq!(def.fields[0].type_name.as_deref(), Some("String"));
        assert_eq!(
            def.fields[1].type_name.as_deref(),
            Some("(i32, bool)"),
            "anonymous inner type captured as declared text"
        );
        assert!(
            def.fields.iter().all(|f| f.name != "method"),
            "methods must not be duplicated as fields"
        );

        assert_eq!(module.units.len(), 1);
        assert_eq!(module.units[0].name, "method");
        assert_eq!(module.units[0].kind, UnitKind::Function);
    }

    #[test]
    fn c_anonymous_type_members_not_attributed_to_enclosing_struct() {
        // Contract: universal-code-model — members of deliberately
        // uncaptured nested types (C/C++ anonymous unions/structs, design
        // D3) belong to the inner type, not to the enclosing captured
        // struct. File-global field queries must not leak raw/scaled/calib
        // into Sensor's field list.
        let engine = c_engine();
        let source = "\
struct Sensor {
    int reading;
    union {
        int raw;
        double scaled;
    };
    struct {
        int calib;
    };
    int unit;
};
";
        let (module, diags) = engine.parse(Path::new("sensor.c"), source).unwrap();

        assert!(diags.is_empty(), "fixture must parse cleanly: {diags:?}");
        assert_eq!(
            module.type_definitions.len(),
            1,
            "anonymous union/struct stay uncaptured (design D3)"
        );
        let sensor = &module.type_definitions[0];
        let field_names: Vec<&str> = sensor.fields.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(
            field_names,
            vec!["reading", "unit"],
            "anonymous union/struct members are not Sensor fields"
        );
    }

    #[test]
    fn go_inline_struct_members_not_attributed_to_enclosing_struct() {
        // Contract: universal-code-model — an inline struct type
        // (uncaptured per design D3) nested inside a captured struct's
        // field must not leak its members into the enclosing definition,
        // and an inline struct inside a function stays outside the type
        // model entirely.
        let engine = go_engine();
        let source = "\
package main

type Config struct {
    Options struct {
        Verbose bool
    }
    Name string
}

func run() {
    var local struct {
        Depth int
    }
    _ = local
}
";
        let (module, diags) = engine.parse(Path::new("config.go"), source).unwrap();

        assert!(diags.is_empty(), "fixture must parse cleanly: {diags:?}");
        assert_eq!(
            module.type_definitions.len(),
            1,
            "inline struct types are not captured (design D3)"
        );
        let config = &module.type_definitions[0];
        let field_names: Vec<&str> = config.fields.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(
            field_names,
            vec!["Options", "Name"],
            "inline struct members are not Config fields"
        );
    }

    #[test]
    fn adapter_without_type_captures_stays_valid() {
        // Contract: universal-code-model / adapter-without-type-captures-stays-valid —
        // a query with only @function.* captures parses to an empty
        // type_definitions list with no error diagnostic.
        let query_source = r#"
(function_definition
  name: (identifier) @function.name
  parameters: (parameters) @function.parameters
  body: (block) @function.body) @function.definition
"#;
        let engine = QueryEngine::new(
            tree_sitter_python::LANGUAGE.into(),
            Language::Python,
            query_source,
        )
        .unwrap();
        let source = "def hello(x):\n    return x\n";
        let (module, diags) = engine.parse(Path::new("test.py"), source).unwrap();

        assert!(
            diags.is_empty(),
            "missing type captures must not be a parse failure"
        );
        assert!(module.type_definitions.is_empty());
        assert_eq!(module.units.len(), 1);
        assert_eq!(module.units[0].name, "hello");
    }

    #[test]
    fn rust_types_fixture_round_trip() {
        // Reusable harness pattern (tasks 6.1): the language's real
        // metrics.scm parses a fixture file; assert TypeDefinition
        // count/kind/ordered fields and that CodeUnits stay separate.
        // Sibling language batches copy this test + fixture pair verbatim.
        let engine = rust_engine();
        let source = include_str!("../tests/fixtures/rust_types_sample.rs");
        let (module, diags) = engine
            .parse(Path::new("rust_types_sample.rs"), source)
            .unwrap();

        assert!(diags.is_empty(), "fixture must parse cleanly: {diags:?}");

        assert_eq!(
            module.type_definitions.len(),
            2,
            "both structs captured, impl/fn items are not type definitions"
        );

        let person = module
            .type_definitions
            .iter()
            .find(|t| t.name == "Person")
            .expect("Person captured");
        assert_eq!(person.kind, TypeKind::Struct);
        assert_eq!(person.language, Language::Rust);
        assert_eq!(
            person.span,
            Span {
                start_line: 1,
                end_line: 5
            }
        );
        let person_fields: Vec<(&str, Option<&str>)> = person
            .fields
            .iter()
            .map(|f| (f.name.as_str(), f.type_name.as_deref()))
            .collect();
        assert_eq!(
            person_fields,
            vec![
                ("name", Some("String")),
                ("age", Some("u8")),
                ("email", Some("String")),
            ],
            "named fields only, in declaration order"
        );

        let pair = module
            .type_definitions
            .iter()
            .find(|t| t.name == "Pair")
            .expect("generic struct captured");
        assert_eq!(pair.kind, TypeKind::Struct);
        assert!(
            pair.name.chars().all(|c| c.is_alphanumeric() || c == '_'),
            "captured name must be bare identifier, no grammar syntax: {:?}",
            pair.name
        );
        let pair_fields: Vec<(&str, Option<&str>)> = pair
            .fields
            .iter()
            .map(|f| (f.name.as_str(), f.type_name.as_deref()))
            .collect();
        assert_eq!(
            pair_fields,
            vec![("first", Some("T")), ("second", Some("T"))],
            "generic parameter text never leaks into names or type names"
        );

        // CodeUnits remain separate from type members.
        assert_eq!(module.units.len(), 2);
        let mut unit_names: Vec<&str> = module.units.iter().map(|u| u.name.as_str()).collect();
        unit_names.sort_unstable();
        assert_eq!(unit_names, vec!["greet", "standalone"]);
        assert!(
            person.fields.iter().all(|f| f.name != "greet"),
            "methods must not be duplicated as fields"
        );
    }

    #[test]
    fn go_types_fixture_round_trip() {
        let engine = go_engine();
        let source = include_str!("../tests/fixtures/go_types_sample.go");
        let (module, diags) = engine
            .parse(Path::new("go_types_sample.go"), source)
            .unwrap();

        assert!(diags.is_empty(), "fixture must parse cleanly: {diags:?}");

        assert_eq!(
            module.type_definitions.len(),
            2,
            "both struct type specs captured; funcs are not type definitions"
        );

        let person = module
            .type_definitions
            .iter()
            .find(|t| t.name == "Person")
            .expect("Person captured");
        assert_eq!(person.kind, TypeKind::Struct);
        assert_eq!(person.language, Language::Go);
        assert_eq!(
            person.span,
            Span {
                start_line: 3,
                end_line: 7
            }
        );
        let person_fields: Vec<(&str, Option<&str>)> = person
            .fields
            .iter()
            .map(|f| (f.name.as_str(), f.type_name.as_deref()))
            .collect();
        assert_eq!(
            person_fields,
            vec![
                ("Name", Some("string")),
                ("Age", Some("uint8")),
                ("Email", Some("string")),
            ],
            "named fields only, in declaration order"
        );

        let bar = module
            .type_definitions
            .iter()
            .find(|t| t.name == "Bar")
            .expect("generic struct captured");
        assert_eq!(bar.kind, TypeKind::Struct);
        assert!(
            bar.name.chars().all(|c| c.is_alphanumeric() || c == '_'),
            "captured name must be bare identifier, no grammar syntax: {:?}",
            bar.name
        );
        let bar_fields: Vec<(&str, Option<&str>)> = bar
            .fields
            .iter()
            .map(|f| (f.name.as_str(), f.type_name.as_deref()))
            .collect();
        assert_eq!(
            bar_fields,
            vec![("First", Some("T")), ("Second", Some("T"))],
            "type parameter syntax must not leak into captured values"
        );

        // CodeUnits remain separate from type members.
        assert_eq!(module.units.len(), 2);
        let mut unit_names: Vec<&str> = module.units.iter().map(|u| u.name.as_str()).collect();
        unit_names.sort_unstable();
        assert_eq!(unit_names, vec!["Greet", "Standalone"]);
        assert!(
            person.fields.iter().all(|f| f.name != "Greet"),
            "methods must not be duplicated as fields"
        );
    }
}
