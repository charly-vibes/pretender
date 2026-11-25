//! Inheritance depth analysis.
//!
//! Detects class inheritance chains and flags files exceeding the
//! configured maximum depth. Single-file analysis only — cross-file
//! parent classes are depth = 1 with a note.

use std::collections::{HashMap, HashSet};

use crate::model::Language;

/// Result of inheritance depth analysis for one file.
#[derive(Debug, Clone, Default)]
pub struct InheritanceAnalysis {
    pub classes: Vec<ClassDepth>,
}

/// A class and its computed inheritance depth.
#[derive(Debug, Clone)]
pub struct ClassDepth {
    #[allow(dead_code)] // informative metadata for future output formats
    pub name: String,
    pub depth: u32,
    #[allow(dead_code)] // informative metadata for future output formats
    pub parent: Option<String>,
    #[allow(dead_code)] // informative metadata for future output formats
    pub cross_file: bool,
}

/// Compute inheritance depth for all classes in a source file.
///
/// Walks the source for class declarations and their parent classes.
/// Only resolves chains within the same file — cross-file parents
/// are depth = 1 with `cross_file = true`.
pub fn analyze(source: &str, language: &Language) -> InheritanceAnalysis {
    let declarations = dispatch_extract(source, language);
    let class_map: HashMap<String, ClassDecl> = declarations
        .iter()
        .map(|decl| (decl.name.clone(), decl.clone()))
        .collect();
    let classes = declarations
        .iter()
        .map(|decl| class_depth(decl, &class_map))
        .collect();
    InheritanceAnalysis { classes }
}

/// Depth resolution for one declaration against the shared class map.
fn class_depth(decl: &ClassDecl, class_map: &HashMap<String, ClassDecl>) -> ClassDepth {
    let (depth, cross_file) = compute_depth(&decl.name, class_map, &mut HashSet::new(), 0);
    ClassDepth {
        name: decl.name.clone(),
        depth,
        parent: decl.parent.clone(),
        cross_file,
    }
}

#[derive(Debug, Clone)]
struct ClassDecl {
    name: String,
    parent: Option<String>,
}

/// Walk the parent chain for one class.
///
/// A parent missing from `class_map` past the root is a cross-file
/// reference; a cycle stops the walk at the depth reached so far.
fn compute_depth(
    name: &str,
    class_map: &HashMap<String, ClassDecl>,
    visited: &mut HashSet<String>,
    depth: u32,
) -> (u32, bool) {
    let Some(decl) = class_map.get(name) else {
        return cross_file_depth(depth);
    };
    let Some(parent) = decl.parent.as_deref() else {
        return (depth, false);
    };
    // Cycle detection
    if visited.contains(parent) {
        return (depth + 1, false);
    }
    visited.insert(parent.to_string());
    compute_depth(parent, class_map, visited, depth + 1)
}

/// A class absent from the map is cross-file only past the root.
fn cross_file_depth(depth: u32) -> (u32, bool) {
    if depth > 0 {
        (depth, true)
    } else {
        (0, false)
    }
}

/// Per-language line scanners. C# shares the Java scanner; languages
/// without an entry yield no declarations.
type Extractor = fn(&str) -> Vec<ClassDecl>;

const EXTRACTORS: &[(Language, Extractor)] = &[
    (Language::Java, extract_java_classes),
    (Language::CSharp, extract_java_classes),
    (Language::Python, extract_python_classes),
    (Language::JavaScript, extract_ts_classes),
    (Language::TypeScript, extract_ts_classes),
    (Language::Cpp, extract_cpp_classes),
    (Language::Ruby, extract_ruby_classes),
    (Language::Rust, extract_rust_traits),
];

fn dispatch_extract(source: &str, language: &Language) -> Vec<ClassDecl> {
    EXTRACTORS
        .iter()
        .find(|(lang, _)| lang == language)
        .map_or_else(Vec::new, |(_, extract)| extract(source))
}

fn extract_java_classes(source: &str) -> Vec<ClassDecl> {
    source.lines().filter_map(java_class_from_line).collect()
}

/// Match: `class Foo` or `class Foo extends Bar`.
fn java_class_from_line(line: &str) -> Option<ClassDecl> {
    let rest = line.trim().strip_prefix("class ")?;
    let name = rest.split_whitespace().next()?.to_string();
    if name.starts_with('{') {
        return None;
    }
    let parent = rest.find(" extends ").and_then(|extends_pos| {
        rest[extends_pos + 9..]
            .split_whitespace()
            .next()
            .map(|s| s.trim_end_matches('{').trim().to_string())
    });
    Some(ClassDecl { name, parent })
}

fn extract_python_classes(source: &str) -> Vec<ClassDecl> {
    source.lines().filter_map(python_class_from_line).collect()
}

