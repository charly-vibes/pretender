use anyhow::{anyhow, Result};
use std::path::Path;
use std::sync::OnceLock;

use crate::engine::QueryEngine;
use crate::model::{Diagnostic, Language, Module, Parser};
use crate::plugin::PluginManifest;

const TS_QUERY: &str = include_str!("../languages/typescript/metrics.scm");
const TS_MANIFEST: &str = include_str!("../languages/typescript/plugin.toml");

pub struct TypeScriptParser;

fn manifest() -> Result<&'static PluginManifest> {
    static MANIFEST: OnceLock<std::result::Result<PluginManifest, String>> = OnceLock::new();
    match MANIFEST
        .get_or_init(|| PluginManifest::from_toml(TS_MANIFEST).map_err(|err| err.to_string()))
    {
        Ok(manifest) => Ok(manifest),
        Err(err) => Err(anyhow!(err.clone())),
    }
}

impl TypeScriptParser {
    fn engine(&self) -> Result<QueryEngine> {
        let m = manifest()?;
        QueryEngine::new_with_branch_weights(
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Language::TypeScript,
            TS_QUERY,
            &crate::engine::WeightMaps {
                branch: &m.branches,
                call: &m.smell_weights,
            },
        )
    }
}

impl Parser for TypeScriptParser {
    fn parse(&self, path: &Path, source: &str) -> Result<(Module, Vec<Diagnostic>)> {
        self.engine()?.parse(path, source)
    }
}

pub struct TypeScriptXParser;

impl TypeScriptXParser {
    fn engine(&self) -> Result<QueryEngine> {
        let m = manifest()?;
        QueryEngine::new_with_branch_weights(
            tree_sitter_typescript::LANGUAGE_TSX.into(),
            Language::TypeScript,
            TS_QUERY,
            &crate::engine::WeightMaps {
                branch: &m.branches,
                call: &m.smell_weights,
            },
        )
    }
}

impl Parser for TypeScriptXParser {
    fn parse(&self, path: &Path, source: &str) -> Result<(Module, Vec<Diagnostic>)> {
        self.engine()?.parse(path, source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Span, TypeKind, UnitKind};
    use std::path::Path;

    fn engine() -> QueryEngine {
        QueryEngine::new(
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Language::TypeScript,
            TS_QUERY,
        )
        .unwrap()
    }

    #[test]
    fn ts_types_fixture_round_trip() {
        // Reusable harness pattern (tasks 6.4): the language's real
        // metrics.scm parses a fixture file; assert TypeDefinition
        // count/kind/ordered fields and that CodeUnits stay separate.
        let e = engine();
        let source = include_str!("../tests/fixtures/ts_types_sample.ts");
        let (module, diags) = e.parse(Path::new("ts_types_sample.ts"), source).unwrap();

        assert!(diags.is_empty(), "fixture must parse cleanly: {diags:?}");

        assert_eq!(
            module.type_definitions.len(),
            2,
            "both classes captured; standalone fn is not a type definition"
        );

        let person = module
            .type_definitions
            .iter()
            .find(|t| t.name == "Person")
            .expect("Person captured");
        assert_eq!(person.kind, TypeKind::Class);
        assert_eq!(person.language, Language::TypeScript);
        assert_eq!(
            person.span,
            Span {
                start_line: 1,
                end_line: 9
            }
        );
        let person_fields: Vec<(&str, Option<&str>)> = person
            .fields
            .iter()
            .map(|f| (f.name.as_str(), f.type_name.as_deref()))
            .collect();
        assert_eq!(
            person_fields,
            vec![
                ("name", Some("string")),
                ("age", Some("number")),
                ("email", Some("string")),
            ],
            "typed class fields only, in declaration order, type text from the annotation"
        );

        let pair = module
            .type_definitions
            .iter()
            .find(|t| t.name == "Pair")
            .expect("generic class captured");
        assert_eq!(pair.kind, TypeKind::Class);
        assert!(
            pair.name.chars().all(|c| c.is_alphanumeric() || c == '_'),
            "captured name must be bare identifier, no type-parameter syntax: {:?}",
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
            "type parameter text must not leak into captured values"
        );

        // CodeUnits remain separate from type members.
        assert_eq!(module.units.len(), 3);
        let mut unit_names: Vec<&str> = module.units.iter().map(|u| u.name.as_str()).collect();
        unit_names.sort_unstable();
        assert_eq!(unit_names, vec!["greet", "standalone", "swap"]);
        assert!(
            person.fields.iter().all(|f| f.name != "greet"),
            "methods must not be duplicated as fields"
        );
    }

    #[test]
    fn ts_function_valued_class_properties_are_units_not_fields() {
        // Contract: members-not-duplicated — function-valued class
        // properties are CodeUnits (kind follows the existing TS
        // convention, where method_definition members are Function too)
        // and never data fields.
        let e = engine();
        let source = "\
class Widget {
  label: string = 'ok';

  onClick = (): void => {
    return;
  };

  render(): string {
    return this.label;
  }
}
";
        let (module, diags) = e.parse(Path::new("widget.ts"), source).unwrap();

        assert!(diags.is_empty(), "fixture must parse cleanly: {diags:?}");

        let widget = module
            .type_definitions
            .iter()
            .find(|t| t.name == "Widget")
            .expect("Widget captured");
        let field_names: Vec<&str> = widget.fields.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(
            field_names,
            vec!["label"],
            "arrow-valued property is a CodeUnit, not a data field"
        );

        let unit_names: Vec<&str> = module.units.iter().map(|u| u.name.as_str()).collect();
        assert_eq!(
            unit_names,
            vec!["onClick", "render"],
            "function-valued property surfaces as a CodeUnit like methods"
        );
        for name in ["onClick", "render"] {
            let unit = module.units.iter().find(|u| u.name == name).unwrap();
            assert_eq!(
                unit.kind,
                UnitKind::Function,
                "{name} follows the existing TS member kind convention"
            );
        }
        assert!(
            widget.fields.iter().all(|f| f.name != "onClick"),
            "members must not be duplicated as fields"
        );
    }
}
