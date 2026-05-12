use anyhow::{Context, Result};
use std::path::Path;
use tree_sitter::Node;

use crate::model::{
    Block, Branch, BranchKind, CodeUnit, Diagnostic, DiagnosticSeverity, Language, Module,
    Parameter, Parser, Span, UnitKind,
};

pub struct PythonParser;

impl Parser for PythonParser {
    fn parse(&self, path: &Path, source: &str) -> Result<(Module, Vec<Diagnostic>)> {
        let source_bytes = source.as_bytes();
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_python::LANGUAGE.into())
            .context("failed to load Python grammar")?;

        let tree = parser
            .parse(source_bytes, None)
            .context("tree-sitter returned no tree")?;

        let root = tree.root_node();
        let mut diagnostics = Vec::new();

        if root.has_error() {
            diagnostics.push(Diagnostic {
                message: format!("Parse errors detected in {}", path.display()),
                span: Some(node_span(root)),
                severity: DiagnosticSeverity::Warning,
            });
        }

        let lines_total = source.lines().count() as u32;
        let units = collect_units(root, source_bytes, &mut diagnostics);

        Ok((
            Module {
                path: path.to_path_buf(),
                language: Language::Python,
                span: Span {
                    start_line: 1,
                    end_line: lines_total,
                },
                lines_total,
                lines_code: 0,
                lines_comment: 0,
                units,
                imports: Vec::new(),
            },
            diagnostics,
        ))
    }

    fn extensions(&self) -> &[&str] {
        &["py"]
    }
}

