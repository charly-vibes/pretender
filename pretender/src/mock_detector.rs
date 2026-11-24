//! Purpose: mock-reference detection for test files.
//!
//! Responsibilities: detect mock, stub, fake, and spy references via
//! per-language pattern matching; separate mock *infrastructure* (imports,
//! trait annotations) from mock *usage* (construction, expectations); only
//! usage references count toward the configurable threshold.
//!
//! Rationale: imports and setup alone don't indicate mock overuse, so
//! counting only usage keeps the overuse threshold meaningful.

use crate::model::Language;
use std::path::Path;

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

    // Group order is observable: infrastructure first, then usage, then the
    // user-defined extras (treated as usage).
    let groups: [(&[String], MockKind); 3] = [
        (&patterns.infrastructure, MockKind::Infrastructure),
        (&patterns.usage, MockKind::Usage),
        (extra_patterns, MockKind::Usage),
    ];

    let mut analysis = MockAnalysis::default();
    for (line_idx, line) in source.lines().enumerate() {
        scan_line(&mut analysis, line, (line_idx + 1) as u32, &groups);
    }
    analysis
}

/// Select the per-language pattern set; `None` for languages without patterns.
fn patterns_for(language: &Language) -> Option<LangMockPatterns> {
    match language {
        Language::Rust => Some(rust_patterns()),
        Language::Python => Some(python_patterns()),
        Language::JavaScript | Language::TypeScript => Some(typescript_patterns()),
        Language::Java => Some(java_patterns()),
        Language::Go => Some(go_patterns()),
        Language::Ruby => Some(ruby_patterns()),
        Language::C | Language::Cpp => Some(c_cpp_patterns()),
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
        for pattern in patterns {
            if trimmed.contains(pattern.as_str()) {
                analysis.record(pattern, line_num, kind);
            }
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

/// Per-language mock pattern definitions.
struct LangMockPatterns {
    /// Patterns that indicate mock infrastructure setup (not counted).
    infrastructure: Vec<String>,
    /// Patterns that indicate mock usage (counted toward threshold).
    usage: Vec<String>,
}

fn rust_patterns() -> LangMockPatterns {
    LangMockPatterns {
        infrastructure: vec![
            "use mockall".into(),
            "use mockito".into(),
            "use mock".into(),
            "#[mock".into(),
            "#[automock".into(),
            "mockall::".into(),
            "mockito::".into(),
        ],
        usage: vec![
            "Mock::new()".into(),
            "Mock::default()".into(),
            "mock!".into(),
            "expect_".into(),
            ".expect(".into(),
            "mockito::mock".into(),
            "mockito::Matcher".into(),
            ".create_mock(".into(),
        ],
    }
}

fn python_patterns() -> LangMockPatterns {
    LangMockPatterns {
        infrastructure: vec![
            "import mock".into(),
            "from mock".into(),
            "import unittest.mock".into(),
            "from unittest.mock".into(),
            "import pytest_mock".into(),
        ],
        usage: vec![
            "MagicMock".into(),
            "Mock()".into(),
            "patch(".into(),
            "patch.object(".into(),
            "mock.patch(".into(),
            "mock.MagicMock".into(),
            "mock.Mock".into(),
            "mocker.patch".into(),
            "mocker.MagicMock".into(),
            ".return_value".into(),
            ".side_effect".into(),
        ],
    }
}

fn typescript_patterns() -> LangMockPatterns {
    LangMockPatterns {
        infrastructure: vec![
            "jest.mock".into(),
            "vi.mock".into(),
            "import sinon".into(),
            "from sinon".into(),
        ],
        usage: vec![
            "jest.fn()".into(),
            "jest.spyOn(".into(),
            "vi.fn()".into(),
            "vi.spyOn(".into(),
            "sinon.stub(".into(),
            "sinon.mock(".into(),
            "sinon.spy(".into(),
            "sinon.fake(".into(),
            ".mockImplementation(".into(),
            ".mockReturnValue(".into(),
            ".mockResolvedValue(".into(),
            ".mockRejectedValue(".into(),
        ],
    }
}

fn java_patterns() -> LangMockPatterns {
    LangMockPatterns {
        infrastructure: vec![
            "import org.mockito".into(),
            "import org.easymock".into(),
            "import org.powermock".into(),
            "@Mock".into(),
            "@MockBean".into(),
            "@InjectMocks".into(),
        ],
        usage: vec![
            "Mockito.mock(".into(),
            "Mockito.when(".into(),
            "Mockito.verify(".into(),
            "Mockito.any(".into(),
            "Mockito.eq(".into(),
            "EasyMock.mock(".into(),
            "EasyMock.expect(".into(),
            "EasyMock.verify(".into(),
            "PowerMock.mock(".into(),
            ".when(".into(),
            ".verify(".into(),
        ],
    }
}

fn go_patterns() -> LangMockPatterns {
    LangMockPatterns {
        infrastructure: vec![
            "import \"github.com/stretchr/testify/mock\"".into(),
            "import \"github.com/golang/mock".into(),
            "mock.Controller".into(),
        ],
        usage: vec![
            "mock.Mock".into(),
            "mock.Anything".into(),
            ".On(".into(),
            ".Return(".into(),
            ".AssertExpectations(".into(),
            "ctrl := gomock.NewController".into(),
            "NewMock".into(),
        ],
    }
}

fn ruby_patterns() -> LangMockPatterns {
    LangMockPatterns {
        infrastructure: vec![
            "require 'rspec/mocks'".into(),
            "require 'mocha'".into(),
            "require 'minitest/mock'".into(),
            "include RSpec::Mocks".into(),
        ],
        usage: vec![
            "double(".into(),
            "mock(".into(),
            "stub(".into(),
            "expect(".into(),
            "allow(".into(),
            "receive(".into(),
            "and_return(".into(),
            "and_raise(".into(),
        ],
    }
}

fn c_cpp_patterns() -> LangMockPatterns {
    LangMockPatterns {
        infrastructure: vec![
            "#include <gmock".into(),
            "#include \"gmock".into(),
            "#include <fake".into(),
            "#include \"fake".into(),
        ],
        usage: vec![
            "MOCK_METHOD".into(),
            "EXPECT_CALL".into(),
            "ON_CALL".into(),
            "FAKE_VALUE_FUNC".into(),
            "FAKE_VOID_FUNC".into(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_rust_mock_usage() {
        let source = r#"
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
"#;
        let analysis = detect_mocks(source, &Language::Rust, &[], Path::new("test.rs"));
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
        // user-defined extra. Group order and usage counting are observable
        // behavior the scanning helpers must preserve.
        let source = "let x = mockall::verifier; expect_x(); custom_mock();";
        let analysis = detect_mocks(
            source,
            &Language::Rust,
            &["custom_mock".to_string()],
            Path::new("test.rs"),
        );
        let names: Vec<&str> = analysis
            .references
            .iter()
            .map(|r| r.name.as_str())
            .collect();
        assert_eq!(names, vec!["mockall::", "expect_", "custom_mock"]);
        let kinds: Vec<MockKind> = analysis.references.iter().map(|r| r.kind).collect();
        assert_eq!(
            kinds,
            vec![MockKind::Infrastructure, MockKind::Usage, MockKind::Usage]
        );
        assert_eq!(analysis.total_usage, 2);
    }
}
