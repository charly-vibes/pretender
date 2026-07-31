//! Error-handling quality analysis: unwrap/except/catch density.
//!
//! Detects risky error-handling patterns per language:
//! - Rust: unwrap(), expect()
//! - Python: bare except:, except Exception:
//! - JS/TS: empty catch {}
//! - Java: catch (Exception e) {}
//! - Go: ignored errors (_ = fn()), bare panic/recover

use crate::model::Language;

/// A single risky error-handling site.
#[derive(Debug, Clone)]
pub struct UnwrapSite {
    #[allow(dead_code)] // informative metadata for future output formats
    pub line: u32,
    #[allow(dead_code)] // informative metadata for future output formats
    pub kind: UnwrapKind,
}

/// Classification of the error-handling pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnwrapKind {
    Unwrap,
    Expect,
    BareExcept,
    EmptyCatch,
    IgnoredError,
    BarePanic,
}

/// Count risky error-handling patterns in source code.
pub fn detect_unwraps(source: &str, language: &Language) -> Vec<UnwrapSite> {
    match language {
        Language::Rust => detect_rust_unwraps(source),
        Language::Python => detect_python_unwraps(source),
        Language::JavaScript | Language::TypeScript => detect_js_unwraps(source),
        Language::Java => detect_java_unwraps(source),
        Language::Go => detect_go_unwraps(source),
        _ => Vec::new(),
    }
}

fn detect_rust_unwraps(source: &str) -> Vec<UnwrapSite> {
    let mut sites = Vec::new();
    for (i, line) in source.lines().enumerate() {
        let line_num = (i + 1) as u32;
        let trimmed = line.trim();
        // Skip comments
        if trimmed.starts_with("//") || trimmed.starts_with("/*") {
            continue;
        }
        // unwrap() calls
        if trimmed.contains(".unwrap(") || trimmed.contains(".unwrap()") {
            // Skip test assertions that use .unwrap() idiomatically
            if !trimmed.starts_with("#[") {
                sites.push(UnwrapSite {
                    line: line_num,
                    kind: UnwrapKind::Unwrap,
                });
            }
        }
        // expect() calls
        if trimmed.contains(".expect(") || trimmed.contains(".expect()") {
            sites.push(UnwrapSite {
                line: line_num,
                kind: UnwrapKind::Expect,
            });
        }
    }
    sites
}

fn detect_python_unwraps(source: &str) -> Vec<UnwrapSite> {
    let mut sites = Vec::new();
    let lines: Vec<&str> = source.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let line_num = (i + 1) as u32;
        let trimmed = lines[i].trim();
        // Bare except: (except: with no type specified)
        if trimmed == "except:" || trimmed.starts_with("except :") {
            sites.push(UnwrapSite {
                line: line_num,
                kind: UnwrapKind::BareExcept,
            });
        }
        // except Exception: (catching too broadly)
        if trimmed.starts_with("except Exception") && !trimmed.contains("as e") && !trimmed.contains(" as ") {
            sites.push(UnwrapSite {
                line: line_num,
                kind: UnwrapKind::BareExcept,
            });
        }
        i += 1;
    }
    sites
}

fn detect_js_unwraps(source: &str) -> Vec<UnwrapSite> {
    let mut sites = Vec::new();
    let lines: Vec<&str> = source.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let line_num = (i + 1) as u32;
        let trimmed = lines[i].trim();
        // Empty catch block: catch {} or catch(e) {} on same line
        if trimmed.contains("catch") && (trimmed.ends_with("{}") || trimmed.ends_with("{ }")) {
            sites.push(UnwrapSite {
                line: line_num,
                kind: UnwrapKind::EmptyCatch,
            });
        }
        // catch on one line, empty block on next
        if trimmed.contains("catch") && i + 1 < lines.len() {
            let next = lines[i + 1].trim();
            if next == "{}" || next == "{ }" {
                sites.push(UnwrapSite {
                    line: line_num,
                    kind: UnwrapKind::EmptyCatch,
                });
            }
        }
        i += 1;
    }
    sites
}

