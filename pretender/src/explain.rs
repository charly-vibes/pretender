use anyhow::{anyhow, Result};

pub struct MetricDoc {
    pub name: &'static str,
    pub measures: &'static str,
    pub formula: &'static str,
    pub default_threshold: &'static str,
    pub citation: &'static str,
    pub improve: &'static str,
}

const CATALOG: &[MetricDoc] = &[
    MetricDoc {
        name: "cyclomatic",
        measures: "The number of independent execution paths through a function.",
        formula: "1 + decision points (if/elif/else-if/for/while/except/&&/||/ternary)",
        default_threshold: "10 (app), 8 (library exported), 3 (test)",
        citation: "McCabe (1976), \"A Complexity Measure\", IEEE TSE",
        improve: "Extract helper functions; replace nested conditionals with early returns or guard clauses.",
    },
    MetricDoc {
        name: "cognitive",
        measures: "The mental effort required to understand a function, penalising nesting and structural breaks in linear flow.",
        formula: "Sum of branch weights × (1 + nesting depth); logical sequences counted once",
        default_threshold: "15 (app), 5 (test)",
        citation: "Campbell (2018), \"Cognitive Complexity — A new way of measuring understandability\", SonarSource",
        improve: "Flatten nested conditionals; extract loops into named helpers; prefer early returns.",
    },
    MetricDoc {
        name: "abc",
        measures: "Composite size proxy: Assignments, Branches, and Calls weighted by smell.",
        formula: "√(A² + B² + C²) where A = assignments, B = branches, C = weighted call count",
        default_threshold: "30 (app)",
        citation: "Fitzpatrick (1997), \"Applying the ABC Metric to C, C++, and Java\"",
        improve: "Break large functions into smaller ones; reduce assignments and deep call chains.",
    },
    MetricDoc {
        name: "function_lines",
        measures: "Source lines spanned by a function or method, including signature and closing brace.",
        formula: "end_line − start_line + 1",
        default_threshold: "40 (app), 100 (script), 80 (test)",
        citation: "Clean Code (Martin, 2008) §3; SonarQube default 150 lines",
        improve: "Extract cohesive sub-routines; separate setup from logic; remove dead branches.",
    },
    MetricDoc {
        name: "file_lines",
        measures: "Total source lines in a file.",
        formula: "Line count of the file",
        default_threshold: "400 (app), 300 (script)",
        citation: "SonarQube default 750; Pretender uses a stricter 400 for application code.",
        improve: "Split large files by responsibility; move helper types or utility functions to separate modules.",
    },
    MetricDoc {
        name: "nesting_max",
        measures: "The maximum nesting depth of control-flow blocks inside a function.",
        formula: "Deepest block depth (0 = top-level statement inside the function body)",
        default_threshold: "3 (app/library), 2 (test)",
        citation: "Lanza & Marinescu (2006), \"Object-Oriented Metrics in Practice\"",
        improve: "Invert conditions to reduce nesting; extract inner blocks into helper functions.",
    },
    MetricDoc {
        name: "params",
        measures: "Number of declared parameters on a function or method.",
        formula: "Count of formal parameters",
        default_threshold: "4 (app), 3 (library exported), 2 (test)",
        citation: "Clean Code (Martin, 2008) §3 — \"Functions should have few arguments\"",
        improve: "Introduce a parameter object or builder; pass context structs instead of primitives.",
    },
    MetricDoc {
        name: "min_assertions",
        measures: "Minimum number of assertion calls required in a test function.",
        formula: "Count of assertion calls (assert_*, expect, assert!, etc.)",
        default_threshold: "1 (test role only)",
        citation: "xUnit patterns — every test must contain at least one verification.",
        improve: "Add at least one assert/expect call; split structural setup from behavioural assertions.",
    },
    MetricDoc {
        name: "exported_cyclomatic",
        measures: "Cyclomatic complexity of exported (public) functions in library code.",
        formula: "Same as cyclomatic, applied only to exported symbols",
        default_threshold: "8 (library role)",
        citation: "API design heuristic: public surfaces should be simpler than internals.",
        improve: "Delegate complexity to private helpers; keep the public API thin.",
    },
    MetricDoc {
        name: "exported_params",
        measures: "Number of parameters on exported functions in library code.",
        formula: "Count of formal parameters on exported symbols",
        default_threshold: "3 (library role)",
        citation: "API usability principle: callers should rarely need more than 3 arguments.",
        improve: "Use builder patterns or config structs for optional parameters.",
    },
    MetricDoc {
        name: "exported_lines",
        measures: "Line span of exported functions in library code.",
        formula: "Same as function_lines, applied only to exported symbols",
        default_threshold: "30 (library role)",
        citation: "API design: public functions should do one thing; longer ones hide complexity from callers.",
        improve: "Delegate to private helpers; split multi-phase exported functions.",
    },
];

