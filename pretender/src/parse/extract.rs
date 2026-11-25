//! Predicate extraction (pretender-krou): reads the query source text
//! and builds the per-pattern [`PredicateChecks`] that `predicates`
//! evaluates against matches.
//!
//! # Responsibilities
//! Pattern slicing (`pattern_slice`, including the merge of trailing
//! predicate-only lines into the preceding real pattern), s-expression
//! tokenizing (`sexp_tokens`), and per-operator check construction
//! (`extract_pattern_checks`).
//!
//! # Rationale
//! tree-sitter keeps its native text predicates private
//! (`general_predicates` only carries unknown operators), so the checks
//! are re-extracted from the query source with the same semantics as the
//! binding's `satisfies_text_predicates`; unknown operators are ignored —
//! tree-sitter keeps owning those. Split from `predicates` so the
//! extraction seam matches the evaluation seam.

use super::predicates::PredicateCheck;
use super::predicates::PredicateChecks;

/// Extract the text predicates of every pattern from the query source.
pub(super) fn extract(query: &tree_sitter::Query, query_source: &str) -> PredicateChecks {
    let count = query.pattern_count();
    let mut per_pattern: Vec<Vec<PredicateCheck>> = (0..count).map(|_| Vec::new()).collect();
    let mut predicate_only = vec![false; count];
    let mut scan = PatternScan {
        per_pattern: &mut per_pattern,
        predicate_only: &mut predicate_only,
        last_real: None,
    };
    for pattern in 0..count {
        if let Some(slice) = pattern_slice(query, pattern, query_source) {
            let last = scan.absorb(slice, query, pattern);
            scan.last_real = last;
        }
    }
    PredicateChecks::new(per_pattern, predicate_only)
}

/// The query source slice of one pattern; empty slices (`start >= end`)
/// are skipped.
fn pattern_slice<'q>(
    query: &tree_sitter::Query,
    pattern: usize,
    query_source: &'q str,
) -> Option<&'q str> {
    let start = query.start_byte_for_pattern(pattern);
    let end = query.end_byte_for_pattern(pattern).min(query_source.len());
    (start < end).then(|| &query_source[start..end])
}

/// Scan state for merging per-pattern predicate slices.
struct PatternScan<'a> {
    per_pattern: &'a mut [Vec<PredicateCheck>],
    predicate_only: &'a mut [bool],
    last_real: Option<usize>,
}

impl PatternScan<'_> {
    /// Merge one pattern slice's predicates: predicate-only lines extend
    /// the preceding real pattern, otherwise the pattern carries its own
    /// checks. Returns the new last-real-pattern marker.
    fn absorb(&mut self, slice: &str, query: &tree_sitter::Query, pattern: usize) -> Option<usize> {
        let extracted = extract_pattern_checks(query, slice);
        if first_meaningful_token_is_predicate(slice) {
            self.predicate_only[pattern] = true;
            return self
                .last_real
                .inspect(|&prev| self.per_pattern[prev].extend(extracted));
        }
        self.per_pattern[pattern].extend(extracted);
        Some(pattern)
    }
}

/// True when the slice opens with a predicate s-expression (`(#match?`…)
/// rather than a node pattern.
fn first_meaningful_token_is_predicate(slice: &str) -> bool {
    slice
        .trim_start()
        .strip_prefix('(')
        .is_some_and(|rest| rest.trim_start().starts_with('#'))
}

/// Extract the text predicates of one pattern slice.
///
/// Predicate s-expressions never nest, so every open paren is an
/// independent candidate; matches the sequential scan's outcomes.
fn extract_pattern_checks(query: &tree_sitter::Query, slice: &str) -> Vec<PredicateCheck> {
    let tokens = sexp_tokens(slice);
    let candidates = tokens
        .iter()
        .enumerate()
        .filter(|(i, token)| token.as_str() == "(" && i + 1 < tokens.len());
    candidates
        .filter_map(|(i, _)| {
            let (operator, args, _end) = read_predicate(&tokens, i);
            build_check(query, &operator, &args)
        })
        .collect()
}

/// Read one `(#operator args…)` s-expression starting at token `i`,
/// classifying each argument as a capture reference or a string literal.
/// Returns the operator (leading `#` stripped) and the index after the
/// closing paren (or one past the token list if unterminated).
fn read_predicate(tokens: &[String], i: usize) -> (String, Vec<StringArg>, usize) {
    let operator = tokens[i + 1].trim_start_matches('#').to_string();
    let end = tokens[i + 2..]
        .iter()
        .position(|token| token == ")")
        .map_or(tokens.len(), |offset| i + 2 + offset);
    let args = tokens[i + 2..end]
        .iter()
        .map(|token| StringArg::parse(token))
        .collect();
    (operator, args, end + 1)
}

