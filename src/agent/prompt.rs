//! The agent's system prompt: the built-in text and its default/override
//! resolution (LCV-143).
//!
//! Kernel-pure: no `egui`, no I/O, no `Settings`. The persisted override is a
//! plain `Option<String>` on `io::settings::Settings`, which knows nothing of
//! the default; `app::agent_turn::turn_config` hands it to [`resolve`] once
//! per turn and the worker only ever sees the resolved text.
//!
//! The prompt grants nothing. Tool advertisement, argument validation, the
//! step budget and the revision fence are all enforced in code, so an
//! override — adversarial or blank — changes wording, never capability.

/// The built-in system prompt, used whenever no override is set.
///
/// The fourth paragraph is the index contract of ADR 0007 §D5: entity handles
/// are positional, so a delete renumbers what the model is about to touch.
/// The last paragraph is the fence's refusal seen from the model's side —
/// retrying cannot help, because the fence is sticky.
pub const DEFAULT_PROMPT: &str = "\
You are the CAD assistant embedded in LaserCAD v2, a focused 2D CAD
application for preparing LaserGRBL-compatible laser drawings.

When asked to construct, modify, or inspect the open drawing, call the
advertised harness tools to do the work; do not only describe how to do it.
Ask a focused question when required dimensions or intent are missing.
Use the fewest tool calls that correctly satisfy the request.

All drawing coordinates, lengths, and radii are canonical millimeters (mm).
Follow each tool schema for angles: existing arc tools accept degrees;
the geometry kernel uses radians. Do not substitute pixels for geometry.

Entity indices are positional, not stable IDs. Deleting an entity shifts
every higher index down by one. Query the live drawing before targeting an
index you have not read in this turn, and query again after deletion before
reusing potentially stale indices.

If create_drawing is advertised, use it for suitable append-only batches
of lines, circles, and arcs, within its declared validation and size limits.
Request a canvas capture only if that tool is advertised and enabled.
Do not invent tools, skills, permissions, or capabilities.

Check every tool outcome. Report only changes and observations that actually
succeeded; never claim unperformed work. If the drawing-change fence refuses
an action or the turn is cancelled, stop rather than retrying. State any
partial completion or refusal honestly and summarize the outcome concisely.";

/// The effective system prompt: the override if one is set, else
/// [`DEFAULT_PROMPT`].
///
/// A present override is returned verbatim — never trimmed or repaired, and a
/// blank or whitespace-only one is honoured rather than replaced by the
/// default (LCV-143 AC 2).
pub fn resolve(override_text: Option<&str>) -> &str {
    override_text.unwrap_or(DEFAULT_PROMPT)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ADR 0007 §D5 — the default discloses the positional-index contract and
    /// the stop-don't-retry fence advice, and promises no stable ids.
    #[test]
    fn the_default_prompt_discloses_the_index_contract() {
        for needle in [
            "positional, not stable IDs",
            "shifts\nevery higher index down by one",
            "Query the live drawing",
            "query again after deletion",
            "millimeters (mm)",
            "degrees",
            "stop rather than retrying",
        ] {
            assert!(
                DEFAULT_PROMPT.contains(needle),
                "the prompt must say `{needle}`"
            );
        }
    }

    /// The default carries no trailing whitespace a hand edit could not see.
    #[test]
    fn the_default_prompt_has_no_edge_whitespace() {
        assert_eq!(DEFAULT_PROMPT, DEFAULT_PROMPT.trim());
    }
}
