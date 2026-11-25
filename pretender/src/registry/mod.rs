//! Declarative language registry: one [`LangRecord`] per parser
//! configuration, one [`GenericParser`] for all of them.
//!
//! # Responsibilities
//! Own the extension → parser mapping as data; construct `QueryEngine`s
//! from a record's grammar binding, language, query, and manifest-derived
//! weight maps; resolve a file extension to its record.
//!
//! # Rationale
//! Replaces the per-extension `match` in `check/scan/parsers.rs` and the
//! 13× repeated `manifest()`/`engine()`/`impl Parser` boilerplate in the
//! per-language modules (design: `.wai/projects/language-registry/design/
//! 2026-11-25-registry-design.md`; research: `research/2026-11-25-dispatch-
//! surface.md`, ticket pretender-948q). Adding a language touches data plus
//! one enum variant each for `model::Language` and [`Grammar`] — no
//! dispatch-site edits. `tree_sitter` grammars are statically linked, so
//! [`Grammar`] is the one inherently non-data bit; its exhaustive match
//! extends, never rewrites. The `include_str!`d data files live in the
//! sibling `languages/<lang>/` directories.

use anyhow::{anyhow, Result};
use std::path::Path;
use std::sync::OnceLock;

use crate::engine::{QueryEngine, WeightMaps};
use crate::model::{Diagnostic, Language, Module, Parser};
use crate::plugin::PluginManifest;

pub mod grammar;

pub use grammar::Grammar;

/// One parser configuration. All fields are data; `manifest` is parsed once
/// per record and cached for engine construction.
pub struct LangRecord {
    pub grammar: Grammar,
    pub language: Language,
    pub extensions: &'static [&'static str],
    pub manifest: &'static str,
    pub query: &'static str,
    manifest_cache: OnceLock<std::result::Result<PluginManifest, String>>,
}

impl LangRecord {
    pub(crate) fn manifest(&self) -> Result<&PluginManifest> {
        let parsed = self.manifest_cache.get_or_init(|| {
            PluginManifest::from_toml(self.manifest).map_err(|err| err.to_string())
        });
        match parsed {
            Ok(manifest) => Ok(manifest),
            Err(err) => Err(anyhow!(err.clone())),
        }
    }
}

/// The single [`Parser`] implementation: behavior is fully determined by the
/// record. TS and TSX are two records over one grammar crate.
pub struct GenericParser {
    pub record: &'static LangRecord,
}

impl Parser for GenericParser {
    fn parse(&self, path: &Path, source: &str) -> Result<(Module, Vec<Diagnostic>)> {
        let manifest = self.record.manifest()?;
        let engine = QueryEngine::new_with_branch_weights(
            self.record.grammar.language(),
            self.record.language.clone(),
            self.record.query,
            &WeightMaps {
                branch: &manifest.metric_sections.branches,
                call: &manifest.metric_sections.smell_weights,
                nesting: &manifest.metric_sections.nesting,
            },
        )?;
        engine.parse(path, source)
    }
}

/// Resolve `ext` to its record. Exact match, as the previous `match` arms —
/// `.PY` stays unsupported; the `r`/`R` pair is data.
pub fn lookup(ext: &str) -> Option<&'static LangRecord> {
    REGISTRY
        .iter()
        .find(|record| record.extensions.contains(&ext))
}

/// Whether `ext` is parser-backed, per the same registry the dispatch honors.
pub fn supports_extension(ext: &str) -> bool {
    lookup(ext).is_some()
}

