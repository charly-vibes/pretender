use anyhow::{anyhow, Result};
use std::path::Path;
use std::sync::OnceLock;

use crate::engine::QueryEngine;
use crate::model::{Diagnostic, Language, Module, Parser};
use crate::plugin::PluginManifest;

const CSHARP_QUERY: &str = include_str!("../languages/csharp/metrics.scm");
const CSHARP_MANIFEST: &str = include_str!("../languages/csharp/plugin.toml");

pub struct CSharpParser;

fn manifest() -> Result<&'static PluginManifest> {
    static MANIFEST: OnceLock<std::result::Result<PluginManifest, String>> = OnceLock::new();
    match MANIFEST
        .get_or_init(|| PluginManifest::from_toml(CSHARP_MANIFEST).map_err(|err| err.to_string()))
    {
        Ok(manifest) => Ok(manifest),
        Err(err) => Err(anyhow!(err.clone())),
    }
}

impl CSharpParser {
    fn engine(&self) -> Result<QueryEngine> {
        let m = manifest()?;
        QueryEngine::new_with_branch_weights(
            tree_sitter_c_sharp::LANGUAGE.into(),
            Language::CSharp,
            CSHARP_QUERY,
            &m.branches,
            &m.smell_weights,
        )
    }
}

impl Parser for CSharpParser {
    fn parse(&self, path: &Path, source: &str) -> Result<(Module, Vec<Diagnostic>)> {
        self.engine()?.parse(path, source)
    }
}

#[cfg(test)]
mod type_captures_tests {
    use super::*;
    use crate::model::{Span, TypeKind};
    use std::path::Path;

    fn engine() -> QueryEngine {
        CSharpParser.engine().unwrap()
    }

    // Reusable harness pattern (tasks 6.3): the language's real metrics.scm
    // parses a fixture file; assert TypeDefinition count/kind/ordered fields
    // and that CodeUnits stay separate.
    #[test]
    fn csharp_types_fixture_round_trip() {
        let source = include_str!("../tests/fixtures/csharp_types_sample.cs");
        let (module, diags) = engine()
            .parse(Path::new("csharp_types_sample.cs"), source)
            .unwrap();

        assert!(diags.is_empty(), "fixture must parse cleanly: {diags:?}");

        assert_eq!(
            module.type_definitions.len(),
            2,
            "both classes captured; methods are not type definitions"
        );

        let person = module
            .type_definitions
            .iter()
            .find(|t| t.name == "Person")
            .expect("Person captured");
        assert_eq!(person.kind, TypeKind::Class);
        assert_eq!(person.language, Language::CSharp);
        assert_eq!(person.span, Span { start_line: 3, end_line: 13 });
        let person_fields: Vec<(&str, Option<&str>)> = person
            .fields
            .iter()
            .map(|f| (f.name.as_str(), f.type_name.as_deref()))
            .collect();
        assert_eq!(
            person_fields,
            vec![
                ("Name", Some("string")),
                ("Age", Some("int")),
                ("Email", Some("string")),
            ],
            "named fields only, in declaration order"
        );

        let boxed = module
            .type_definitions
            .iter()
            .find(|t| t.name == "Box")
            .expect("generic class captured");
        assert_eq!(boxed.kind, TypeKind::Class);
        assert!(
            boxed
                .name
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_'),
            "captured name must be bare identifier, no grammar syntax: {:?}",
            boxed.name
        );
        let box_fields: Vec<(&str, Option<&str>)> = boxed
            .fields
            .iter()
            .map(|f| (f.name.as_str(), f.type_name.as_deref()))
            .collect();
        assert_eq!(
            box_fields,
            vec![("Value", Some("T"))],
            "type parameter syntax must not leak into captured values"
        );

        // CodeUnits remain separate from type members.
        assert_eq!(module.units.len(), 2);
        let mut unit_names: Vec<&str> = module.units.iter().map(|u| u.name.as_str()).collect();
        unit_names.sort_unstable();
        assert_eq!(unit_names, vec!["Get", "Greet"]);
        assert!(
            person.fields.iter().all(|f| f.name != "Greet"),
            "methods must not be duplicated as fields"
        );
    }

    #[test]
    fn csharp_record_maps_to_record_kind() {
        // The pinned tree-sitter-c-sharp grammar (0.23.5) exposes
        // `record_declaration` as the only node for record declarations
        // (there is no record_struct_declaration); record components arrive
        // as a parameter_list of parameter nodes. The node kind contains
        // "record", so determine_type_kind maps it to TypeKind::Record.
        let source = "public record Point(int X, string Y) {\n    public int Get() {\n        return X;\n    }\n}\n";
        let (module, diags) = engine().parse(Path::new("Point.cs"), source).unwrap();

        assert!(diags.is_empty(), "record must parse cleanly: {diags:?}");
        assert_eq!(module.type_definitions.len(), 1);
        let point = &module.type_definitions[0];
        assert_eq!(point.name, "Point");
        assert_eq!(point.kind, TypeKind::Record);
        let fields: Vec<(&str, Option<&str>)> = point
            .fields
            .iter()
            .map(|f| (f.name.as_str(), f.type_name.as_deref()))
            .collect();
        assert_eq!(
            fields,
            vec![("X", Some("int")), ("Y", Some("string"))],
            "record components captured as ordered fields"
        );
        assert_eq!(module.units.len(), 1);
        assert_eq!(module.units[0].name, "Get");
    }
}
