//! Fixture: shared test-support plumbing (pretender-jg9y gap 3).
//!
//! assert_cmd-style wiring: the `.expect(...)` calls are error plumbing,
//! not mock usage. From a `tests/support/` path this module must not fire
//! `mock_count`; staged as a leaf `*_test.rs` it still must.

use std::path::Path;
use std::process::Output;

pub fn run_tool(repo: &Path) -> Output {
    std::process::Command::new("git")
        .current_dir(repo)
        .output()
        .expect("tool runs")
}

pub fn read_fixture(path: &Path) -> String {
    std::fs::read_to_string(path).expect("fixture is readable")
}

pub fn write_fixture(path: &Path, contents: &str) {
    std::fs::write(path, contents).expect("fixture is writable");
}

pub fn make_dir(path: &Path) {
    std::fs::create_dir_all(path).expect("directory is created");
}

pub fn copy_fixture(source: &Path, destination: &Path) {
    std::fs::copy(source, destination).expect("fixture is copied");
}

pub fn remove_dir(path: &Path) {
    std::fs::remove_dir_all(path).expect("directory is removed");
}

pub fn read_env(key: &str) -> String {
    std::env::var(key).expect("env var is set")
}

pub fn current_dir() -> std::path::PathBuf {
    std::env::current_dir().expect("current dir resolves")
}