/// Dispatch: `class Foo(Bar):` vs `class Foo:`.
fn python_class_from_line(line: &str) -> Option<ClassDecl> {
    let rest = line.trim().strip_prefix("class ")?;
    if let Some(paren) = rest.find('(') {
        python_inherited_class(rest, paren)
    } else {
        python_standalone_class(rest)
    }
}

/// `class Foo(Bar):` — first parent only (multiple inheritance ignored).
fn python_inherited_class(rest: &str, paren: usize) -> Option<ClassDecl> {
    let name = rest[..paren].trim().to_string();
    let parent_part = rest[paren + 1..].trim_end_matches(':').trim();
    let parent = parent_part.split(',').next().map(|s| s.trim().to_string());
    if !name.is_empty() && parent.as_deref() != Some("object") {
        return Some(ClassDecl { name, parent });
    }
    None
}

/// `class Foo:` — declared without a parent.
fn python_standalone_class(rest: &str) -> Option<ClassDecl> {
    let name = rest[..rest.find(':')?].trim().to_string();
    if name.is_empty() {
        return None;
    }
    Some(ClassDecl { name, parent: None })
}

fn extract_ts_classes(source: &str) -> Vec<ClassDecl> {
    source.lines().filter_map(ts_class_from_line).collect()
}

/// Match: `class Foo extends Bar {`.
fn ts_class_from_line(line: &str) -> Option<ClassDecl> {
    let rest = line.trim().strip_prefix("class ")?;
    let extends_pos = rest.find(" extends ")?;
    let name = rest[..extends_pos].split_whitespace().next()?.to_string();
    let parent = rest[extends_pos + 9..].split_whitespace().next().map(|s| {
        s.trim_matches(|c: char| c.is_whitespace() || c == '{' || c == ',')
            .to_string()
    });
    Some(ClassDecl { name, parent })
}

fn extract_cpp_classes(source: &str) -> Vec<ClassDecl> {
    source.lines().filter_map(cpp_class_from_line).collect()
}

/// Match: `class Foo : public Bar {`.
fn cpp_class_from_line(line: &str) -> Option<ClassDecl> {
    let rest = line.trim().strip_prefix("class ")?;
    let name = rest.split_whitespace().next()?.to_string();
    if name.starts_with(';') {
        return None;
    }
    let colon_pos = rest.find(" : ")?;
    Some(ClassDecl {
        name,
        parent: cpp_parent(rest[colon_pos + 3..].trim()),
    })
}

/// First parent after `:`, with visibility keyword stripped.
fn cpp_parent(after_colon: &str) -> Option<String> {
    let visibility = ["public ", "private ", "protected "];
    visibility
        .iter()
        .find_map(|v| after_colon.strip_prefix(v))
        .or(Some(after_colon))
        .and_then(|s| {
            s.split_whitespace()
                .next()
                .map(|s| s.trim_matches(|c: char| c == '{' || c == ',').to_string())
        })
}

fn extract_ruby_classes(source: &str) -> Vec<ClassDecl> {
    source.lines().filter_map(ruby_class_from_line).collect()
}

/// Match: `class Foo < Bar`.
fn ruby_class_from_line(line: &str) -> Option<ClassDecl> {
    let rest = line.trim().strip_prefix("class ")?;
    let lt_pos = rest.find(" < ")?;
    let name = rest[..lt_pos].trim().to_string();
    let parent = rest[lt_pos + 3..]
        .split_whitespace()
        .next()
        .map(|s| s.to_string());
    if name.is_empty() {
        return None;
    }
    Some(ClassDecl { name, parent })
}

fn extract_rust_traits(source: &str) -> Vec<ClassDecl> {
    source.lines().filter_map(rust_trait_from_line).collect()
}

/// Match: `trait Foo: Bar + Baz {` — first non-stdlib bound is the parent.
fn rust_trait_from_line(line: &str) -> Option<ClassDecl> {
    let rest = line.trim().strip_prefix("trait ")?;
    let parts: Vec<&str> = rest.trim().splitn(2, ": ").collect();
    let name = parts[0].split_whitespace().next()?.to_string();
    let parent = if parts.len() > 1 {
        first_custom_bound(parts[1])
    } else {
        // No bounds — standalone trait
        None
    };
    Some(ClassDecl { name, parent })
}

/// First bound in `Bar + Baz` that is not a known stdlib trait.
fn first_custom_bound(bounds: &str) -> Option<String> {
    let bounds = bounds.trim_end_matches(['{', '}']).trim();
    bounds
        .split('+')
        .map(|s| s.trim())
        .find(|b| !is_stdlib_bound(b))
        .map(|s| s.to_string())
}

/// Bounds that never count as inheritance parents.
fn is_stdlib_bound(bound: &str) -> bool {
    matches!(
        bound,
        "Display"
            | "Debug"
            | "Clone"
            | "Copy"
            | "Send"
            | "Sync"
            | "Sized"
            | "Default"
            | "Eq"
            | "PartialEq"
            | "Ord"
            | "PartialOrd"
            | "Hash"
            | "Into"
            | "From"
    )
}
