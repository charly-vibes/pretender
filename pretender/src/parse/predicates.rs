//! Query-predicate re-validation (pretender-krou): text predicates
//! extracted from the query source and evaluated against matches.
//!
//! Purpose: mirror the binding's private `satisfies_text_predicates` so
//! junk matches (which can replay stale capture buffers carrying
//! `@function.definition` and produce phantom code units) are rejected
//! before unit building.
//!
//! # Responsibilities
//! `PredicateCheck` evaluation (`holds`), per-pattern predicate
//! extraction (`PredicateChecks::extract` with slice merging), and the
//! s-expression tokenizer feeding extraction.
//!
//! # Rationale
//! tree-sitter-clojure ships an ABI-14 grammar whose query matches under
//! tree-sitter 0.25 are not reliably suppressed when a predicate fails —
//! the cursor yields junk matches. The binding keeps its native text
//! predicates private (`general_predicates` only carries unknown
//! operators), so the predicates are re-extracted from the query source
//! and evaluated here with the same semantics as the binding's
//! `satisfies_text_predicates`. Split out of `parse` so the module
//! boundary follows the responsibility.

/// One re-validated query predicate (pretender-krou): the
/// grammar-independent mirror of the binding's private
/// `satisfies_text_predicates`.
pub(super) enum PredicateCheck {
    /// `#match?` / `#not-match?`: every captured node's text must (not)
    /// match the regex.
    MatchRegex {
        capture_idx: u32,
        regex: regex::Regex,
        positive: bool,
    },
    /// `#eq?` / `#not-eq?` against a literal string.
    EqString {
        capture_idx: u32,
        text: String,
        positive: bool,
    },
    /// `#any-of?` / `#not-any-of?`: membership in a literal set.
    AnyOf {
        capture_idx: u32,
        options: Vec<String>,
        positive: bool,
    },
}

impl PredicateCheck {
    fn holds(&self, m: &tree_sitter::QueryMatch<'_, '_>, source_bytes: &[u8]) -> bool {
        let (any_satisfied, all_satisfied) = self.satisfaction(m, source_bytes);
        match self.positive() {
            true => all_satisfied,
            false => !any_satisfied,
        }
    }

    /// Satisfaction of this check over the match's captures of its
    /// capture index. Mirrors the binding: predicates only constrain
    /// nodes actually captured by the match; a missing capture vacuously
    /// passes. Unreadable capture text counts as unsatisfied.
    fn satisfaction(
        &self,
        m: &tree_sitter::QueryMatch<'_, '_>,
        source_bytes: &[u8],
    ) -> (bool, bool) {
        let satisfied: Vec<bool> = m
            .captures
            .iter()
            .filter(|c| c.index == self.capture_idx())
            .map(|capture| {
                capture
                    .node
                    .utf8_text(source_bytes)
                    .is_ok_and(|text| self.satisfies(text))
            })
            .collect();
        let any_satisfied = satisfied.iter().any(|satisfied| *satisfied);
        let all_satisfied = satisfied.iter().all(|satisfied| *satisfied);
        (any_satisfied, all_satisfied)
    }

    fn satisfies(&self, text: &str) -> bool {
        match self {
            Self::MatchRegex { regex, .. } => regex.is_match(text),
            Self::EqString { text: expected, .. } => text == expected,
            Self::AnyOf { options, .. } => options.iter().any(|option| option == text),
        }
    }

    fn capture_idx(&self) -> u32 {
        match self {
            Self::MatchRegex { capture_idx, .. }
            | Self::EqString { capture_idx, .. }
            | Self::AnyOf { capture_idx, .. } => *capture_idx,
        }
    }

    fn positive(&self) -> bool {
        match self {
            Self::MatchRegex { positive, .. }
            | Self::EqString { positive, .. }
            | Self::AnyOf { positive, .. } => *positive,
        }
    }
}

/// Re-validated text predicates for every query pattern
/// (pretender-krou).
///
/// tree-sitter delimits each trailing predicate line as its own
/// pattern slice, so those checks are merged into the preceding real
/// pattern; a match reported under a predicate-only index is junk by
/// construction and is rejected outright.
pub(super) struct PredicateChecks {
    per_pattern: Vec<Vec<PredicateCheck>>,
    predicate_only: Vec<bool>,
}

impl PredicateChecks {
    /// Extract the text predicates of every pattern from the query
    /// source. Unknown operators are ignored — tree-sitter keeps owning
    /// those.
    pub(super) fn extract(query: &tree_sitter::Query, query_source: &str) -> Self {
        super::extract::extract(query, query_source)
    }

    /// Assembled by `super::extract` from the per-pattern check lists.
    pub(super) fn new(per_pattern: Vec<Vec<PredicateCheck>>, predicate_only: Vec<bool>) -> Self {
        Self {
            per_pattern,
            predicate_only,
        }
    }

    /// True when the match satisfies every predicate of its pattern.
    pub(super) fn pass(&self, m: &tree_sitter::QueryMatch<'_, '_>, source_bytes: &[u8]) -> bool {
        let Some(index) = self.per_pattern.get(m.pattern_index) else {
            // A pattern index beyond the query is cursor junk.
            return false;
        };
        if self.predicate_only[m.pattern_index] {
            return false;
        }
        index.iter().all(|check| check.holds(m, source_bytes))
    }
}
