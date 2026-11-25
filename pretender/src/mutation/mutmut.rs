//! mutmut report parsing.
//!
//! Purpose: turn `mutmut run` summary output and `mutmut results` blocks
//! into a `MutationReport`.
//! Responsibilities: parse the progress line, collect per-block surviving
//! test names, pad to the survived count.
//! Rationale: one module per external tool; block collection and survivor
//! numbering stay byte-identical to the pre-split parser.

use super::{compute_score, MutationReport, SurvivingMutant};

/// Parse the final progress line from `mutmut run`; returns (total, survived).
/// Line format: "N/M  🎉 K  ⏰ T  🤔 U  🙁 S"
pub(crate) fn parse_mutmut_summary(text: &str) -> (u32, u32) {
    for line in text.lines() {
        if let Some((total, survived)) = try_parse_mutmut_line(line) {
            return (total, survived);
        }
    }
    (0, 0)
}

fn try_parse_mutmut_line(line: &str) -> Option<(u32, u32)> {
    let fraction = line.split_whitespace().find(|t| t.contains('/'))?;
    let mut parts = fraction.split('/');
    let _current: u32 = parts.next()?.parse().ok()?;
    let total: u32 = parts.next()?.parse().ok()?;
    if total == 0 {
        return None;
    }
    let survived = survived_after_skull(line).unwrap_or(0);
    Some((total, survived))
}

/// The number after the 🙁 (U+1F641) emoji, if the line carries one.
fn survived_after_skull(line: &str) -> Option<u32> {
    let skull = "\u{1F641}";
    let pos = line.find(skull)?;
    let after = &line[pos + skull.len()..];
    after.split_whitespace().next()?.parse().ok()
}

/// Build a MutationReport from mutmut outputs.
/// `summary_text`: stdout from `mutmut run` (contains progress line).
/// `results_text`: stdout from `mutmut results` (contains surviving test names).
pub(crate) fn build_mutmut_report(summary_text: &str, results_text: &str) -> MutationReport {
    let (total, survived_count) = parse_mutmut_summary(summary_text);
    let killed = total.saturating_sub(survived_count);
    let score_pct = compute_score(total, survived_count);
    let survivors = parse_mutmut_surviving_tests(results_text, survived_count);

    MutationReport {
        tool: "mutmut".to_string(),
        total,
        killed,
        survived: survived_count,
        score_pct,
        survivors,
    }
}

/// `mutmut results` blocks as survivors, padded to `survived_count` with unknowns.
fn parse_mutmut_surviving_tests(text: &str, survived_count: u32) -> Vec<SurvivingMutant> {
    let mut survivors = parse_mutmut_blocks(text);
    while survivors.len() < survived_count as usize {
        survivors.push(mutmut_survivor(survivors.len() + 1, vec![]));
    }
    survivors
}

/// One `SurvivingMutant` for mutmut block `index` (1-based) with `tests`.
fn mutmut_survivor(index: usize, tests: Vec<String>) -> SurvivingMutant {
    SurvivingMutant {
        file: String::new(),
        line: 0,
        description: format!("mutant {index} survived"),
        missed_tests: tests,
    }
}

/// Is this line a "---- N ----" block header?
fn is_mutmut_block_header(trimmed: &str) -> bool {
    trimmed.starts_with("----") && trimmed.ends_with("----") && trimmed.len() > 8
}

/// A line inside a block contributes a test name unless it is blank or
/// mutmut's trailing noise ("To ..." hint / "mutmut" footer).
fn collects_mutmut_test_line(trimmed: &str, in_block: bool) -> bool {
    in_block && !trimmed.is_empty() && !trimmed.starts_with("To ") && !trimmed.starts_with("mutmut")
}

/// Flush pending test names as one survivor when the block held any.
fn flush_mutmut_block(current_tests: &mut Vec<String>, survivors: &mut Vec<SurvivingMutant>) {
    if !current_tests.is_empty() {
        let tests = std::mem::take(current_tests);
        survivors.push(mutmut_survivor(survivors.len() + 1, tests));
    }
}

/// Collect one survivor per "---- N ----" block of test names.
fn parse_mutmut_blocks(text: &str) -> Vec<SurvivingMutant> {
    let mut survivors = Vec::new();
    let mut current_tests: Vec<String> = Vec::new();
    let mut in_block = false;

    for line in text.lines() {
        let trimmed = line.trim();
        if is_mutmut_block_header(trimmed) {
            flush_mutmut_block(&mut current_tests, &mut survivors);
            in_block = true;
            current_tests.clear();
        } else if collects_mutmut_test_line(trimmed, in_block) {
            current_tests.push(trimmed.to_string());
        }
    }
    flush_mutmut_block(&mut current_tests, &mut survivors);
    survivors
}
