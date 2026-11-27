//! Fixture: a 3-link assert_cmd-style assertion chain (pretender-jg9y gap 1).
//!
//! Every link in the chain is its own call: `.assert()` + `.success()` +
//! `.stdout(...)` must count as 3 assertions for the `three_link_chain`
//! unit, not 1 (or 0).

struct Harness;

impl Harness {
    fn assert(&self) -> Assert {
        Assert
    }
}

struct Assert;

impl Assert {
    fn success(self) -> Self {
        self
    }

    fn stdout(self, _expected: &str) -> Self {
        self
    }
}

fn harness() -> Harness {
    Harness
}

#[test]
fn three_link_chain() {
    harness().assert().success().stdout("ok");
}