/// Build the check for one known operator, resolving the leading capture
/// argument to its query capture index. Unknown operators yield `None`.
fn build_check(
    query: &tree_sitter::Query,
    operator: &str,
    args: &[StringArg],
) -> Option<PredicateCheck> {
    if !matches!(operator, "match?" | "eq?" | "any-of?") {
        return None;
    }
    let capture_name = args.first()?.as_capture()?.strip_prefix('@')?;
    let capture_idx = query.capture_index_for_name(capture_name)?;
    let positive = !operator.starts_with("not-");
    match operator.trim_start_matches("not-") {
        "match?" => match_check(args.get(1), capture_idx, positive),
        "eq?" => eq_check(args.get(1), capture_idx, positive),
        _ => any_of_check(&args[1..], capture_idx, positive),
    }
}

/// `#match?` / `#not-match?`: the second argument compiles to a regex.
fn match_check(
    arg: Option<&StringArg>,
    capture_idx: u32,
    positive: bool,
) -> Option<PredicateCheck> {
    arg.and_then(|arg| arg.as_regex())
        .map(|regex| PredicateCheck::MatchRegex {
            capture_idx,
            regex,
            positive,
        })
}

/// `#eq?` / `#not-eq?`: the second argument is a literal string.
fn eq_check(arg: Option<&StringArg>, capture_idx: u32, positive: bool) -> Option<PredicateCheck> {
    arg.and_then(|arg| arg.as_string())
        .map(|text| PredicateCheck::EqString {
            capture_idx,
            text: text.to_string(),
            positive,
        })
}

/// `#any-of?` / `#not-any-of?`: the remaining arguments are the literal
/// option set.
fn any_of_check(args: &[StringArg], capture_idx: u32, positive: bool) -> Option<PredicateCheck> {
    let options = args
        .iter()
        .filter_map(|arg| arg.as_string().map(str::to_string))
        .collect();
    Some(PredicateCheck::AnyOf {
        capture_idx,
        options,
        positive,
    })
}

/// One token of a predicate s-expression: a capture reference or a
/// string literal (escapes resolved).
enum StringArg {
    Capture(String),
    Str(String),
}

impl StringArg {
    fn parse(token: &str) -> Self {
        if token.starts_with('@') {
            Self::Capture(token.to_string())
        } else {
            Self::Str(token.to_string())
        }
    }

    fn as_string(&self) -> Option<&str> {
        match self {
            Self::Capture(_) => None,
            Self::Str(text) => Some(text),
        }
    }

    fn as_capture(&self) -> Option<&str> {
        match self {
            Self::Capture(name) => Some(name),
            Self::Str(_) => None,
        }
    }

    fn as_regex(&self) -> Option<regex::Regex> {
        self.as_string()
            .and_then(|text| regex::Regex::new(text).ok())
    }
}

/// Split a query source slice into `(`/`)` and atom tokens, keeping
/// quoted strings (with escapes resolved) as single atoms.
fn sexp_tokens(slice: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut chars = slice.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '(' | ')' => tokens.push(ch.to_string()),
            '"' => tokens.push(read_string(&mut chars)),
            ';' => {
                // Comment to end of line.
                chars.find(|c| *c == '\n');
            }
            other if other.is_whitespace() => {}
            other => tokens.push(read_atom(&mut chars, other)),
        }
    }
    tokens
}

/// Read a quoted string with escapes resolved; the opening `"` is
/// already consumed.
fn read_string(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> String {
    let mut token = String::new();
    while let Some(c) = chars.next() {
        match c {
            '"' => break,
            '\\' => {
                if let Some(escaped) = chars.next() {
                    token.push(match escaped {
                        'n' => '\n',
                        't' => '\t',
                        'r' => '\r',
                        other => other,
                    });
                }
            }
            other => token.push(other),
        }
    }
    token
}

/// Read a bare atom up to whitespace, paren, or quote; `first` is the
/// already-consumed initial character.
fn read_atom(chars: &mut std::iter::Peekable<std::str::Chars<'_>>, first: char) -> String {
    std::iter::once(first)
        .chain(std::iter::from_fn(|| {
            let next = *chars.peek()?;
            if next.is_whitespace() || next == '(' || next == ')' || next == '"' {
                return None;
            }
            chars.next()
        }))
        .collect()
}
