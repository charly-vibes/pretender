//! Unit pins for the `error_metrics` unwrap heuristics (moved out of the
//! app file to stay under the pretender file-lines budget).

use pretender::error_metrics::{detect_unwraps, detect_unwraps_with_role, UnwrapKind};
use pretender::model::Language;
use pretender::roles::Role;

#[test]
fn detects_rust_unwrap_and_expect() {
    let source = r#"
fn parse(input: &str) -> i32 {
    input.parse().unwrap()
}
fn logged(input: &str) {
    let x = input.to_string().expect("should be valid");
}
"#;
    let sites = detect_unwraps(source, &Language::Rust);
    assert_eq!(sites.len(), 2);
    assert_eq!(sites[0].kind, UnwrapKind::Unwrap);
    assert_eq!(sites[1].kind, UnwrapKind::Expect);
}

#[test]
fn unwrap_or_recovery_never_counts() {
    // pretender-jg9y: unwrap_or recovers instead of panicking.
    let source =
        "let _ = x.unwrap_or(0);\nlet _ = x.unwrap_or_else(f);\nlet _ = x.unwrap_or_default();\n";
    let sites = detect_unwraps(source, &Language::Rust);
    assert_eq!(sites.len(), 0);
}

#[test]
fn cfg_test_module_pin_by_role() {
    // pretender-jg9y: for test-role files, expects inside #[cfg(test)]-gated
    // modules are excluded (harness plumbing), but gated unwraps still count;
    // the region closes at module end; app role always counts.
    let gated = "#[cfg(test)]\nmod tests {\n    fn inner() {\n        let x = value.expect(\"gated\");\n    }\n}\n";
    let test_sites = detect_unwraps_with_role(gated, &Language::Rust, Role::Test);
    assert_eq!(test_sites.len(), 0);
    let app_sites = detect_unwraps_with_role(gated, &Language::Rust, Role::App);
    assert_eq!(app_sites.len(), 1);
    assert_eq!(app_sites[0].kind, UnwrapKind::Expect);

    let unwraps =
        "#[cfg(test)]\nmod tests {\n    fn inner() {\n        let x = value.unwrap();\n    }\n}\n";
    let sites = detect_unwraps_with_role(unwraps, &Language::Rust, Role::Test);
    assert_eq!(sites.len(), 1);
    assert_eq!(sites[0].kind, UnwrapKind::Unwrap);

    let closed = "#[cfg(test)]\nmod tests {\n    fn inner() {\n        let x = value.expect(\"gated\");\n    }\n}\n\nfn after() {\n    let x = value.expect(\"after\");\n}\n";
    let sites = detect_unwraps_with_role(closed, &Language::Rust, Role::Test);
    assert_eq!(sites.len(), 1);
    assert_eq!(sites[0].line, 9);
}

#[test]
fn detects_other_language_error_markers() {
    let python = r#"
try:
    do_something()
except:
    pass
"#;
    let sites = detect_unwraps(python, &Language::Python);
    assert_eq!(sites.len(), 1);
    assert_eq!(sites[0].kind, UnwrapKind::BareExcept);

    let propagation = r#"
if err != nil {
    return err
}
"#;
    let sites = detect_unwraps(propagation, &Language::Go);
    assert_eq!(sites.len(), 0);

    let ignored = r#"_ = doSomething()"#;
    let sites = detect_unwraps(ignored, &Language::Go);
    assert_eq!(sites.len(), 1);
    assert_eq!(sites[0].kind, UnwrapKind::IgnoredError);

    let js = r#"
try {
    doSomething();
} catch(e) {}
"#;
    let sites = detect_unwraps(js, &Language::JavaScript);
    assert_eq!(sites.len(), 1);
    assert_eq!(sites[0].kind, UnwrapKind::EmptyCatch);
}

#[test]
fn unsupported_and_clean_sources_are_ignored() {
    let sites = detect_unwraps("nothing", &Language::Julia);
    assert_eq!(sites.len(), 0);
    let clean = r#"
fn safe_parse(input: &str) -> Result<i32, String> {
    input.parse().map_err(|e| e.to_string())
}
"#;
    let sites = detect_unwraps(clean, &Language::Rust);
    assert_eq!(sites.len(), 0);
}