pub static REGISTRY: [LangRecord; 14] = [
    LangRecord {
        grammar: Grammar::CSharp,
        language: Language::CSharp,
        extensions: &["cs"],
        manifest: include_str!("../../languages/csharp/plugin.toml"),
        query: include_str!("../../languages/csharp/metrics.scm"),
        manifest_cache: OnceLock::new(),
    },
    LangRecord {
        grammar: Grammar::Clojure,
        language: Language::Clojure,
        extensions: &["clj", "cljs", "cljc", "edn"],
        manifest: include_str!("../../languages/clojure/plugin.toml"),
        query: include_str!("../../languages/clojure/metrics.scm"),
        manifest_cache: OnceLock::new(),
    },
    LangRecord {
        grammar: Grammar::C,
        language: Language::C,
        extensions: &["c", "h"],
        manifest: include_str!("../../languages/c/plugin.toml"),
        query: include_str!("../../languages/c/metrics.scm"),
        manifest_cache: OnceLock::new(),
    },
    LangRecord {
        grammar: Grammar::Cpp,
        language: Language::Cpp,
        extensions: &["cpp", "cc", "cxx", "hpp", "hxx"],
        manifest: include_str!("../../languages/cpp/plugin.toml"),
        query: include_str!("../../languages/cpp/metrics.scm"),
        manifest_cache: OnceLock::new(),
    },
    LangRecord {
        grammar: Grammar::Go,
        language: Language::Go,
        extensions: &["go"],
        manifest: include_str!("../../languages/go/plugin.toml"),
        query: include_str!("../../languages/go/metrics.scm"),
        manifest_cache: OnceLock::new(),
    },
    LangRecord {
        grammar: Grammar::Java,
        language: Language::Java,
        extensions: &["java"],
        manifest: include_str!("../../languages/java/plugin.toml"),
        query: include_str!("../../languages/java/metrics.scm"),
        manifest_cache: OnceLock::new(),
    },
    LangRecord {
        grammar: Grammar::Julia,
        language: Language::Julia,
        extensions: &["jl"],
        manifest: include_str!("../../languages/julia/plugin.toml"),
        query: include_str!("../../languages/julia/metrics.scm"),
        manifest_cache: OnceLock::new(),
    },
    LangRecord {
        grammar: Grammar::JavaScript,
        language: Language::JavaScript,
        extensions: &["js", "jsx", "mjs", "cjs"],
        manifest: include_str!("../../languages/javascript/plugin.toml"),
        query: include_str!("../../languages/javascript/metrics.scm"),
        manifest_cache: OnceLock::new(),
    },
    LangRecord {
        grammar: Grammar::Python,
        language: Language::Python,
        extensions: &["py"],
        manifest: include_str!("../../languages/python/plugin.toml"),
        query: include_str!("../../languages/python/metrics.scm"),
        manifest_cache: OnceLock::new(),
    },
    LangRecord {
        grammar: Grammar::R,
        language: Language::R,
        extensions: &["r", "R"],
        manifest: include_str!("../../languages/r/plugin.toml"),
        query: include_str!("../../languages/r/metrics.scm"),
        manifest_cache: OnceLock::new(),
    },
    LangRecord {
        grammar: Grammar::Ruby,
        language: Language::Ruby,
        extensions: &["rb"],
        manifest: include_str!("../../languages/ruby/plugin.toml"),
        query: include_str!("../../languages/ruby/metrics.scm"),
        manifest_cache: OnceLock::new(),
    },
    LangRecord {
        grammar: Grammar::Rust,
        language: Language::Rust,
        extensions: &["rs"],
        manifest: include_str!("../../languages/rust/plugin.toml"),
        query: include_str!("../../languages/rust/metrics.scm"),
        manifest_cache: OnceLock::new(),
    },
    LangRecord {
        grammar: Grammar::TypeScript,
        language: Language::TypeScript,
        extensions: &["ts", "mts"],
        manifest: include_str!("../../languages/typescript/plugin.toml"),
        query: include_str!("../../languages/typescript/metrics.scm"),
        manifest_cache: OnceLock::new(),
    },
    LangRecord {
        grammar: Grammar::Tsx,
        language: Language::TypeScript,
        extensions: &["tsx", "cts"],
        manifest: include_str!("../../languages/typescript/plugin.toml"),
        query: include_str!("../../languages/typescript/metrics.scm"),
        manifest_cache: OnceLock::new(),
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Parser as ParserTrait;
    use std::collections::BTreeSet;

    #[test]
    fn extensions_are_unique_across_records() {
        let mut seen = BTreeSet::new();
        for record in REGISTRY.iter() {
            for ext in record.extensions {
                assert!(seen.insert(*ext), "duplicate extension {ext:?} in registry");
            }
        }
    }

    #[test]
    fn every_manifest_parses_via_the_contract() {
        for record in REGISTRY.iter() {
            let manifest = record
                .manifest()
                .unwrap_or_else(|e| panic!("manifest for {:?} must parse: {e:?}", record.language));
            assert!(!manifest.extensions.is_empty());
            assert_eq!(manifest.query, "metrics.scm");
        }
    }

    #[test]
    fn lookup_resolves_every_registered_extension() {
        for record in REGISTRY.iter() {
            for ext in record.extensions {
                let found = lookup(ext).unwrap_or_else(|| panic!("{ext:?} must resolve"));
                assert_eq!(found.language, record.language);
                assert_eq!(found.grammar, record.grammar);
            }
        }
    }

    #[test]
    fn lookup_misses_unknown_extensions() {
        assert!(lookup("zip").is_none());
        assert!(
            lookup("PY").is_none(),
            "case-insensitive lookup would change behavior"
        );
    }

    #[test]
    fn every_grammar_constructs_an_engine() {
        for record in REGISTRY.iter() {
            let manifest = record.manifest().expect("parses");
            let engine = QueryEngine::new_with_branch_weights(
                record.grammar.language(),
                record.language.clone(),
                record.query,
                &WeightMaps {
                    branch: &manifest.metric_sections.branches,
                    call: &manifest.metric_sections.smell_weights,
                    nesting: &manifest.metric_sections.nesting,
                },
            );
            assert!(
                engine.is_ok(),
                "{:?} engine must construct",
                record.language
            );
        }
    }

    #[test]
    fn generic_parser_round_trips_a_python_sample() {
        let record = lookup("py").expect("py registered");
        let parser = GenericParser { record };
        let source = include_str!("../../tests/fixtures/python_simple.py");
        let (module, diagnostics) = parser
            .parse(Path::new("tests/fixtures/python_simple.py"), source)
            .unwrap_or_else(|e| panic!("python_simple.py must parse: {e:?}"));
        assert!(diagnostics.is_empty(), "unexpected: {diagnostics:?}");
        assert!(
            module.units.iter().any(|u| u.name == "simple"),
            "units must capture the fixture function"
        );
    }
}
