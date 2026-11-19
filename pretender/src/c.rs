use anyhow::{anyhow, Result};
use std::path::Path;
use std::sync::OnceLock;

use crate::engine::QueryEngine;
use crate::model::{Diagnostic, Language, Module, Parser};
use crate::plugin::PluginManifest;

const C_QUERY: &str = include_str!("../languages/c/metrics.scm");
const C_MANIFEST: &str = include_str!("../languages/c/plugin.toml");

pub struct CParser;

fn manifest() -> Result<&'static PluginManifest> {
    static MANIFEST: OnceLock<std::result::Result<PluginManifest, String>> = OnceLock::new();
    match MANIFEST
        .get_or_init(|| PluginManifest::from_toml(C_MANIFEST).map_err(|err| err.to_string()))
    {
        Ok(manifest) => Ok(manifest),
        Err(err) => Err(anyhow!(err.clone())),
    }
}

impl CParser {
    fn engine(&self) -> Result<QueryEngine> {
        let m = manifest()?;
        QueryEngine::new_with_branch_weights(
            tree_sitter_c::LANGUAGE.into(),
            Language::C,
            C_QUERY,
            &m.branches,
            &m.smell_weights,
        )
    }
}

impl Parser for CParser {
    fn parse(&self, path: &Path, source: &str) -> Result<(Module, Vec<Diagnostic>)> {
        self.engine()?.parse(path, source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::TypeKind;

    const C_TYPES_SAMPLE: &str = include_str!("../tests/fixtures/c_types_sample.c");

    #[test]
    fn c_types_sample_round_trip() {
        let parser = CParser;
        let (module, diagnostics) = parser
            .parse(Path::new("tests/fixtures/c_types_sample.c"), C_TYPES_SAMPLE)
            .unwrap();
        assert!(diagnostics.is_empty(), "unexpected: {diagnostics:?}");

        // Structs are captured; unions are not (design D3).
        assert_eq!(
            module.type_definitions.len(),
            1,
            "the union must not yield a TypeDefinition"
        );
        let def = &module.type_definitions[0];
        assert_eq!(def.name, "Person");
        assert_eq!(def.kind, TypeKind::Struct);
        assert_eq!(def.language, Language::C);

        let names: Vec<&str> = def.fields.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["name", "age", "height"]);
        let types: Vec<Option<&str>> = def.fields.iter().map(|f| f.type_name.as_deref()).collect();
        assert_eq!(types, [Some("char"), Some("int"), Some("double")]);

        // The free function stays a CodeUnit and is not duplicated as a field.
        assert!(module.units.iter().any(|u| u.name == "person_age"));
        assert!(def.fields.iter().all(|f| f.name != "person_age"));
    }
}
