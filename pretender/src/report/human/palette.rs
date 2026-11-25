//! Per-pass color palette and severity markers for the human writer.
//!
//! # Responsibilities
//! Own `HumanPalette` construction (ANSI color codes and gate/advisory
//! icons) and the severity → (icon, accent, reset) mapping used by the
//! human-report stage writers.
//!
//! # Rationale
//! Split from `human/mod.rs` so both files stay within the gate's
//! file-lines cap; bodies moved verbatim to keep output bytes unchanged.

use super::super::summary::Severity;

/// Color codes and icons shared by one human-report rendering pass.
pub(super) struct HumanPalette {
    pub(super) red: &'static str,
    pub(super) yellow: &'static str,
    pub(super) green: &'static str,
    pub(super) reset: &'static str,
    pub(super) icon: &'static str,
    pub(super) label: &'static str,
    pub(super) accent: &'static str,
}

/// ANSI code when color is on, empty string otherwise.
fn color_or_empty(color: bool, code: &'static str) -> &'static str {
    if color {
        code
    } else {
        ""
    }
}

pub(super) fn human_palette(color: bool, blocking: bool) -> HumanPalette {
    let red = color_or_empty(color, "\u{1b}[31m");
    let yellow = color_or_empty(color, "\u{1b}[33m");
    let green = color_or_empty(color, "\u{1b}[32m");
    let reset = color_or_empty(color, "\u{1b}[0m");
    let (icon, label, accent) = if blocking {
        ("✗", "VIOLATION", red)
    } else {
        ("⚠", "ADVISORY", yellow)
    };
    HumanPalette {
        red,
        yellow,
        green,
        reset,
        icon,
        label,
        accent,
    }
}

/// The (icon, accent, reset) triple a file's severity renders with. Gate
/// mode stains red findings with the hard-fail icon; advisory modes keep
/// them yellow.
pub(super) fn severity_marker(
    severity: Severity,
    palette: &HumanPalette,
    blocking: bool,
) -> (&'static str, &'static str, &'static str) {
    let red_accent = if blocking {
        palette.red
    } else {
        palette.yellow
    };
    match severity {
        Severity::Green => ("✓", palette.green, palette.reset),
        Severity::Yellow => ("⚠", palette.yellow, palette.reset),
        Severity::Red => (palette.icon, red_accent, palette.reset),
    }
}
