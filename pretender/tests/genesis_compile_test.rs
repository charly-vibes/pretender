/// Compile-time test verifying genesis crate modules are accessible.
/// This ensures the git dependency resolves and the required modules
/// (envelope, suggestions, feedback) are stable.
#[test]
fn genesis_envelope_accessible() {
    use genesis::envelope::{set_author, Envelope, EnvelopeKind, ErrorResult, RemediationEntry};

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
    registry.register(
        "pretender",
        vec![
            "check".into(),
            "complexity".into(),
            "report".into(),
            "doctor".into(),
            "init".into(),
            "duplication".into(),
            "mutation".into(),
        ],
    );

    // Typo detection
    let suggestion = engine.suggest_typo("complxity", &registry);
    assert!(suggestion.is_some());
    if let Some(Suggestion::DidYouMean {
        original,
        suggestion,
    }) = suggestion
    {
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
    let _reduced =
        genesis::feedback::redactor::reduce_git_remote_url("https://github.com/owner/repo.git");
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

#[test]
fn genesis_config_accessible() {
    use genesis::config::{ConfigRegistry, ConfigStore, ConfigValidation};
    use serde::Deserialize;
    use std::path::{Path, PathBuf};

    // A minimal config type implementing ConfigFile.
    #[derive(Debug, Default, PartialEq, Eq, Deserialize)]
    struct MockConfig {
        name: String,
    }

    impl genesis::config::ConfigFile for MockConfig {
        fn path(repo_root: &Path) -> PathBuf {
            repo_root.join("mock.toml")
        }
    }

    // Registry registration + discovery contract.
    let mut registry = ConfigRegistry::new();
    registry.register::<MockConfig>("mock", "mock.toml");
    assert!(registry.is_registered("mock"));
    assert_eq!(registry.marker("mock"), Some("mock.toml"));

    let store = ConfigStore::new(registry);
    assert!(!store.registry().is_empty());

    // Validation result constructors compile.
    let warn = ConfigValidation::warning("x", "be careful");
    let err = ConfigValidation::error("y", "no good");
    assert_eq!(warn.field, "x");
    assert_eq!(err.field, "y");
}

#[test]
fn genesis_guide_accessible() {
    use genesis::guide::{Guide, Output, Verbosity};

    // Guide builder assembles a CLI scaffold.
    let guide = Guide::builder("pretender", "0.3.1")
        .commands(&["check", "doctor"])
        .max_verbosity(2)
        .build();
    assert_eq!(guide.name(), "pretender");
    assert_eq!(guide.version(), "0.3.1");
    assert_eq!(guide.verbosity(), Verbosity::Verbose);
    assert!(guide.registry().all().contains(&"check"));

    // Output<T> fluent construction + verbosity filtering.
    let output: Output<&str> = Output::success("done")
        .with_next_step("run doctor")
        .with_warning("check config");
    assert!(!output.is_error);
    assert_eq!(output.warnings.len(), 1);

    // ErrorSink is wired for the tool.
    let sink = guide.error_sink();
    assert_eq!(sink.tool_name, "pretender");

    // Guide::run returns 0 on a success Output.
    let exit = guide.run(|| Ok(Output::success("ok")));
    assert_eq!(exit, 0);
}
