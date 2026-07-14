use anyhow::{Context, Result};
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use streaming_iterator::StreamingIterator;

use crate::model::{
    Block, Branch, BranchKind, CallSite, CodeUnit, Diagnostic, DiagnosticSeverity, Language,
    Module, Parameter, Span, UnitKind,
};
use crate::plugin::BranchWeights;

pub struct QueryEngine {
    language: tree_sitter::Language,
    lang_kind: Language,
    query: tree_sitter::Query,
    fn_def_idx: u32,
    fn_name_idx: u32,
    fn_params_idx: u32,
    fn_body_idx: u32,
    branch_captures: Vec<(u32, BranchCaptureSpec)>,
    assert_capture_indices: Vec<u32>,
    call_idx: Option<u32>,
    call_callee_idx: Option<u32>,
    assign_idx: Option<u32>,
    call_weights: BTreeMap<String, f64>,
}

#[derive(Clone, Copy)]
struct BranchCaptureSpec {
    kind: BranchKind,
    cyclomatic_weight: u32,
    cognitive_weight: u32,
}

impl QueryEngine {
    #[allow(dead_code)]
    pub fn new(
        language: tree_sitter::Language,
        lang_kind: Language,
        query_source: &str,
    ) -> Result<Self> {
        Self::new_with_branch_weights(
            language,
            lang_kind,
            query_source,
            &BTreeMap::new(),
            &BTreeMap::new(),
        )
    }

    pub fn new_with_branch_weights(
        language: tree_sitter::Language,
        lang_kind: Language,
        query_source: &str,
        branch_weights: &BTreeMap<String, BranchWeights>,
        call_weights: &BTreeMap<String, f64>,
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
                    let weights =
                        branch_weights
                            .get(&capture_name)
                            .copied()
                            .unwrap_or(BranchWeights {
                                cyclomatic: 1,
                                cognitive: 1,
                            });
                    (
                        idx,
                        BranchCaptureSpec {
                            kind: *kind,
                            cyclomatic_weight: weights.cyclomatic,
                            cognitive_weight: weights.cognitive,
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
            call_weights: call_weights.clone(),
        })
    }

    pub fn parse(&self, path: &Path, source: &str) -> Result<(Module, Vec<Diagnostic>)> {
        let source_bytes = source.as_bytes();
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&self.language)
            .context("failed to set language")?;

        let tree = parser
            .parse(source_bytes, None)
            .context("tree-sitter returned no tree")?;

        let root = tree.root_node();
        let mut diagnostics = Vec::new();
        let lines_total = source.lines().count() as u32;

        let (lines_code, lines_comment) = classify_lines(root, lines_total);

        if root.has_error() {
            diagnostics.push(Diagnostic {
                message: format!("Parse errors detected in {}", path.display()),
                span: Some(node_span(root)),
                severity: DiagnosticSeverity::Warning,
            });
            return Ok((
                Module {
                    path: path.to_path_buf(),
                    language: self.lang_kind.clone(),
                    span: Span {
                        start_line: 1,
                        end_line: lines_total,
                    },
                    lines_total,
                    lines_code,
                    lines_comment,
                    units: Vec::new(),
                    imports: Vec::new(),
                },
                diagnostics,
            ));
        }

        let mut query_cursor = tree_sitter::QueryCursor::new();
        let mut functions: Vec<FunctionCapture> = Vec::new();
        let mut captures = CaptureMap::default();

        let mut query_matches = query_cursor.matches(&self.query, root, source_bytes);
        while let Some(m) = query_matches.next() {
            let has_fn_def = m.captures.iter().any(|c| c.index == self.fn_def_idx);

            if has_fn_def {
                let mut fc = FunctionCapture::default();
                for capture in m.captures {
                    if capture.index == self.fn_def_idx {
                        fc.def_node = Some(capture.node);
                    } else if capture.index == self.fn_name_idx {
                        fc.name_node = Some(capture.node);
                    } else if capture.index == self.fn_params_idx {
                        fc.params_node = Some(capture.node);
                    } else if capture.index == self.fn_body_idx {
                        fc.body_node = Some(capture.node);
                    }
                }
                functions.push(fc);
            } else {
                let mut pending_call: Option<(usize, Option<String>)> = None;
                for capture in m.captures {
                    if let Some(spec) = self.branch_capture_for_index(capture.index) {
                        captures.branches.insert(capture.node.id(), spec);
                    } else if self.assert_capture_indices.contains(&capture.index) {
                        captures.assertions.insert(capture.node.id());
                    } else if Some(capture.index) == self.assign_idx {
                        captures.assignments.insert(capture.node.id());
                    } else if Some(capture.index) == self.call_idx {
                        let id = capture.node.id();
                        match &mut pending_call {
                            Some((existing_id, _)) if *existing_id == id => {}
                            _ => pending_call = Some((id, None)),
                        }
                    } else if Some(capture.index) == self.call_callee_idx {
                        let callee = capture
                            .node
                            .utf8_text(source_bytes)
                            .ok()
                            .map(str::to_string);
                        if let Some((_, slot)) = pending_call.as_mut() {
                            *slot = callee;
                        }
                    }
                }
                if let Some((id, callee)) = pending_call {
                    captures.calls.insert(id, callee.unwrap_or_default());
                }
            }
        }

