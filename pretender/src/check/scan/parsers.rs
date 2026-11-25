//! Parser registry: file-extension → parser dispatch for the check scan.
//!
//! # Responsibilities
//! Resolve the parser for a path; the accepted extension set is the single
//! source both the collector's `is_supported_extension` (in
//! `super::super::collect`) and this registry honor.
//!
//! # Rationale
//! Split from the scan module (pretender-5zo.15.12). Since pretender-948q
//! the dispatch itself is data: `lookup` resolves the extension against the
//! declarative language registry and hands back the one `GenericParser`,
//! so adding a language no longer edits a `match` here.

use anyhow::{anyhow, Result};
use pretender::model;
use pretender::registry::{self, GenericParser};
use std::path::Path;

/// Resolve the parser for `path` based on its file extension.
pub(crate) fn get_parser(path: &Path) -> Result<Box<dyn model::Parser>> {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("missing file extension for path: {}", path.display()))?;
    parser_for_extension(ext).ok_or_else(|| anyhow!("unsupported file extension '.{}'", ext))
}

/// Parser lookup, backed by the declarative language registry.
fn parser_for_extension(ext: &str) -> Option<Box<dyn model::Parser>> {
    registry::lookup(ext)
        .map(|record| Box::new(GenericParser { record }) as Box<dyn model::Parser>)
}
