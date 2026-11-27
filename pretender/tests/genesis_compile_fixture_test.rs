/// Purpose: Compile-time smoke test verifying the fixture-surface genesis
/// modules (guide, cli, scaffold, discovery, fixture, aix) resolve and
/// behave as documented.
/// Responsibilities:
/// - Exercise each module's key constructors/functions for real, not just
///   reference symbols, and assert the documented outcomes.
/// - Run filesystem-touching paths (scaffold, discovery, fixture) inside
///   throwaway fixture roots.
///
/// Rationale: The genesis crate is a git/crates.io dependency consumed by
/// this workspace; these tests fail fast on dependency upgrades that break
/// its API surface or invariants.
#[test]
fn genesis_guide_accessible() {
    use genesis::guide::{Guide, Output, Verbosity};

    // Guide builder assembles a CLI scaffold.
    let guide = Guide::builder("pretender", "0.4.0")
        .commands(&["check", "doctor"])
        .max_verbosity(2)
        .build();
    assert_eq!(guide.name(), "pretender");
    assert_eq!(guide.version(), "0.4.0");
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

#[test]
fn genesis_cli_accessible() {
    // Verify the module and its key functions compile
    // (real usage in main.rs: Completions variant handler and version-json pre-parse guard)
    let _ = genesis::cli::generate_completions;
    // Real behavior: a no-flag invocation must return false and print nothing.
    assert!(!genesis::cli::maybe_print_version_json(
        "pretender",
        "0.2.0"
    ));
}

#[test]
fn genesis_scaffold_accessible() {
    use genesis::fixture::Fixture;

    // Run the builder for real inside a throwaway fixture root.
    let fixture = Fixture::new()
        .with_marker(".git")
        .build()
        .unwrap_or_else(|e| panic!("fixture should build: {e:?}"));
    let result = genesis::scaffold::Scaffold::new(fixture.root())
        .dir("tools")
        .default_config("tools.toml", "[probe]\n")
        .build()
        .unwrap_or_else(|e| panic!("scaffold should build: {e:?}"));
    assert_eq!(result.created.len(), 2, "dir + config should be created");
    assert!(
        fixture.root().join("tools.toml").exists(),
        "scaffold should write tools.toml"
    );
}

#[test]
fn genesis_discovery_accessible() {
    use genesis::fixture::Fixture;

    // register() writes .genesis/tools.toml under project_root — run it in a
    // throwaway fixture root instead of just referencing the symbol.
    let fixture = Fixture::new()
        .with_marker(".git")
        .build()
        .unwrap_or_else(|e| panic!("fixture should build: {e:?}"));
    let result = genesis::discovery::register(fixture.root(), "probe", "probe", "file", "probe.sh");
    assert!(result.is_ok(), "register should succeed in fixture root");
    assert!(
        fixture.root().join(".genesis").exists(),
        "register should create the .genesis directory"
    );
}

#[test]
fn genesis_fixture_accessible() {
    use genesis::fixture::Fixture;

    let fixture = Fixture::new()
        .with_file("hello.txt", "world")
        .with_marker(".git")
        .build()
        .unwrap_or_else(|e| panic!("fixture should build: {e:?}"));

    let path = fixture.path("hello.txt");
    assert!(path.exists(), "fixture file should exist on disk");
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("should read fixture file: {e:?}"));
    assert_eq!(content, "world", "fixture file content should match");

    let marker = fixture.path(".git");
    assert!(marker.exists(), "fixture marker dir should exist on disk");
}

#[test]
fn genesis_aix_accessible() {
    use genesis::aix::agents_block;

    let block = agents_block("WAI", "Agent instructions for wai");
    assert!(block.contains("WAI"), "block should contain the tool name");
    assert!(
        block.contains("Agent instructions for wai"),
        "block should contain the body"
    );
}