        let units: Vec<CodeUnit> = functions
            .iter()
            .filter_map(|fc| {
                self.build_code_unit(fc, source_bytes, &captures)
                    .map_err(|e| {
                        diagnostics.push(Diagnostic {
                            message: format!("Failed to build CodeUnit: {e}"),
                            span: fc.def_node.map(node_span),
                            severity: DiagnosticSeverity::Error,
                        });
                    })
                    .ok()
            })
            .collect();

        Ok((
            Module {
                path: path.to_path_buf(),
                language: self.lang_kind.clone(),
                span: Span {
                    start_line: 1,
                    end_line: lines_total,
                },
                lines_total,
                lines_code,
                lines_comment,
                units,
                imports: Vec::new(),
            },
            diagnostics,
        ))
    }

    fn branch_capture_for_index(&self, idx: u32) -> Option<BranchCaptureSpec> {
        self.branch_captures
            .iter()
            .find(|(capture_idx, _)| *capture_idx == idx)
            .map(|(_, spec)| *spec)
    }

    fn build_code_unit(
        &self,
        fc: &FunctionCapture,
        source: &[u8],
        captures: &CaptureMap,
    ) -> Result<CodeUnit, String> {
        let def_node = fc.def_node.ok_or("missing definition node")?;
        let name_node = fc.name_node.ok_or("missing name node")?;
        let body_node = fc.body_node.ok_or("missing body node")?;

        let name = name_node
            .utf8_text(source)
            .map_err(|_| "name contains invalid UTF-8")?
            .to_string();

        let kind = determine_unit_kind(def_node, &name);
        let is_exported = !name.starts_with('_');

        let parameters = fc
            .params_node
            .map(|p| extract_params(p, source))
            .unwrap_or_default();

        let body = build_block(body_node, source, captures, 0, &self.call_weights);

        Ok(CodeUnit {
            name,
            kind,
            span: node_span(def_node),
            parameters,
            body,
            is_exported,
            assertions: count_captured_nodes(body_node, &captures.assertions),
        })
    }
}

#[derive(Default)]
struct CaptureMap {
    branches: HashMap<usize, BranchCaptureSpec>,
    calls: HashMap<usize, String>,
    assignments: std::collections::HashSet<usize>,
    assertions: std::collections::HashSet<usize>,
}

#[derive(Default)]
struct FunctionCapture<'a> {
    def_node: Option<tree_sitter::Node<'a>>,
    name_node: Option<tree_sitter::Node<'a>>,
    params_node: Option<tree_sitter::Node<'a>>,
    body_node: Option<tree_sitter::Node<'a>>,
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
            "parameter" => child
                .child_by_field_name("name")
                .and_then(|n| n.utf8_text(source).ok())
                .map(str::to_string),
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
            params.push(Parameter {
                span: node_span(child),
                name,
            });
        }
    }
    params
}

fn build_block(
    block_node: tree_sitter::Node,
    source: &[u8],
    captures: &CaptureMap,
    nesting: u32,
    call_weights: &BTreeMap<String, f64>,
) -> Block {
    let mut children = Vec::new();
    // Check the block_node itself against captures first.
    // This handles languages where the function body IS a branch node
    // (e.g. Clojure: (defn f [x] (if ...)) where the entire body is an if form).
    let visited = visit_child(
        block_node,
        block_node,
        source,
        captures,
        nesting,
        &mut children,
        call_weights,
    );
    if !visited {
        walk_block(
            block_node,
            source,
            captures,
            nesting,
            &mut children,
            call_weights,
        );
    }
    Block {
        span: node_span(block_node),
        nesting,
        children,
    }
}

