//! Role resolution for a source file.
//!
//! Resolves a file's [`Role`] from explicit `pretender:`/`pretender-role:`
//! pragmas, configured path globs, and heuristic path rules, in that
//! precedence order.
//!
//! # Responsibilities
//! [`RoleDetector`] compiles the configured globs (with specificity
//! ordering) and applies pragma > glob > heuristic > App fallback;
//! the pragma and comment parsers recognize both supported spellings.
//!
//! # Rationale
//! Detection machinery (config, globset, source-text coupling) is split out
//! of `roles.rs` so that module keeps only the pure role and threshold
//! tables; bodies moved verbatim to keep role assignment identical.
use crate::config::Config;
use crate::roles::{heuristic_role, Role};
use globset::{Glob, GlobMatcher};
use std::path::Path;

pub struct RoleDetector {
    patterns: Vec<RolePattern>,
}

impl RoleDetector {
    pub fn new(config: &Config) -> Result<Self, globset::Error> {
        let mut patterns = Vec::new();
        add_patterns(&mut patterns, Role::Test, &config.roles.test.paths)?;
        add_patterns(&mut patterns, Role::Library, &config.roles.library.paths)?;
        add_patterns(&mut patterns, Role::Script, &config.roles.script.paths)?;
        add_patterns(
            &mut patterns,
            Role::Generated,
            &config.roles.generated.paths,
        )?;
        add_patterns(&mut patterns, Role::Vendor, &config.roles.vendor.paths)?;
        add_patterns(&mut patterns, Role::UnitTest, &config.roles.unit_test.paths)?;
        add_patterns(
            &mut patterns,
            Role::IntegrationTest,
            &config.roles.integration_test.paths,
        )?;
        Ok(Self { patterns })
    }

    pub fn detect(&self, path: &Path, source: &str) -> Role {
        explicit_pragma_role(source)
            .or_else(|| self.glob_role(path))
            .or_else(|| heuristic_role(path))
            .unwrap_or(Role::App)
    }

    fn glob_role(&self, path: &Path) -> Option<Role> {
        self.patterns
            .iter()
            .filter(|pattern| pattern.matcher.is_match(path))
            .max_by_key(|pattern| pattern.specificity)
            .map(|pattern| pattern.role)
    }
}

struct RolePattern {
    role: Role,
    matcher: GlobMatcher,
    specificity: usize,
}

fn add_patterns(
    patterns: &mut Vec<RolePattern>,
    role: Role,
    globs: &[String],
) -> Result<(), globset::Error> {
    for pattern in globs {
        patterns.push(RolePattern {
            role,
            matcher: Glob::new(pattern)?.compile_matcher(),
            specificity: pattern
                .chars()
                .filter(|c| !matches!(c, '*' | '?' | '[' | ']'))
                .count(),
        });
    }
    Ok(())
}

fn explicit_pragma_role(source: &str) -> Option<Role> {
    source
        .lines()
        .take(8)
        .filter_map(comment_text)
        .find_map(parse_role_pragma)
}

fn comment_text(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    trimmed
        .strip_prefix('#')
        .or_else(|| trimmed.strip_prefix("//"))
        .or_else(|| trimmed.strip_prefix("/*"))
        .map(str::trim)
}

fn parse_role_pragma(comment: &str) -> Option<Role> {
    let lower = comment
        .trim()
        .trim_end_matches("*/")
        .trim()
        .to_ascii_lowercase();
    let value = if let Some(rest) = lower.strip_prefix("pretender:") {
        let (key, value) = rest.split_once('=')?;
        (key.trim() == "role").then_some(value.trim())?
    } else {
        lower.strip_prefix("pretender-role:").map(str::trim)?
    };
    Role::parse(value)
}

impl Role {
    fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "app" => Some(Self::App),
            "library" | "lib" => Some(Self::Library),
            "test" | "tests" | "spec" => Some(Self::Test),
            "script" | "scripts" => Some(Self::Script),
            "generated" => Some(Self::Generated),
            "vendor" => Some(Self::Vendor),
            "unit-test" | "unit_test" | "unit" => Some(Self::UnitTest),
            "integration-test" | "integration_test" | "integration" => Some(Self::IntegrationTest),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests;