fn detect_java_unwraps(source: &str) -> Vec<UnwrapSite> {
    let mut sites = Vec::new();
    for (i, line) in source.lines().enumerate() {
        let line_num = (i + 1) as u32;
        let trimmed = line.trim();
        // catch (Exception e) {} — overly broad catch
        if trimmed.contains("catch (Exception") && (trimmed.ends_with("{}") || trimmed.ends_with("{ }")) {
            sites.push(UnwrapSite {
                line: line_num,
                kind: UnwrapKind::EmptyCatch,
            });
        }
    }
    sites
}

fn detect_go_unwraps(source: &str) -> Vec<UnwrapSite> {
    let mut sites = Vec::new();
    for (i, line) in source.lines().enumerate() {
        let line_num = (i + 1) as u32;
        let trimmed = line.trim();
        // Ignored error: _ = fn()
        if trimmed.starts_with("_ = ") || trimmed.starts_with("_, ") {
            sites.push(UnwrapSite {
                line: line_num,
                kind: UnwrapKind::IgnoredError,
            });
        }
        // Bare panic or recover outside of error check
        // Note: `if err != nil { return err }` is idiomatic Go — NOT flagged
        if trimmed == "panic(" || trimmed.starts_with("panic(") && !trimmed.starts_with("panic(err") {
            sites.push(UnwrapSite {
                line: line_num,
                kind: UnwrapKind::BarePanic,
            });
        }
    }
    sites
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_rust_unwrap() {
        let source = r#"
fn parse(input: &str) -> i32 {
    input.parse().unwrap()
}
"#;
        let sites = detect_unwraps(source, &Language::Rust);
        assert_eq!(sites.len(), 1);
        assert_eq!(sites[0].kind, UnwrapKind::Unwrap);
    }

    #[test]
    fn detects_rust_expect() {
        let source = r#"let x = value.expect("should be valid");"#;
        let sites = detect_unwraps(source, &Language::Rust);
        assert_eq!(sites.len(), 1);
        assert_eq!(sites[0].kind, UnwrapKind::Expect);
    }

    #[test]
    fn detects_python_bare_except() {
        let source = r#"
try:
    do_something()
except:
    pass
"#;
        let sites = detect_unwraps(source, &Language::Python);
        assert_eq!(sites.len(), 1);
        assert_eq!(sites[0].kind, UnwrapKind::BareExcept);
    }

    #[test]
    fn go_idiomatic_propagation_not_flagged() {
        let source = r#"
if err != nil {
    return err
}
"#;
        let sites = detect_unwraps(source, &Language::Go);
        assert_eq!(sites.len(), 0);
    }

    #[test]
    fn go_ignored_error_flagged() {
        let source = r#"_ = doSomething()"#;
        let sites = detect_unwraps(source, &Language::Go);
        assert_eq!(sites.len(), 1);
        assert_eq!(sites[0].kind, UnwrapKind::IgnoredError);
    }

    #[test]
    fn js_empty_catch_flagged() {
        let source = r#"
try {
    doSomething();
} catch(e) {}
"#;
        let sites = detect_unwraps(source, &Language::JavaScript);
        assert_eq!(sites.len(), 1);
        assert_eq!(sites[0].kind, UnwrapKind::EmptyCatch);
    }

    #[test]
    fn unsupported_language_empty() {
        let sites = detect_unwraps("nothing", &Language::Julia);
        assert_eq!(sites.len(), 0);
    }

    #[test]
    fn no_false_positive_on_clean_code() {
        let source = r#"
fn safe_parse(input: &str) -> Result<i32, String> {
    input.parse().map_err(|e| e.to_string())
}
"#;
        let sites = detect_unwraps(source, &Language::Rust);
        assert_eq!(sites.len(), 0);
    }
}