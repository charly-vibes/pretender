use anyhow::{anyhow, Result};
use std::path::Path;
use std::sync::OnceLock;

use crate::engine::QueryEngine;
use crate::model::{Diagnostic, Language, Module, Parser};
use crate::plugin::PluginManifest;

const CPP_QUERY: &str = include_str!("../languages/cpp/metrics.scm");
const CPP_MANIFEST: &str = include_str!("../languages/cpp/plugin.toml");

pub struct CppParser;

fn manifest() -> Result<&'static PluginManifest> {
    static MANIFEST: OnceLock<std::result::Result<PluginManifest, String>> = OnceLock::new();
    match MANIFEST
        .get_or_init(|| PluginManifest::from_toml(CPP_MANIFEST).map_err(|err| err.to_string()))
    {
        Ok(manifest) => Ok(manifest),
        Err(err) => Err(anyhow!(err.clone())),
    }
}

impl CppParser {
    fn engine(&self) -> Result<QueryEngine> {
        let m = manifest()?;
        QueryEngine::new_with_branch_weights(
            tree_sitter_cpp::LANGUAGE.into(),
            Language::Cpp,
            CPP_QUERY,
            &m.branches,
            &m.smell_weights,
        )
    }
}

impl Parser for CppParser {
    fn parse(&self, path: &Path, source: &str) -> Result<(Module, Vec<Diagnostic>)> {
        self.engine()?.parse(path, source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::TypeKind;

    const CPP_TYPES_SAMPLE: &str = include_str!("../tests/fixtures/cpp_types_sample.cpp");

    #[test]
    fn cpp_types_sample_round_trip() {
        let parser = CppParser;
        let (module, diagnostics) = parser
            .parse(
                Path::new("tests/fixtures/cpp_types_sample.cpp"),
                CPP_TYPES_SAMPLE,
            )
            .unwrap();
        assert!(diagnostics.is_empty(), "unexpected: {diagnostics:?}");

        assert_eq!(module.type_definitions.len(), 2);
        let widget = module
            .type_definitions
            .iter()
            .find(|d| d.name == "Widget")
            .expect("class captured");
        // class_specifier contains neither "record" nor "struct" → Class.
        assert_eq!(widget.kind, TypeKind::Class);
        assert_eq!(widget.language, Language::Cpp);

        let names: Vec<&str> = widget.fields.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["width", "title", "visible"]);
        let types: Vec<Option<&str>> = widget.fields.iter().map(|f| f.type_name.as_deref()).collect();
        assert_eq!(types, [Some("int"), Some("char"), Some("bool")]);

        let point = module
            .type_definitions
            .iter()
            .find(|d| d.name == "Point")
            .expect("struct captured");
        assert_eq!(point.kind, TypeKind::Struct);
        let point_names: Vec<&str> = point.fields.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(point_names, ["x", "y"]);

        // The method stays a CodeUnit and is not duplicated as a field.
        assert!(module.units.iter().any(|u| u.name == "area"));
        assert!(widget.fields.iter().all(|f| f.name != "area"));
    }
}
