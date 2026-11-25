//! Parser registry: file-extension → parser dispatch for the check scan.
//!
//! # Responsibilities
//! Resolve the parser for a path; the accepted extension set is the single
//! source both the collector's `is_supported_extension` (in
//! `super::super::collect`) and this registry honor.
//!
//! # Rationale
//! Split from the scan module (pretender-5zo.15.12); bodies moved verbatim
//! with the extension match split in two only to stay within the
//! function-size limit.
use anyhow::{anyhow, Result};
use pretender::model;
use pretender::{
    c, clojure, cpp, csharp, go, java, javascript, julia, python, r, ruby, rust, typescript,
};
use std::path::Path;

/// Resolve the parser for `path` based on its file extension.
pub(crate) fn get_parser(path: &Path) -> Result<Box<dyn model::Parser>> {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("missing file extension for path: {}", path.display()))?;
    parser_for_extension(ext).ok_or_else(|| anyhow!("unsupported file extension '.{}'", ext))
}

/// Parser lookup for the first half of the supported extensions.
fn parser_for_extension(ext: &str) -> Option<Box<dyn model::Parser>> {
    match ext {
        "cs" => Some(Box::new(csharp::CSharpParser)),
        "clj" | "cljs" | "cljc" | "edn" => Some(Box::new(clojure::ClojureParser)),
        "c" | "h" => Some(Box::new(c::CParser)),
        "cpp" | "cc" | "cxx" | "hpp" | "hxx" => Some(Box::new(cpp::CppParser)),
        "go" => Some(Box::new(go::GoParser)),
        "java" => Some(Box::new(java::JavaParser)),
        "jl" => Some(Box::new(julia::JuliaParser)),
        "js" | "jsx" | "mjs" | "cjs" => Some(Box::new(javascript::JavaScriptParser)),
        _ => parser_for_extension_rest(ext),
    }
}

/// Parser lookup for the remaining supported extensions.
fn parser_for_extension_rest(ext: &str) -> Option<Box<dyn model::Parser>> {
    match ext {
        "py" => Some(Box::new(python::PythonParser)),
        "r" | "R" => Some(Box::new(r::RParser)),
        "rb" => Some(Box::new(ruby::RubyParser)),
        "rs" => Some(Box::new(rust::RustParser)),
        "ts" | "mts" => Some(Box::new(typescript::TypeScriptParser)),
        "tsx" | "cts" => Some(Box::new(typescript::TypeScriptXParser)),
        _ => None,
    }
}
