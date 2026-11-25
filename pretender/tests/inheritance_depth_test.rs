// Purpose: Integration tests for inheritance depth analysis.
// Responsibilities: Assert depth computation for Java chains, Python
//                   inheritance, Rust trait bounds, cross-file parents,
//                   and unsupported-language behavior.
// Rationale: Moved verbatim from pretender/src/inheritance_depth.rs unit
//            tests so the source file passes pretender's own structural
//            gate; assertions are unchanged.

use pretender::inheritance_depth::analyze;
use pretender::model::Language;

#[test]
fn java_single_inheritance() {
    let source = r#"
class A {}
class B extends A {}
class C extends B {}
"#;
    let analysis = analyze(source, &Language::Java);
    let c = analysis
        .classes
        .iter()
        .find(|c| c.name == "C")
        .unwrap_or_else(|| panic!("class C not found"));
    assert_eq!(c.depth, 2);
}

#[test]
fn java_exceeds_threshold() {
    let source = r#"
class A {}
class B extends A {}
class C extends B {}
class D extends C {}
"#;
    let analysis = analyze(source, &Language::Java);
    let d = analysis
        .classes
        .iter()
        .find(|c| c.name == "D")
        .unwrap_or_else(|| panic!("class D not found"));
    assert_eq!(d.depth, 3);
}

#[test]
fn no_parent_depth_zero() {
    let source = "class A {}";
    let analysis = analyze(source, &Language::Java);
    let a = analysis
        .classes
        .iter()
        .find(|c| c.name == "A")
        .unwrap_or_else(|| panic!("class A not found"));
    assert_eq!(a.depth, 0);
}

#[test]
fn python_class_inheritance() {
    let source = r#"
class A:
    pass
class B(A):
    pass
"#;
    let analysis = analyze(source, &Language::Python);
    let b = analysis
        .classes
        .iter()
        .find(|c| c.name == "B")
        .unwrap_or_else(|| panic!("class B not found"));
    assert_eq!(b.depth, 1);
}

#[test]
fn rust_stdlib_trait_excluded() {
    let source = r#"
trait Foo: Display + Debug {}
"#;
    let analysis = analyze(source, &Language::Rust);
    let f = analysis
        .classes
        .iter()
        .find(|c| c.name == "Foo")
        .unwrap_or_else(|| panic!("trait Foo not found"));
    assert_eq!(f.depth, 0);
}

#[test]
fn rust_custom_trait() {
    let source = r#"
trait Bar {}
trait Foo: Bar {}
"#;
    let analysis = analyze(source, &Language::Rust);
    let f = analysis
        .classes
        .iter()
        .find(|c| c.name == "Foo")
        .unwrap_or_else(|| panic!("trait Foo not found"));
    assert_eq!(f.depth, 1);
}

#[test]
fn cross_file_parent_depth_one() {
    let source = "class Foo extends Bar {}";
    let analysis = analyze(source, &Language::Java);
    let f = analysis
        .classes
        .iter()
        .find(|c| c.name == "Foo")
        .unwrap_or_else(|| panic!("class Foo not found"));
    assert_eq!(f.depth, 1);
    assert!(f.cross_file);
}

#[test]
fn inheritance_depth_unsupported_language_empty() {
    let analysis = analyze("nothing", &Language::Julia);
    assert!(analysis.classes.is_empty());
}
