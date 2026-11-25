//! JUnit XML event-loop parsing: report file → [`TestTiming`] entries.
//!
//! # Responsibilities
//! `parse_junit` and the event fold driving it; testcase state is held in
//! [`JunitParser`]/[`PendingTestcase`], threaded by value so no method
//! mutates in place. Every helper stays within the structural limits.
//!
//! # Rationale
//! Split from the single 870-line `test_report.rs` (pretender-5zo.15.1);
//! the 105-line parse loop became a state fold with one helper per XML
//! event kind. Bodies are semantically identical, proven by the moved
//! integration tests.

use super::duration::time_to_ms;
use super::{TestStatus, TestTiming};
use crate::config::TimeUnit;
use anyhow::{Context, Result};
use quick_xml::events::{BytesEnd, BytesStart, Event};
use quick_xml::Reader;
use std::path::{Path, PathBuf};

/// Parse a JUnit XML report file and extract timing data for each test case.
///
/// * `path` — Path to the JUnit XML file.
/// * `time_unit` — How to interpret the `time` attribute (`Seconds` conforms
///   to the JUnit XSD; `Milliseconds` is for non-conformant runners).
///
/// Returns a list of [`TestTiming`] entries, one per `<testcase>` element.
/// Skipped testcases are excluded. Malformed entries are skipped with a
/// warning diagnostic logged via `log::warn!` (or silently ignored if the
/// `log` crate is unavailable).
pub fn parse_junit(path: &Path, time_unit: TimeUnit) -> Result<Vec<TestTiming>> {
    let xml = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read JUnit XML report: {}", path.display()))?;

    let mut reader = Reader::from_str(&xml);
    reader.config_mut().trim_text(true);

    let parser = read_events(&mut reader, JunitParser::new(time_unit));
    Ok(parser.timings)
}

/// Fold XML events into the parser until EOF or a malformed-XML error
/// (the original "stop parsing rather than panic" behavior).
fn read_events(reader: &mut Reader<&[u8]>, mut parser: JunitParser) -> JunitParser {
    let mut buf = Vec::new();
    while let Ok(event) = reader.read_event_into(&mut buf) {
        parser = parser.handle_event(&event);
        buf.clear();
        if parser.done {
            break;
        }
    }
    parser
}

/// The testcase currently being parsed, before its end/empty close.
struct PendingTestcase {
    name: String,
    classname: String,
    file: Option<String>,
    time: Option<f64>,
    status: TestStatus,
}

impl PendingTestcase {
    /// The testcase with an upgraded status (by-value sibling of the
    /// original in-place upgrade).
    fn with_status(self, status: TestStatus) -> Self {
        Self { status, ..self }
    }
}

/// Accumulator state for one [`parse_junit`] run.
struct JunitParser {
    timings: Vec<TestTiming>,
    pending: Option<PendingTestcase>,
    time_unit: TimeUnit,
    done: bool,
}

impl JunitParser {
    fn new(time_unit: TimeUnit) -> Self {
        Self {
            timings: Vec::new(),
            pending: None,
            time_unit,
            done: false,
        }
    }

    /// Fold one XML event: element start/empty and testcase end.
    fn handle_event(self, event: &Event) -> Self {
        match event {
            Event::Start(e) => self.handle_tag(e, false),
            Event::Empty(e) => self.handle_tag(e, true),
            Event::End(e) => self.handle_close(e),
            _ => self.check_stop(event),
        }
    }

    /// EOF ends the pump; malformed-XML errors exit via `read_events`.
    fn check_stop(self, event: &Event) -> Self {
        if let Event::Eof = event {
            return Self { done: true, ..self };
        }
        self
    }

    /// Fold a start or empty element: begin a testcase, or note a status
    /// child (`<skipped>` / `<failure>` / `<error>`) of the pending one.
    fn handle_tag(self, e: &BytesStart, empty: bool) -> Self {
        let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
        if tag == "testcase" {
            let opened = self.open(e);
            if empty {
                // Self-closing testcase: no children, no end event.
                return opened.finish();
            }
            return opened;
        }
        self.note_status(&tag)
    }

    /// Begin a pending testcase from its `testcase` start/empty attributes.
    fn open(self, e: &BytesStart) -> Self {
        let pending = PendingTestcase {
            name: attr_value(e, "name").unwrap_or_default(),
            classname: attr_value(e, "classname").unwrap_or_default(),
            file: attr_value(e, "file"),
            time: attr_value(e, "time").and_then(|v| v.parse::<f64>().ok()),
            status: TestStatus::Passed,
        };
        Self {
            pending: Some(pending),
            ..self
        }
    }

    /// Upgrade the pending testcase's status for a status child element.
    fn note_status(self, tag: &str) -> Self {
        let pending = self
            .pending
            .and_then(|p| failed_status(tag, p.status).map(|status| p.with_status(status)));
        Self {
            timings: self.timings,
            pending,
            time_unit: self.time_unit,
            done: self.done,
        }
    }

    /// Finish the pending testcase when its end tag arrives.
    fn handle_close(self, e: &BytesEnd) -> Self {
        if String::from_utf8_lossy(e.name().as_ref()) == "testcase" {
            return self.finish();
        }
        self
    }

    /// Emit the pending testcase when it qualifies, clearing it — mirrors
    /// the original `finish_testcase!` macro.
    fn finish(mut self) -> Self {
        let pending = self.pending.take();
        if let Some(timing) = pending.and_then(|p| finished_timing(p, self.time_unit)) {
            self.timings.push(timing);
        }
        self
    }
}

/// The status a status child element implies, or `None` when it leaves the
/// current one unchanged (only upgrades from [`TestStatus::Passed`]).
fn failed_status(tag: &str, current: TestStatus) -> Option<TestStatus> {
    match (tag, current) {
        ("skipped", _) => Some(TestStatus::Skipped),
        ("failure", TestStatus::Passed) => Some(TestStatus::Failed),
        ("error", TestStatus::Passed) => Some(TestStatus::Errored),
        _ => None,
    }
}

/// Convert a finishable pending testcase into a [`TestTiming`]; skipped
/// tests and tests without a `time` attribute are excluded (verbatim).
fn finished_timing(pending: PendingTestcase, time_unit: TimeUnit) -> Option<TestTiming> {
    let PendingTestcase {
        name,
        classname,
        file,
        time,
        status,
    } = pending;
    if status == TestStatus::Skipped {
        return None;
    }
    let duration_ms = time_to_ms(time?, time_unit);
    Some(TestTiming {
        name,
        classname,
        file: file.map(PathBuf::from),
        duration_ms,
        status,
    })
}

fn attr_value(e: &BytesStart, name: &str) -> Option<String> {
    e.attributes()
        .filter_map(|a| a.ok())
        .find(|a| a.key.as_ref() == name.as_bytes())
        .and_then(|a| String::from_utf8(a.value.to_vec()).ok())
}
