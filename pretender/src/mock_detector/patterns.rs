//! Per-language mock pattern tables.
//!
//! Purpose: declare the infrastructure vs usage string patterns each
//! language's mock frameworks produce, in observable group order.
//! Rationale: split out of `mock_detector.rs` so the pattern data lives
//! apart from the scanning logic; pattern order within each group is
//! observable and unchanged.

/// Per-language mock pattern definitions.
pub(super) struct LangMockPatterns {
    /// Patterns that indicate mock infrastructure setup (not counted).
    pub(super) infrastructure: Vec<String>,
    /// Patterns that indicate mock usage (counted toward threshold).
    pub(super) usage: Vec<String>,
}

pub(super) fn rust_patterns() -> LangMockPatterns {
    LangMockPatterns {
        infrastructure: rust_infrastructure(),
        usage: rust_usage(),
    }
}

fn rust_infrastructure() -> Vec<String> {
    vec![
        "use mockall".into(),
        "use mockito".into(),
        "use mock".into(),
        "#[mock".into(),
        "#[automock".into(),
        "mockall::".into(),
        "mockito::".into(),
    ]
}

fn rust_usage() -> Vec<String> {
    vec![
        "Mock::new()".into(),
        "Mock::default()".into(),
        "mock!".into(),
        "expect_".into(),
        ".expect(".into(),
        "mockito::mock".into(),
        "mockito::Matcher".into(),
        ".create_mock(".into(),
    ]
}

pub(super) fn python_patterns() -> LangMockPatterns {
    LangMockPatterns {
        infrastructure: python_infrastructure(),
        usage: python_usage(),
    }
}

fn python_infrastructure() -> Vec<String> {
    vec![
        "import mock".into(),
        "from mock".into(),
        "import unittest.mock".into(),
        "from unittest.mock".into(),
        "import pytest_mock".into(),
    ]
}

fn python_usage() -> Vec<String> {
    vec![
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
    ]
}

pub(super) fn typescript_patterns() -> LangMockPatterns {
    LangMockPatterns {
        infrastructure: typescript_infrastructure(),
        usage: typescript_usage(),
    }
}

fn typescript_infrastructure() -> Vec<String> {
    vec![
        "jest.mock".into(),
        "vi.mock".into(),
        "import sinon".into(),
        "from sinon".into(),
    ]
}

fn typescript_usage() -> Vec<String> {
    vec![
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
    ]
}

pub(super) fn java_patterns() -> LangMockPatterns {
    LangMockPatterns {
        infrastructure: java_infrastructure(),
        usage: java_usage(),
    }
}

fn java_infrastructure() -> Vec<String> {
    vec![
        "import org.mockito".into(),
        "import org.easymock".into(),
        "import org.powermock".into(),
        "@Mock".into(),
        "@MockBean".into(),
        "@InjectMocks".into(),
    ]
}

fn java_usage() -> Vec<String> {
    vec![
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
    ]
}

pub(super) fn go_patterns() -> LangMockPatterns {
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

pub(super) fn ruby_patterns() -> LangMockPatterns {
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

pub(super) fn c_cpp_patterns() -> LangMockPatterns {
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
