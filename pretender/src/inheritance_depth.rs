//! Inheritance depth analysis.
//!
//! Detects class inheritance chains and flags files exceeding the
//! configured maximum depth. Single-file analysis only — cross-file
//! parent classes are depth = 1 with a note.

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
    let declarations = match language {
        Language::Java => extract_java_classes(source),
        Language::Python => extract_python_classes(source),
        Language::JavaScript | Language::TypeScript => extract_ts_classes(source),
        Language::Cpp => extract_cpp_classes(source),
        Language::CSharp => extract_csharp_classes(source),
        Language::Ruby => extract_ruby_classes(source),
        Language::Rust => extract_rust_traits(source),
        _ => Vec::new(),
    };

    // Build a lookup map for quick parent resolution
    let mut class_map: std::collections::HashMap<String, ClassDecl> =
        std::collections::HashMap::new();
    for decl in &declarations {
        class_map.insert(decl.name.clone(), decl.clone());
    }

    // Compute depth for each class by walking the parent chain
    let mut classes = Vec::new();
    for decl in &declarations {
        let (depth, cross_file) = compute_depth(
            &decl.name,
            &class_map,
            &mut std::collections::HashSet::new(),
            0,
        );
        classes.push(ClassDepth {
            name: decl.name.clone(),
            depth,
            parent: decl.parent.clone(),
            cross_file,
        });
    }

    InheritanceAnalysis { classes }
}

#[derive(Debug, Clone)]
struct ClassDecl {
    name: String,
    parent: Option<String>,
}

fn compute_depth(
    name: &str,
    class_map: &std::collections::HashMap<String, ClassDecl>,
    visited: &mut std::collections::HashSet<String>,
    depth: u32,
) -> (u32, bool) {
    if let Some(decl) = class_map.get(name) {
        if let Some(ref parent) = decl.parent {
            // Cycle detection
            if visited.contains(parent) {
                return (depth + 1, false);
            }
            visited.insert(parent.clone());
            let (parent_depth, cross_file) = compute_depth(parent, class_map, visited, depth + 1);
            (parent_depth, cross_file)
        } else {
            (depth, false)
        }
    } else if depth > 0 {
        // Parent class not in this file — cross-file reference
        (depth, true)
    } else {
        (0, false)
    }
}

fn extract_java_classes(source: &str) -> Vec<ClassDecl> {
    let mut classes = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        // Match: class Foo or class Foo extends Bar
        if let Some(rest) = trimmed.strip_prefix("class ") {
            let name = rest.split_whitespace().next().unwrap_or("").to_string();
            if name.is_empty() || name.starts_with('{') {
                continue;
            }
            let parent = if let Some(extends_pos) = rest.find(" extends ") {
                rest[extends_pos + 9..]
                    .split_whitespace()
                    .next()
                    .map(|s| s.trim_end_matches('{').trim().to_string())
            } else {
                None
            };
            classes.push(ClassDecl { name, parent });
        }
    }
    classes
}

fn extract_python_classes(source: &str) -> Vec<ClassDecl> {
    let mut classes = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("class ") {
            // class Foo(Bar):
            if let Some(paren) = rest.find('(') {
                let name = rest[..paren].trim().to_string();
                let parent_part = rest[paren + 1..].trim_end_matches(':').trim();
                // Get the first parent (ignore multiple inheritance for simplicity)
                let parent = parent_part.split(',').next().map(|s| s.trim().to_string());
                if !name.is_empty() && parent.as_deref() != Some("object") {
                    classes.push(ClassDecl { name, parent });
                }
            }
            // class Foo: (no parent)
            else if let Some(end) = rest.find(':') {
                let name = rest[..end].trim().to_string();
                if !name.is_empty() {
                    classes.push(ClassDecl { name, parent: None });
                }
            }
        }
    }
    classes
}

fn extract_ts_classes(source: &str) -> Vec<ClassDecl> {
    let mut classes = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        // class Foo extends Bar
        if let Some(rest) = trimmed.strip_prefix("class ") {
            if let Some(extends_pos) = rest.find(" extends ") {
                let name = rest[..extends_pos]
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .to_string();
                let parent = rest[extends_pos + 9..].split_whitespace().next().map(|s| {
                    s.trim_matches(|c: char| c.is_whitespace() || c == '{' || c == ',')
                        .to_string()
                });
                if !name.is_empty() {
                    classes.push(ClassDecl { name, parent });
                }
            }
        }
    }
    classes
}