fn visit_child(
    parent: tree_sitter::Node,
    child: tree_sitter::Node,
    source: &[u8],
    captures: &CaptureMap,
    nesting: u32,
    out: &mut Vec<crate::model::Node>,
    call_weights: &BTreeMap<String, f64>,
) -> bool {
    if let Some(&spec) = captures.branches.get(&child.id()) {
        let sequence_id = match spec.kind {
            BranchKind::LogicalAnd | BranchKind::LogicalOr => Some(parent.id() as u32),
            _ => None,
        };
        out.push(crate::model::Node::Branch(Branch {
            kind: spec.kind,
            span: node_span(child),
            nesting_at: nesting,
            sequence_id,
            cyclomatic_weight: spec.cyclomatic_weight,
            cognitive_weight: spec.cognitive_weight,
        }));
        collect_nested_blocks(child, source, captures, nesting, out, call_weights);
        true
    } else if captures.assignments.contains(&child.id()) {
        out.push(crate::model::Node::Assignment(node_span(child)));
        collect_nested_blocks(child, source, captures, nesting, out, call_weights);
        true
    } else if let Some(callee) = captures.calls.get(&child.id()) {
        let smell_weight = call_weights.get(callee.as_str()).copied().unwrap_or(1.0);
        out.push(crate::model::Node::Call(CallSite {
            callee: callee.clone(),
            span: node_span(child),
            smell_weight,
        }));
        collect_nested_blocks(child, source, captures, nesting, out, call_weights);
        true
    } else {
        false
    }
}

fn walk_block(
    node: tree_sitter::Node,
    source: &[u8],
    captures: &CaptureMap,
    nesting: u32,
    out: &mut Vec<crate::model::Node>,
    call_weights: &BTreeMap<String, f64>,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "function_definition" || child.kind() == "class_definition" {
            continue;
        }
        if !visit_child(node, child, source, captures, nesting, out, call_weights) {
            walk_block(child, source, captures, nesting, out, call_weights);
        }
    }
}

fn collect_nested_blocks(
    branch_node: tree_sitter::Node,
    source: &[u8],
    captures: &CaptureMap,
    nesting: u32,
    out: &mut Vec<crate::model::Node>,
    call_weights: &BTreeMap<String, f64>,
) {
    let mut cursor = branch_node.walk();
    for child in branch_node.children(&mut cursor) {
        if !visit_child(
            branch_node,
            child,
            source,
            captures,
            nesting,
            out,
            call_weights,
        ) {
            if child.kind() == "block" {
                out.push(crate::model::Node::NestedBlock(build_block(
                    child,
                    source,
                    captures,
                    nesting + 1,
                    call_weights,
                )));
            } else {
                collect_nested_blocks(child, source, captures, nesting, out, call_weights);
            }
        }
    }
}

/// Classify each line in the file using tree-sitter node types.
/// Returns (lines_code, lines_comment).
/// A line is a comment line if every leaf node on that line is a comment.
/// Otherwise if any named non-comment leaf touches the line, it is a code line.
fn classify_lines(root: tree_sitter::Node, lines_total: u32) -> (u32, u32) {
    use std::collections::HashSet;

    let mut comment_lines: HashSet<u32> = HashSet::new();
    let mut code_lines: HashSet<u32> = HashSet::new();

    let mut cursor = root.walk();
    loop {
        let node = cursor.node();
        if node.child_count() == 0 {
            // leaf node
            let start = node.start_position().row as u32 + 1;
            let end = node.end_position().row as u32 + 1;
            let is_comment = node.kind().contains("comment");
            for line in start..=end {
                if is_comment {
                    if !code_lines.contains(&line) {
                        comment_lines.insert(line);
                    }
                } else if !node.kind().trim().is_empty() {
                    code_lines.insert(line);
                    comment_lines.remove(&line);
                }
            }
        }

        if cursor.goto_first_child() {
            continue;
        }
        loop {
            if cursor.goto_next_sibling() {
                break;
            }
            if !cursor.goto_parent() {
                return (
                    code_lines.len() as u32,
                    comment_lines.iter().filter(|&&l| l <= lines_total).count() as u32,
                );
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

fn is_nested_definition(node: tree_sitter::Node) -> bool {
    matches!(
        node.kind(),
        "function_definition"
            | "class_definition"
            | "function_declaration"
            | "method_definition"
            | "generator_function_declaration"
            | "arrow_function"
            | "class"
    )
}

fn node_span(node: tree_sitter::Node) -> Span {
    Span {
        start_line: node.start_position().row as u32 + 1,
        end_line: node.end_position().row as u32 + 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;
    use crate::plugin::BranchWeights;
    use std::collections::BTreeMap;

    fn python_language() -> tree_sitter::Language {
        tree_sitter_python::LANGUAGE.into()
    }

    fn python_query_source() -> &'static str {
        include_str!("../languages/python/metrics.scm")
    }

    fn engine() -> QueryEngine {
        QueryEngine::new(python_language(), Language::Python, python_query_source()).unwrap()
    }

    fn weighted_engine(weights: BTreeMap<String, BranchWeights>) -> QueryEngine {
        QueryEngine::new_with_branch_weights(
            python_language(),
            Language::Python,
            python_query_source(),
            &weights,
            &BTreeMap::new(),
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
}
