//! pretender library target: all analysis modules in one place so that
//! integration tests in `tests/` can exercise parsers and the engine through
//! the public API (`use pretender::<module>::…`).
//!
//! Purpose: hold the module tree previously declared in `main.rs`, keeping the
//! binary a thin CLI shell over this crate.
//! Responsibilities: declare every analysis module `pub`; own no logic.
//! Rationale: `tests/*_roundtrip_test.rs` (epic pretender-5zo) need crate
//! imports, which a bin-only target cannot provide. Splitting the target
//! changes no behavior: the same files compile into the same binary.

pub mod c;
pub mod clojure;
pub mod cluster_detector;
pub mod config;
pub mod coupling;
pub mod cpp;
pub mod csharp;
pub mod data_model;
pub mod doctor;
pub mod duplication;
pub mod engine;
pub mod error_metrics;
pub mod explain;
pub mod external_plugin;
pub mod git;
pub mod go;
pub mod history;
pub mod inheritance_depth;
pub mod java;
pub mod javascript;
pub mod julia;
pub(crate) mod line_counts;
pub(crate) mod metric_application;
pub mod metrics;
pub mod mock_detector;
pub mod model;
pub mod mutability_metrics;
pub mod mutation;
pub(crate) mod mutation_entry;
pub(crate) mod parse;
pub mod plugin;
pub mod primitive_obsession;
pub mod python;
pub mod r;
pub(crate) mod role_resolution;
pub mod roles;
pub mod ruby;
pub mod rust;
pub mod test_report;
pub(crate) mod type_definitions;
pub mod typescript;
