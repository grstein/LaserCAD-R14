//! Text form of a [`CheckReport`]: the lines every consumer prints.
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`.

use super::{CheckReport, Finding};
use crate::geometry::Vec2;

/// The only line of a report without findings.
const CLEAN: &str = "CHECK: no problems found.";

/// Kind order of the summary and of the finding lines, with the singular and
/// plural noun of each summary line.
const KINDS: [(&str, &str); 5] = [
    ("open end", "open ends"),
    ("gap", "gaps"),
    ("duplicate", "duplicates"),
    ("degenerate entity", "degenerate entities"),
    ("off-bed entity", "off-bed entities"),
];

impl CheckReport {
    /// Summary lines first, one per kind with a count (`CHECK: 2 open ends`),
    /// in the kind order open end, gap, duplicate, degenerate, off-bed; then
    /// one line per finding, in the report's order. Coordinates are mm at
    /// three decimals. With no findings the only line is
    /// `CHECK: no problems found.`
    pub fn lines(&self) -> Vec<String> {
        if self.findings.is_empty() {
            return vec![CLEAN.to_string()];
        }
        let mut counts = [0_usize; KINDS.len()];
        for f in &self.findings {
            counts[kind(f)] += 1;
        }
        let summary = KINDS.iter().zip(counts).filter(|(_, n)| *n > 0);
        let summary = summary.map(|((one, many), n)| {
            let noun = if n == 1 { one } else { many };
            format!("CHECK: {n} {noun}")
        });
        summary.chain(self.findings.iter().map(line)).collect()
    }
}

/// Position of `f`'s kind in [`KINDS`].
fn kind(f: &Finding) -> usize {
    match f {
        Finding::OpenEnd { .. } => 0,
        Finding::Gap { .. } => 1,
        Finding::Duplicate { .. } => 2,
        Finding::Degenerate { .. } => 3,
        Finding::OffBed { .. } => 4,
    }
}

/// `(x, y)` at three decimals.
fn pt(p: Vec2) -> String {
    format!("({:.3}, {:.3})", p.x, p.y)
}

/// The finding line of `f`.
fn line(f: &Finding) -> String {
    match f {
        Finding::OpenEnd { index, at } => format!("open end: entity {index} at {} mm", pt(*at)),
        Finding::Gap { a, b, mid, width } => format!(
            "gap: entities {a} and {b}, {width:.3} mm wide at {} mm",
            pt(*mid)
        ),
        Finding::Duplicate { index, of, at } => {
            format!("duplicate: entity {index} of entity {of} at {} mm", pt(*at))
        }
        Finding::Degenerate { index, at } => {
            format!("degenerate: entity {index} at {} mm", pt(*at))
        }
        Finding::OffBed { index, min, max } => format!(
            "off-bed: entity {index} from {} to {} mm",
            pt(*min),
            pt(*max)
        ),
    }
}
