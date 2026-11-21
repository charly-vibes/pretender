use anyhow::{anyhow, Result};
use std::path::Path;
use std::sync::OnceLock;

use crate::engine::QueryEngine;
use crate::model::{Diagnostic, Language, Module, Parser};
use crate::plugin::PluginManifest;

const PYTHON_QUERY: &str = include_str!("../languages/python/metrics.scm");
const PYTHON_MANIFEST: &str = include_str!("../languages/python/plugin.toml");

pub struct PythonParser;

fn manifest() -> Result<&'static PluginManifest> {
    static MANIFEST: OnceLock<std::result::Result<PluginManifest, String>> = OnceLock::new();
    match MANIFEST
        .get_or_init(|| PluginManifest::from_toml(PYTHON_MANIFEST).map_err(|err| err.to_string()))
    {
        Ok(manifest) => Ok(manifest),
        Err(err) => Err(anyhow!(err.clone())),
    }
}

impl PythonParser {
    fn engine(&self) -> Result<QueryEngine> {
        let m = manifest()?;
        QueryEngine::new_with_branch_weights(
            tree_sitter_python::LANGUAGE.into(),
            Language::Python,
            PYTHON_QUERY,
            &m.branches,
            &m.smell_weights,
        )
    }
}

impl Parser for PythonParser {
    fn parse(&self, path: &Path, source: &str) -> Result<(Module, Vec<Diagnostic>)> {
        self.engine()?.parse(path, source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Span, TypeKind, UnitKind};

    #[test]
    fn python_types_fixture_round_trip() {
        // Reusable harness pattern (tasks 6.2): the language's real
        // metrics.scm parses a fixture file; assert TypeDefinition
        // count/kind/ordered fields and that CodeUnits stay separate.
        let parser = PythonParser;
        let source = include_str!("../tests/fixtures/python_types_sample.py");
        let (module, diags) = parser
            .parse(Path::new("python_types_sample.py"), source)
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
        // Design D3: @dataclass ships as Class — no decorator refinement.
        assert_eq!(person.kind, TypeKind::Class);
        assert_eq!(person.language, Language::Python);
        assert_eq!(
            person.span,
            Span {
                start_line: 7,
                end_line: 16
            }
        );
        assert!(
            person.name.chars().all(|c| c.is_alphanumeric() || c == '_'),
            "captured name must be bare identifier, no grammar syntax: {:?}",
            person.name
        );
        let person_fields: Vec<(&str, Option<&str>)> = person
            .fields
            .iter()
            .map(|f| (f.name.as_str(), f.type_name.as_deref()))
            .collect();
        assert_eq!(
            person_fields,
            vec![
                ("name", Some("str")),
                ("age", Some("int")),
                ("email", Some("str")),
            ],
            "annotated attributes only, in declaration order"
        );
        assert!(
            person.fields.iter().all(|f| f.name != "greeting"),
            "method-local annotated variables must not become fields"
        );

        let account = module
            .type_definitions
            .iter()
            .find(|t| t.name == "Account")
            .expect("Account captured");
        assert_eq!(account.kind, TypeKind::Class);
        assert_eq!(
            account.span,
            Span {
                start_line: 19,
                end_line: 27
            }
        );
        let account_fields: Vec<(&str, Option<&str>)> = account
            .fields
            .iter()
            .map(|f| (f.name.as_str(), f.type_name.as_deref()))
            .collect();
        assert_eq!(
            account_fields,
            vec![("balance", Some("float")), ("owner", Some("str"))],
            "__init__ self.x assignments must not be duplicated as fields"
        );

        // CodeUnits remain separate from type members.
        let mut unit_names: Vec<&str> = module.units.iter().map(|u| u.name.as_str()).collect();
        unit_names.sort_unstable();
        assert_eq!(unit_names, vec!["__init__", "greet"]);
        let greet = module.units.iter().find(|u| u.name == "greet").unwrap();
        assert_eq!(greet.kind, UnitKind::Method);
        let ctor = module.units.iter().find(|u| u.name == "__init__").unwrap();
        assert_eq!(ctor.kind, UnitKind::Ctor);
    }
}
