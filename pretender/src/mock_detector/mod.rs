//! Mock-reference detection for test files.
//!
//! Purpose: detect mock, stub, fake, and spy references via per-language
//! pattern matching; separate mock *infrastructure* (imports, trait
//! annotations) from mock *usage* (construction, expectations); only usage
//! references count toward the configurable threshold.
//!
//! Rationale: imports and setup alone don't indicate mock overuse, so
//! counting only usage keeps the overuse threshold meaningful. The
//! per-language pattern tables live in `patterns`.

use crate::model::Language;
use std::path::Path;

use patterns::LangMockPatterns;

mod patterns;

/// A single mock reference found in source code.
#[derive(Debug, Clone)]
pub struct MockReference {
    pub name: String,
    pub line: u32,
    pub kind: MockKind,
}

/// Whether a mock reference is infrastructure setup or actual usage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MockKind {
    /// Import, use statement, or annotation that sets up mock infrastructure.
    Infrastructure,
    /// Actual mock construction, expectation, or call verification.
    Usage,
}

/// Result of mock-overuse analysis for one file.
#[derive(Debug, Clone, Default)]
pub struct MockAnalysis {
    pub total_usage: u32,
    pub references: Vec<MockReference>,
}

/// Analyze a source file for mock references.
pub fn detect_mocks(
    source: &str,
    language: &Language,
    extra_patterns: &[String],
    _path: &Path,
) -> MockAnalysis {
    let Some(patterns) = patterns_for(language) else {
        return MockAnalysis::default();
    };
    let groups = mock_pattern_groups(&patterns, extra_patterns);
    let mut analysis = MockAnalysis::default();
    for (line_idx, line) in source.lines().enumerate() {
        scan_line(&mut analysis, line, (line_idx + 1) as u32, &groups);
    }
    analysis
}

/// Observable group order: infrastructure first, then usage, then the
/// user-defined extras (treated as usage).
fn mock_pattern_groups<'a>(
    patterns: &'a LangMockPatterns,
    extra_patterns: &'a [String],
) -> [(&'a [String], MockKind); 3] {
    [
        (&patterns.infrastructure, MockKind::Infrastructure),
        (&patterns.usage, MockKind::Usage),
        (extra_patterns, MockKind::Usage),
    ]
}

/// Select the per-language pattern set; `None` for languages without patterns.
fn patterns_for(language: &Language) -> Option<LangMockPatterns> {
    match language {
        Language::Rust => Some(patterns::rust_patterns()),
        Language::Python => Some(patterns::python_patterns()),
        Language::JavaScript | Language::TypeScript => Some(patterns::typescript_patterns()),
        Language::Java => Some(patterns::java_patterns()),
        Language::Go => Some(patterns::go_patterns()),
        Language::Ruby => Some(patterns::ruby_patterns()),
        Language::C | Language::Cpp => Some(patterns::c_cpp_patterns()),
        _ => None,
    }
}

/// Scan one source line against every pattern group in group order.
fn scan_line(
    analysis: &mut MockAnalysis,
    line: &str,
    line_num: u32,
    groups: &[(&[String], MockKind)],
) {
    let trimmed = line.trim();
    for &(patterns, kind) in groups {
        for pattern in patterns.iter().filter(|p| trimmed.contains(p.as_str())) {
            analysis.record(pattern, line_num, kind);
        }
    }
}

impl MockAnalysis {
    /// Record one reference; usage references also count toward the threshold.
    fn record(&mut self, pattern: &str, line_num: u32, kind: MockKind) {
        if kind == MockKind::Usage {
            self.total_usage += 1;
        }
        self.references.push(MockReference {
            name: pattern.to_string(),
            line: line_num,
            kind,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_rust_mock_usage() {
        let analysis = detect_mocks(
            rust_mock_usage_fixture(),
            &Language::Rust,
            &[],
            Path::new("test.rs"),
        );
        assert!(analysis.total_usage > 0);
        assert!(analysis
            .references
            .iter()
            .any(|r| r.kind == MockKind::Infrastructure));
        assert!(analysis
            .references
            .iter()
            .any(|r| r.kind == MockKind::Usage));
    }

    /// Minimal Rust source hitting infrastructure and usage patterns.
    fn rust_mock_usage_fixture() -> &'static str {
        r#"
use mockall::automock;
use mockito;

#[automock]
trait MyTrait {
    fn do_something(&self);
}

fn test() {
    let mock = MockMyTrait::new();
    mock.expect_do_something().returning(|| ());
    mockito::mock("GET", "/api").create();
}
"#
    }

    #[test]
    fn detects_python_mock_usage() {
        let source = r#"
from unittest.mock import MagicMock, patch

def test_something():
    mock = MagicMock()
    mock.some_method.return_value = 42
    with patch("module.function") as p:
        p.return_value = "mocked"
"#;
        let analysis = detect_mocks(source, &Language::Python, &[], Path::new("test.py"));
        assert!(analysis.total_usage > 0);
    }

    #[test]
    fn empty_file_no_mocks() {
        let source = "fn foo() { 1 + 1 }";
        let analysis = detect_mocks(source, &Language::Rust, &[], Path::new("test.rs"));
        assert_eq!(analysis.total_usage, 0);
    }

    #[test]
    fn unsupported_language_no_mocks() {
        let source = "some content";
        let analysis = detect_mocks(source, &Language::Julia, &[], Path::new("test.jl"));
        assert_eq!(analysis.total_usage, 0);
    }

    #[test]
    fn extra_patterns_are_detected() {
        let source = "custom_mock_helper()";
        let analysis = detect_mocks(
            source,
            &Language::Rust,
            &["custom_mock_helper".to_string()],
            Path::new("test.rs"),
        );
        assert_eq!(analysis.total_usage, 1);
    }

    #[test]
    fn references_keep_kind_grouping_order_within_a_line() {
        // One line hitting all three pattern groups: infrastructure, usage,
        // user-defined extra. Group order and usage counting are observable.
        let source = "let x = mockall::verifier; expect_x(); custom_mock();";
        let analysis = rust_extra_scan(source, "custom_mock");
        assert_eq!(
            reference_pairs(&analysis),
            vec![
                ("mockall::", MockKind::Infrastructure),
                ("expect_", MockKind::Usage),
                ("custom_mock", MockKind::Usage),
            ]
        );
        assert_eq!(analysis.total_usage, 2);
    }

    /// Scan a Rust snippet with one extra user-defined usage pattern.
    fn rust_extra_scan(source: &str, extra: &str) -> MockAnalysis {
        detect_mocks(
            source,
            &Language::Rust,
            &[extra.to_string()],
            Path::new("test.rs"),
        )
    }

    /// (name, kind) pairs in recorded order, for order assertions.
    fn reference_pairs(analysis: &MockAnalysis) -> Vec<(&str, MockKind)> {
        analysis
            .references
            .iter()
            .map(|r| (r.name.as_str(), r.kind))
            .collect()
    }
}
