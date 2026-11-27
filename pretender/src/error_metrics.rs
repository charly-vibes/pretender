//! Error-handling quality analysis: unwrap/except/catch density.
//!
//! Detects risky error-handling patterns per language:
//! - Rust: unwrap(), expect()
//! - Python: bare except:, except Exception:
//! - JS/TS: empty catch {}
//! - Java: catch (Exception e) {}
//! - Go: ignored errors (_ = fn()), bare panic/recover

use crate::model::Language;
use crate::roles::Role;

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
    detect_unwraps_with_role(source, language, Role::App)
}

/// Role-aware variant: for test-role files, `.expect(...)` sites inside
/// `#[cfg(test)]`-gated modules are excluded from the count
/// (pretender-jg9y) — the file is already test code, and the gated module
/// double-counts harness plumbing against the test unwrap budget.
pub fn detect_unwraps_with_role(source: &str, language: &Language, role: Role) -> Vec<UnwrapSite> {
    match language {
        Language::Rust => detect_rust_unwraps(source, role == Role::Test),
        Language::Python => detect_python_unwraps(source),
        Language::JavaScript | Language::TypeScript => detect_js_unwraps(source),
        Language::Java => detect_java_unwraps(source),
        Language::Go => detect_go_unwraps(source),
        _ => Vec::new(),
    }
}

fn detect_rust_unwraps(source: &str, role_test: bool) -> Vec<UnwrapSite> {
    let lines: Vec<&str> = source.lines().collect();
    let cfg_test = if role_test {
        cfg_test_module_lines(&lines)
    } else {
        vec![false; lines.len()]
    };
    let mut sites = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let line_num = (i + 1) as u32;
        if let Some(kind) = rust_line_unwrap(line.trim(), cfg_test[i]) {
            sites.push(UnwrapSite {
                line: line_num,
                kind,
            });
        }
    }
    sites
}

/// Classify one Rust line: comments are skipped, `.unwrap()` is skipped for
/// attribute lines, `.expect()` always flags. The `unwrap_or` family
/// (`.unwrap_or`, `.unwrap_or_else`, ...) never flags: it recovers from
/// None/Err instead of panicking, and its name never matches the
/// `.unwrap(` pattern. Expects inside a `#[cfg(test)]`-gated module body
/// are skipped when the caller says the file's role is test.
fn rust_line_unwrap(trimmed: &str, in_cfg_test_body: bool) -> Option<UnwrapKind> {
    if trimmed.starts_with("//") || trimmed.starts_with("/*") {
        return None;
    }
    // unwrap() calls, skipping #[...] attribute lines (`.unwrap(` covers `.unwrap()`)
    if trimmed.contains(".unwrap(") && !trimmed.starts_with("#[") {
        return Some(UnwrapKind::Unwrap);
    }
    (trimmed.contains(".expect(") && !in_cfg_test_body).then_some(UnwrapKind::Expect)
}

/// One scan state over the source lines, tracking whether the line sits
/// inside a `#[cfg(test)]`-gated module body.
#[derive(Default)]
struct CfgTestScan {
    armed: bool,
    inside: bool,
    depth: i32,
    opened: bool,
}

/// Per-line marks for lines inside a `#[cfg(test)]`-gated module body.
/// Line-scan heuristic: a `#[cfg(test)]` attribute arms the tracker, the
/// module line opens the region, and the region closes when its brace
/// depth returns to zero. Comment-line brace noise is tolerated (the
/// unwrap scanner itself is line-based).
fn cfg_test_module_lines(lines: &[&str]) -> Vec<bool> {
    lines
        .iter()
        .map(|line| line.trim())
        .scan(CfgTestScan::default(), |state, trimmed| {
            step_cfg_test_scan(state, trimmed);
            Some(state.inside && state.opened)
        })
        .collect()
}

/// One scan step: react to attribute/module lines outside a region, then
/// track brace depth inside one.
fn step_cfg_test_scan(state: &mut CfgTestScan, trimmed: &str) {
    if !state.inside {
        open_cfg_test_region(state, trimmed);
    }
    advance_cfg_test_body(state, trimmed);
}

/// React to `#[cfg(test)]` and module lines while outside a region: the
/// attribute alone arms the tracker, a module line on the attribute or
/// the next `mod` line opens the region.
fn open_cfg_test_region(state: &mut CfgTestScan, trimmed: &str) {
    if let Some(body) = trimmed.strip_prefix("#[cfg(test)]") {
        let body = body.trim_start();
        if body.starts_with("mod") && !body.trim_end().ends_with(';') {
            state.inside = true;
            state.depth = 0;
            state.opened = false;
        } else if body.is_empty() || body.starts_with("#[") {
            state.armed = true;
        }
    } else if state.armed && trimmed.starts_with("mod") && !trimmed.trim_end().ends_with(';') {
        state.armed = false;
        state.inside = true;
        state.depth = 0;
        state.opened = false;
    }
}

