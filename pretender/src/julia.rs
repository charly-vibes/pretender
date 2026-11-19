use anyhow::{anyhow, Result};
use std::path::Path;
use std::sync::OnceLock;

use crate::engine::QueryEngine;
use crate::model::{Diagnostic, Language, Module, Parser};
use crate::plugin::PluginManifest;

const JULIA_QUERY: &str = include_str!("../languages/julia/metrics.scm");
const JULIA_MANIFEST: &str = include_str!("../languages/julia/plugin.toml");

pub struct JuliaParser;

fn manifest() -> Result<&'static PluginManifest> {
    static MANIFEST: OnceLock<std::result::Result<PluginManifest, String>> = OnceLock::new();
    match MANIFEST
        .get_or_init(|| PluginManifest::from_toml(JULIA_MANIFEST).map_err(|err| err.to_string()))
    {
        Ok(manifest) => Ok(manifest),
        Err(err) => Err(anyhow!(err.clone())),
    }
}

impl JuliaParser {
    fn engine(&self) -> Result<QueryEngine> {
        let m = manifest()?;
        QueryEngine::new_with_branch_weights(
            tree_sitter_julia::LANGUAGE.into(),
            Language::Julia,
            JULIA_QUERY,
            &m.branches,
            &m.smell_weights,
        )
    }
}

impl Parser for JuliaParser {
    fn parse(&self, path: &Path, source: &str) -> Result<(Module, Vec<Diagnostic>)> {
        self.engine()?.parse(path, source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::QueryEngine;
    use crate::model::{Span, TypeKind};
    use std::path::Path;

    fn julia_engine() -> QueryEngine {
        QueryEngine::new(
            tree_sitter_julia::LANGUAGE.into(),
            Language::Julia,
            JULIA_QUERY,
        )
        .unwrap()
    }

    #[test]
    fn julia_types_fixture_round_trip() {
        // Reusable harness pattern (tasks 6.1/6.6): the language's real
        // metrics.scm parses a fixture file; assert TypeDefinition
        // count/kind/ordered fields and that CodeUnits stay separate.
        // struct and mutable struct both parse as struct_definition and
        // must both land as Struct with their fields in declaration order.
        let engine = julia_engine();
        let source = include_str!("../tests/fixtures/julia_types_sample.jl");
        let (module, diags) = engine
            .parse(Path::new("julia_types_sample.jl"), source)
            .unwrap();

        assert!(diags.is_empty(), "fixture must parse cleanly: {diags:?}");

        assert_eq!(
            module.type_definitions.len(),
            2,
            "struct and mutable struct captured, functions are not type definitions"
        );

        let person = module
            .type_definitions
            .iter()
            .find(|t| t.name == "Person")
            .expect("Person captured");
        assert_eq!(person.kind, TypeKind::Struct);
        assert_eq!(person.language, Language::Julia);
        assert_eq!(person.span, Span { start_line: 1, end_line: 5 });
        let person_fields: Vec<(&str, Option<&str>)> = person
            .fields
            .iter()
            .map(|f| (f.name.as_str(), f.type_name.as_deref()))
            .collect();
        assert_eq!(
            person_fields,
            vec![
                ("name", Some("String")),
                ("age", Some("Int")),
                ("email", Some("String")),
            ],
            "typed field declarations in source order"
        );

        let pair = module
            .type_definitions
            .iter()
            .find(|t| t.name == "Pair")
            .expect("parametric mutable struct captured");
        assert_eq!(pair.kind, TypeKind::Struct);
        assert!(
            pair.name
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_'),
            "captured name must be bare identifier, no grammar syntax: {:?}",
            pair.name
        );
        let pair_fields: Vec<(&str, Option<&str>)> = pair
            .fields
            .iter()
            .map(|f| (f.name.as_str(), f.type_name.as_deref()))
            .collect();
        assert_eq!(
            pair_fields,
            vec![("first", Some("T")), ("second", Some("T"))],
            "type parameter syntax must not leak into captured values"
        );

        // CodeUnits remain separate from type members.
        assert_eq!(module.units.len(), 2);
        let mut unit_names: Vec<&str> = module.units.iter().map(|u| u.name.as_str()).collect();
        unit_names.sort_unstable();
        assert_eq!(unit_names, vec!["greet", "standalone"]);
        assert!(
            person.fields.iter().all(|f| f.name != "greet"),
            "methods must not be duplicated as fields"
        );
    }
}