fn extract_cpp_classes(source: &str) -> Vec<ClassDecl> {
    let mut classes = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        // class Foo : public Bar
        if let Some(rest) = trimmed.strip_prefix("class ") {
            let name = rest.split_whitespace().next().unwrap_or("").to_string();
            if !name.is_empty() && !name.starts_with(';') {
                // Check for inheritance
                if let Some(colon_pos) = rest.find(" : ") {
                    let visibility = ["public ", "private ", "protected "];
                    let after_colon = rest[colon_pos + 3..].trim();
                    let parent = visibility
                        .iter()
                        .find_map(|v| after_colon.strip_prefix(v))
                        .or(Some(after_colon))
                        .and_then(|s| {
                            s.split_whitespace()
                                .next()
                                .map(|s| s.trim_matches(|c: char| c == '{' || c == ',').to_string())
                        });
                    classes.push(ClassDecl { name, parent });
                }
            }
        }
    }
    classes
}

fn extract_csharp_classes(source: &str) -> Vec<ClassDecl> {
    // Same pattern as Java for C#
    extract_java_classes(source)
}

fn extract_ruby_classes(source: &str) -> Vec<ClassDecl> {
    let mut classes = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        // class Foo < Bar
        if let Some(rest) = trimmed.strip_prefix("class ") {
            if let Some(lt_pos) = rest.find(" < ") {
                let name = rest[..lt_pos].trim().to_string();
                let parent = rest[lt_pos + 3..]
                    .split_whitespace()
                    .next()
                    .map(|s| s.to_string());
                if !name.is_empty() {
                    classes.push(ClassDecl { name, parent });
                }
            }
        }
    }
    classes
}

fn extract_rust_traits(source: &str) -> Vec<ClassDecl> {
    let mut classes = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        // trait Foo: Bar + Baz
        if let Some(rest) = trimmed.strip_prefix("trait ") {
            // Split by ':' to get name and bounds separately
            let parts: Vec<&str> = rest.trim().splitn(2, ": ").collect();
            let name = parts[0].split_whitespace().next().unwrap_or("").to_string();
            if name.is_empty() {
                continue;
            }
            if parts.len() > 1 {
                let bounds = parts[1].trim_end_matches(['{', '}']).trim();
                // Find the first non-stdlib bound
                let parent = bounds
                    .split('+')
                    .map(|s| s.trim())
                    .find(|b| {
                        !matches!(
                            *b,
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
                    })
                    .map(|s| s.to_string());
                classes.push(ClassDecl { name, parent });
            } else {
                // No bounds — standalone trait
                classes.push(ClassDecl { name, parent: None });
            }
        }
    }
    classes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn java_single_inheritance() {
        let source = r#"
class A {}
class B extends A {}
class C extends B {}
"#;
        let analysis = analyze(source, &Language::Java);
        let c = analysis.classes.iter().find(|c| c.name == "C").unwrap();
        assert_eq!(c.depth, 2);
    }

    #[test]
    fn java_exceeds_threshold() {
        let source = r#"
class A {}
class B extends A {}
class C extends B {}
class D extends C {}
"#;
        let analysis = analyze(source, &Language::Java);
        let d = analysis.classes.iter().find(|c| c.name == "D").unwrap();
        assert_eq!(d.depth, 3);
    }

    #[test]
    fn no_parent_depth_zero() {
        let source = "class A {}";
        let analysis = analyze(source, &Language::Java);
        let a = analysis.classes.iter().find(|c| c.name == "A").unwrap();
        assert_eq!(a.depth, 0);
    }

    #[test]
    fn python_class_inheritance() {
        let source = r#"
class A:
    pass
class B(A):
    pass
"#;
        let analysis = analyze(source, &Language::Python);
        let b = analysis.classes.iter().find(|c| c.name == "B").unwrap();
        assert_eq!(b.depth, 1);
    }

    #[test]
    fn rust_stdlib_trait_excluded() {
        let source = r#"
trait Foo: Display + Debug {}
"#;
        let analysis = analyze(source, &Language::Rust);
        let f = analysis.classes.iter().find(|c| c.name == "Foo").unwrap();
        assert_eq!(f.depth, 0);
    }

    #[test]
    fn rust_custom_trait() {
        let source = r#"
trait Bar {}
trait Foo: Bar {}
"#;
        let analysis = analyze(source, &Language::Rust);
        let f = analysis.classes.iter().find(|c| c.name == "Foo").unwrap();
        assert_eq!(f.depth, 1);
    }

    #[test]
    fn cross_file_parent_depth_one() {
        let source = "class Foo extends Bar {}";
        let analysis = analyze(source, &Language::Java);
        let f = analysis.classes.iter().find(|c| c.name == "Foo").unwrap();
        assert_eq!(f.depth, 1);
        assert!(f.cross_file);
    }

    #[test]
    fn unsupported_language_empty() {
        let analysis = analyze("nothing", &Language::Julia);
        assert!(analysis.classes.is_empty());
    }
}