/// Track brace depth inside the region; it closes when the opened depth
/// returns to zero.
fn advance_cfg_test_body(state: &mut CfgTestScan, trimmed: &str) {
    if !state.inside {
        return;
    }
    state.depth += brace_delta(trimmed);
    state.opened = state.opened || state.depth > 0;
    if state.opened && state.depth <= 0 {
        state.inside = false;
        state.opened = false;
    }
}

/// Net brace delta of one line.
fn brace_delta(line: &str) -> i32 {
    line.chars().fold(0, |delta, c| match c {
        '{' => delta + 1,
        '}' => delta - 1,
        _ => delta,
    })
}

/// One located unwrap site (avoids struct-literal width churn at call sites).
fn site(line_num: u32, kind: UnwrapKind) -> UnwrapSite {
    UnwrapSite {
        line: line_num,
        kind,
    }
}

fn detect_python_unwraps(source: &str) -> Vec<UnwrapSite> {
    source
        .lines()
        .enumerate()
        .flat_map(|(i, line)| python_line_unwrap((i + 1) as u32, line.trim()))
        .collect()
}

/// Classify one Python line: bare `except:` and broad `except Exception:`.
fn python_line_unwrap(line_num: u32, trimmed: &str) -> Vec<UnwrapSite> {
    let bare = trimmed == "except:" || trimmed.starts_with("except :");
    // except Exception: (catching too broadly)
    let broad = trimmed.starts_with("except Exception")
        && !trimmed.contains("as e")
        && !trimmed.contains(" as ");
    [bare, broad]
        .iter()
        .filter(|hits| **hits)
        .map(|_| site(line_num, UnwrapKind::BareExcept))
        .collect()
}

fn detect_js_unwraps(source: &str) -> Vec<UnwrapSite> {
    let lines: Vec<&str> = source.lines().collect();
    lines
        .iter()
        .enumerate()
        .flat_map(|(i, line)| {
            js_line_unwrap((i + 1) as u32, line.trim(), lines.get(i + 1).copied())
        })
        .collect()
}

/// An empty block closing a line (`… {}`), or braces alone on a line.
fn is_empty_catch_tail(line: &str) -> bool {
    line.ends_with("{}") || line.ends_with("{ }")
}

/// An empty block alone on a line (the catch body on the next line).
fn is_bare_empty_block(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed == "{}" || trimmed == "{ }"
}

/// Classify one JS line: `catch` followed by an empty block on the same
/// line, or an empty block alone on the next line.
fn js_line_unwrap(line_num: u32, trimmed: &str, next: Option<&str>) -> Vec<UnwrapSite> {
    // Empty catch block: catch {} or catch(e) {} on same line
    let same_line = trimmed.contains("catch") && is_empty_catch_tail(trimmed);
    // catch on one line, empty block on next
    let next_line = trimmed.contains("catch") && next.is_some_and(is_bare_empty_block);
    [same_line, next_line]
        .iter()
        .filter(|hits| **hits)
        .map(|_| site(line_num, UnwrapKind::EmptyCatch))
        .collect()
}

fn detect_java_unwraps(source: &str) -> Vec<UnwrapSite> {
    let mut sites = Vec::new();
    for (i, line) in source.lines().enumerate() {
        let line_num = (i + 1) as u32;
        let trimmed = line.trim();
        // catch (Exception e) {} — overly broad catch
        if trimmed.contains("catch (Exception")
            && (trimmed.ends_with("{}") || trimmed.ends_with("{ }"))
        {
            sites.push(UnwrapSite {
                line: line_num,
                kind: UnwrapKind::EmptyCatch,
            });
        }
    }
    sites
}

fn detect_go_unwraps(source: &str) -> Vec<UnwrapSite> {
    source
        .lines()
        .enumerate()
        .flat_map(|(i, line)| go_line_unwrap((i + 1) as u32, line.trim()))
        .collect()
}

/// Classify one Go line: ignored errors and bare panic/recover.
/// Note: `if err != nil { return err }` is idiomatic Go — NOT flagged.
fn go_line_unwrap(line_num: u32, trimmed: &str) -> Vec<UnwrapSite> {
    // Ignored error: _ = fn()
    let ignored = trimmed.starts_with("_ = ") || trimmed.starts_with("_, ");
    // Bare panic or recover outside of error check
    let bare_panic =
        trimmed == "panic(" || trimmed.starts_with("panic(") && !trimmed.starts_with("panic(err");
    let candidates = [
        (ignored, UnwrapKind::IgnoredError),
        (bare_panic, UnwrapKind::BarePanic),
    ];
    candidates
        .into_iter()
        .filter(|(hit, _)| *hit)
        .map(|(_, kind)| site(line_num, kind))
        .collect()
}