fn collect_units<'a>(
    node: Node<'a>,
    source: &[u8],
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<CodeUnit> {
    let mut units = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "function_definition" => match extract_unit(child, source, UnitKind::Function) {
                Ok(unit) => units.push(unit),
                Err(e) => diagnostics.push(Diagnostic {
                    message: format!("Failed to extract function: {e}"),
                    span: Some(node_span(child)),
                    severity: DiagnosticSeverity::Error,
                }),
            },
            "decorated_definition" => {
                let mut c = child.walk();
                for inner in child.children(&mut c) {
                    if inner.kind() == "function_definition" {
                        match extract_unit(inner, source, UnitKind::Function) {
                            Ok(unit) => units.push(unit),
                            Err(e) => diagnostics.push(Diagnostic {
                                message: format!("Failed to extract decorated function: {e}"),
                                span: Some(node_span(inner)),
                                severity: DiagnosticSeverity::Error,
                            }),
                        }
                    }
                }
            }
            "class_definition" => {
                if let Some(body) = child.child_by_field_name("body") {
                    let mut bc = body.walk();
                    for member in body.children(&mut bc) {
                        match member.kind() {
                            "function_definition" => {
                                let kind = method_kind(member, source);
                                match extract_unit(member, source, kind) {
                                    Ok(unit) => units.push(unit),
                                    Err(e) => diagnostics.push(Diagnostic {
                                        message: format!("Failed to extract method: {e}"),
                                        span: Some(node_span(member)),
                                        severity: DiagnosticSeverity::Error,
                                    }),
                                }
                            }
                            "decorated_definition" => {
                                let mut dc = member.walk();
                                for inner in member.children(&mut dc) {
                                    if inner.kind() == "function_definition" {
                                        let kind = method_kind(inner, source);
                                        match extract_unit(inner, source, kind) {
                                            Ok(unit) => units.push(unit),
                                            Err(e) => diagnostics.push(Diagnostic {
                                                message: format!(
                                                    "Failed to extract decorated method: {e}"
                                                ),
                                                span: Some(node_span(inner)),
                                                severity: DiagnosticSeverity::Error,
                                            }),
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }

    units
}

fn extract_unit(node: Node<'_>, source: &[u8], kind: UnitKind) -> Result<CodeUnit, String> {
    let name_node = node
        .child_by_field_name("name")
        .ok_or_else(|| "missing name node".to_string())?;
    let name = name_node
        .utf8_text(source)
        .map_err(|_| "name node contains invalid UTF-8".to_string())?
        .to_string();
    let params_node = node.child_by_field_name("parameters");
    let body_node = node
        .child_by_field_name("body")
        .ok_or_else(|| "missing body node".to_string())?;

    let span = node_span(node);
    let parameters = params_node.map_or_else(Vec::new, |p| extract_params(p, source));
    let is_exported = !name.starts_with('_');
    let body = build_block(body_node, source, 0);

    Ok(CodeUnit {
        name,
        kind,
        span,
        parameters,
        body,
        is_exported,
    })
}

fn extract_params(params_node: Node<'_>, source: &[u8]) -> Vec<Parameter> {
    let mut params = Vec::new();
    let mut cursor = params_node.walk();
    for child in params_node.children(&mut cursor) {
        let param_name = match child.kind() {
            "identifier" => child.utf8_text(source).ok().map(str::to_string),
            "typed_parameter" | "default_parameter" | "typed_default_parameter" => child
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

fn build_block(node: Node<'_>, source: &[u8], nesting: u32) -> Block {
    Block {
        span: node_span(node),
        nesting,
        children: collect_block_children(node, source, nesting),
    }
}

fn collect_block_children(node: Node<'_>, source: &[u8], nesting: u32) -> Vec<crate::model::Node> {
    let mut children = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        let kind = child.kind();
        match kind {
            // Never descend into nested function or class definitions
            "function_definition" | "class_definition" => continue,

            "if_statement"
            | "for_statement"
            | "while_statement"
            | "try_statement"
            | "boolean_operator"
            | "conditional_expression" => {
                dispatch_branch(child, source, nesting, &mut children);
            }

            _ => {
                // Recurse into all other nodes to find branches deeper in expressions/statements
                children.extend(collect_block_children(child, source, nesting));
            }
        }
    }

    children
}

fn dispatch_branch(node: Node<'_>, source: &[u8], nesting: u32, out: &mut Vec<crate::model::Node>) {
    let kind = node.kind();
    match kind {
        "if_statement" => handle_if(node, source, nesting, out),
        "for_statement" | "while_statement" => handle_loop(node, source, nesting, out),
        "try_statement" => handle_try(node, source, nesting, out),
        "boolean_operator" => handle_logical(node, source, nesting, out),
        "conditional_expression" => handle_ternary(node, source, nesting, out),
        _ => {
            // Log or handle unknown branch types if needed
        }
    }
}

fn handle_if(node: Node<'_>, source: &[u8], nesting: u32, out: &mut Vec<crate::model::Node>) {
    out.push(crate::model::Node::Branch(Branch {
        kind: BranchKind::If,
        span: node_span(node),
        nesting_at: nesting,
        sequence_id: None,
    }));
    // consequence block becomes a NestedBlock
    if let Some(body) = node.child_by_field_name("consequence") {
        out.push(crate::model::Node::NestedBlock(build_block(
            body,
            source,
            nesting + 1,
        )));
    }
    // elif/else alternatives
    if let Some(alt) = node.child_by_field_name("alternative") {
        collect_alternatives(alt, source, nesting, out);
    }
}

fn handle_loop(node: Node<'_>, source: &[u8], nesting: u32, out: &mut Vec<crate::model::Node>) {
    out.push(crate::model::Node::Branch(Branch {
        kind: BranchKind::Loop,
        span: node_span(node),
        nesting_at: nesting,
        sequence_id: None,
    }));
    if let Some(body) = node.child_by_field_name("body") {
        out.push(crate::model::Node::NestedBlock(build_block(
            body,
            source,
            nesting + 1,
        )));
    }
}

fn handle_try(node: Node<'_>, source: &[u8], nesting: u32, out: &mut Vec<crate::model::Node>) {
    let mut tc = node.walk();
    for clause in node.children(&mut tc) {
        if clause.kind() == "except_clause" || clause.kind() == "except_group_clause" {
            out.push(crate::model::Node::Branch(Branch {
                kind: BranchKind::Catch,
                span: node_span(clause),
                nesting_at: nesting,
                sequence_id: None,
            }));
            // Recurse into except body (last child of except_clause that is a block)
            let mut ec = clause.walk();
            for except_child in clause.children(&mut ec) {
                if except_child.kind() == "block" {
                    out.push(crate::model::Node::NestedBlock(build_block(
                        except_child,
                        source,
                        nesting + 1,
                    )));
                }
            }
        }
    }
    // Also process the try body
    let mut tc2 = node.walk();
    for clause in node.children(&mut tc2) {
        if clause.kind() == "block" {
            out.extend(collect_block_children(clause, source, nesting));
            break;
        }
    }
}

fn handle_logical(node: Node<'_>, source: &[u8], nesting: u32, out: &mut Vec<crate::model::Node>) {
    let operator = node.utf8_text(source).ok().and_then(|text| {
        text.split_whitespace()
            .find(|part| *part == "and" || *part == "or")
    });
    let kind = match operator {
        Some("or") => BranchKind::LogicalOr,
        _ => BranchKind::LogicalAnd,
    };

    out.push(crate::model::Node::Branch(Branch {
        kind,
        span: node_span(node),
        nesting_at: nesting,
        sequence_id: None,
    }));
    // Recurse for nested branches within the expression
    out.extend(collect_block_children(node, source, nesting));
}

fn handle_ternary(node: Node<'_>, source: &[u8], nesting: u32, out: &mut Vec<crate::model::Node>) {
    out.push(crate::model::Node::Branch(Branch {
        kind: BranchKind::Ternary,
        span: node_span(node),
        nesting_at: nesting,
        sequence_id: None,
    }));
    out.extend(collect_block_children(node, source, nesting));
}

fn collect_alternatives(
    node: Node<'_>,
    source: &[u8],
    nesting: u32,
    out: &mut Vec<crate::model::Node>,
) {
    match node.kind() {
        "elif_clause" => {
            out.push(crate::model::Node::Branch(Branch {
                kind: BranchKind::ElseIf,
                span: node_span(node),
                nesting_at: nesting,
                sequence_id: None,
            }));
            if let Some(body) = node.child_by_field_name("consequence") {
                out.push(crate::model::Node::NestedBlock(build_block(
                    body,
                    source,
                    nesting + 1,
                )));
            }
            if let Some(alt) = node.child_by_field_name("alternative") {
                collect_alternatives(alt, source, nesting, out);
            }
        }
        "else_clause" => {
            if let Some(body) = node.child_by_field_name("body") {
                out.push(crate::model::Node::NestedBlock(build_block(
                    body,
                    source,
                    nesting + 1,
                )));
            }
        }
        _ => {}
    }
}

fn method_kind(node: Node<'_>, source: &[u8]) -> UnitKind {
    if let Some(name_node) = node.child_by_field_name("name") {
        if let Ok(name) = name_node.utf8_text(source) {
            if name == "__init__" || name == "__new__" {
                return UnitKind::Ctor;
            }
        }
    }
    UnitKind::Method
}

fn node_span(node: Node<'_>) -> Span {
    Span {
        start_line: node.start_position().row as u32 + 1,
        end_line: node.end_position().row as u32 + 1,
    }
}