pub fn lookup(name: &str) -> Option<&'static MetricDoc> {
    CATALOG.iter().find(|m| m.name == name)
}

pub fn all_names() -> Vec<&'static str> {
    CATALOG.iter().map(|m| m.name).collect()
}

pub fn print_doc(doc: &MetricDoc) -> Result<()> {
    let mut out = std::io::stdout().lock();
    use std::io::Write;
    writeln!(out, "{}", doc.name)?;
    writeln!(out, "  Measures:   {}", doc.measures)?;
    writeln!(out, "  Formula:    {}", doc.formula)?;
    writeln!(out, "  Threshold:  {}", doc.default_threshold)?;
    writeln!(out, "  Citation:   {}", doc.citation)?;
    writeln!(out, "  Improve:    {}", doc.improve)?;
    Ok(())
}

pub fn run(metric: &str) -> Result<()> {
    match lookup(metric) {
        Some(doc) => print_doc(doc),
        None => {
            let names = all_names().join(", ");
            Err(anyhow!("unknown metric: {metric}\nAvailable: {names}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cyclomatic_has_mccabe_citation() {
        let doc = lookup("cyclomatic").expect("cyclomatic must be in catalog");
        assert!(doc.citation.contains("McCabe"));
        assert!(doc.default_threshold.contains("10"));
    }

    #[test]
    fn cognitive_has_campbell_citation() {
        let doc = lookup("cognitive").expect("cognitive must be in catalog");
        assert!(doc.citation.contains("Campbell"));
        assert!(doc.default_threshold.contains("15"));
    }

    #[test]
    fn abc_has_fitzpatrick_citation() {
        let doc = lookup("abc").expect("abc must be in catalog");
        assert!(doc.citation.contains("Fitzpatrick"));
        assert!(doc.formula.contains("√"));
    }

    #[test]
    fn all_catalog_entries_have_non_empty_fields() {
        for doc in CATALOG {
            assert!(!doc.name.is_empty(), "name empty");
            assert!(!doc.measures.is_empty(), "{}: measures empty", doc.name);
            assert!(!doc.formula.is_empty(), "{}: formula empty", doc.name);
            assert!(!doc.default_threshold.is_empty(), "{}: threshold empty", doc.name);
            assert!(!doc.citation.is_empty(), "{}: citation empty", doc.name);
            assert!(!doc.improve.is_empty(), "{}: improve empty", doc.name);
        }
    }

    #[test]
    fn unknown_metric_returns_error() {
        assert!(run("not_a_metric").is_err());
    }

    #[test]
    fn error_message_lists_available_metrics() {
        let err = run("bogus").unwrap_err().to_string();
        assert!(err.contains("cyclomatic"));
        assert!(err.contains("cognitive"));
    }

    #[test]
    fn all_names_returns_every_catalog_entry() {
        let names = all_names();
        assert!(names.contains(&"cyclomatic"));
        assert!(names.contains(&"abc"));
        assert!(names.contains(&"nesting_max"));
        assert_eq!(names.len(), CATALOG.len());
    }
}
