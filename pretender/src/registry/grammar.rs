//! Statically linked grammar binding for the language registry.
//!
//! # Responsibilities
//! Map each registry grammar variant to its compiled-in `tree_sitter`
//! language.
//!
//! # Rationale
//! The registry's one inherently non-data bit (design constraint 3 of
//! pretender-948q): grammars are statically linked, so extension means one
//! exhaustive match arm — never a rewrite. Split from `registry` so the
//! records table and the grammar binding each stay under the structural
//! file cap.

/// Grammar variants, one per distinct `tree_sitter` language binding.
/// TS and TSX share a crate; every other language maps 1:1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Grammar {
    C,
    Clojure,
    Cpp,
    CSharp,
    Go,
    Java,
    JavaScript,
    Julia,
    Python,
    R,
    Ruby,
    Rust,
    TypeScript,
    Tsx,
}

impl Grammar {
    pub(crate) fn language(self) -> tree_sitter::Language {
        match self {
            Grammar::C => tree_sitter_c::LANGUAGE.into(),
            Grammar::Clojure => tree_sitter_clojure::LANGUAGE.into(),
            Grammar::Cpp => tree_sitter_cpp::LANGUAGE.into(),
            Grammar::CSharp => tree_sitter_c_sharp::LANGUAGE.into(),
            Grammar::Go => tree_sitter_go::LANGUAGE.into(),
            Grammar::Java => tree_sitter_java::LANGUAGE.into(),
            Grammar::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
            Grammar::Julia => tree_sitter_julia::LANGUAGE.into(),
            Grammar::Python => tree_sitter_python::LANGUAGE.into(),
            Grammar::R => tree_sitter_r::LANGUAGE.into(),
            Grammar::Ruby => tree_sitter_ruby::LANGUAGE.into(),
            Grammar::Rust => tree_sitter_rust::LANGUAGE.into(),
            Grammar::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Grammar::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
        }
    }
}
