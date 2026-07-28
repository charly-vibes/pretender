/// Compile-time test verifying genesis crate modules are accessible.
/// This ensures the git dependency resolves and the required modules
/// (envelope, suggestions, feedback) are stable.
#[test]
fn genesis_envelope_accessible() {
    use genesis::envelope::{Envelope, EnvelopeKind, ErrorResult, RemediationEntry, set_author};

    // Create a success envelope
    let env = Envelope::success(
        EnvelopeKind::Ok,
        serde_json::json!({"key": "value"}),
        vec![],
        vec![],
    );
    assert!(env.ok);
    assert_eq!(env.envelope_kind, EnvelopeKind::Ok);

    // Create an error envelope
    let err = ErrorResult::new(
        "E001",
        "test error",
        None,
        None,
        None,
        vec![],
        vec![RemediationEntry {
            command: "just fix".into(),
            description: "run fix".into(),
        }],
    )
    .expect("remediation must be non-empty");
    let error_env = Envelope::error(err, vec![]);
    assert!(!error_env.ok);
    assert_eq!(error_env.envelope_kind, EnvelopeKind::Error);

    // set_author compiles
    set_author("pretender".into());

    // EnvelopeKind variants compile
    let _ = EnvelopeKind::Check;
    let _ = EnvelopeKind::List;
    let _ = EnvelopeKind::Doctor;
    let _ = EnvelopeKind::Empty;
}

#[test]
fn genesis_suggestions_accessible() {
    use genesis::suggestions::{CommandRegistry, Suggestion, SuggestionEngine};

    // Create an engine and registry
    let engine = SuggestionEngine::new();
    let mut registry = CommandRegistry::new();
    registry.register("pretender", vec![
        "check".into(),
        "complexity".into(),
        "report".into(),
        "doctor".into(),
        "init".into(),
        "duplication".into(),
        "mutation".into(),
    ]);

    // Typo detection
    let suggestion = engine.suggest_typo("complxity", &registry);
    assert!(suggestion.is_some());
    if let Some(Suggestion::DidYouMean { original, suggestion }) = suggestion {
        assert_eq!(original, "complxity");
        assert_eq!(suggestion, "complexity");
    }

    // Footer formatting
    let footer = Suggestion::DidYouMean {
        original: "staus".into(),
        suggestion: "status".into(),
    }
    .footer();
    assert_eq!(footer, Some("→ Run: status".into()));
}

#[test]
fn genesis_feedback_modules_accessible() {
    // Verify feedback::scratch compiles
    let record = genesis::feedback::scratch::ErrorRecord {
        ts: "2026-07-28T00:00:00Z".into(),
        argv: vec!["pretender".into(), "check".into()],
        exit: 1,
        footer: Some("→ Run: pretender doctor".into()),
        kind: "Fix".into(),
    };
    genesis::feedback::scratch::write_scratch_best_effort("pretender", &record);

    // Verify feedback::redactor compiles
    let _reduced = genesis::feedback::redactor::reduce_git_remote_url(
        "https://github.com/owner/repo.git",
    );
}

#[test]
fn genesis_managed_block_accessible() {
    use genesis::managed_block::{BlockDef, BlockInjector, BlockRegistry};

    // Create a registry and injector
    let mut reg = BlockRegistry::new();
    reg.register(BlockDef::new("WAI"));
    reg.register(BlockDef::new("OPENSPEC"));
    reg.register(BlockDef::new("DONT"));

    let injector = BlockInjector::new(reg);

    // Verify names
    let names = injector.registry().names();
    assert!(names.contains(&"WAI"));
    assert!(names.contains(&"DONT"));
    assert!(names.contains(&"OPENSPEC"));
}

#[test]
fn genesis_constants_accessible() {
    // Verify constants compile
    let _version = genesis::envelope::ENVELOPE_VERSION;
    let _cli_version = genesis::envelope::CLI_VERSION;
}