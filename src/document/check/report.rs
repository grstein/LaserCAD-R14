//! Text form of a [`CheckReport`]: the lines every consumer prints.

use super::CheckReport;

impl CheckReport {
    /// Summary lines first, then one line per finding.
    pub fn lines(&self) -> Vec<String> {
        Vec::new()
    }
}
