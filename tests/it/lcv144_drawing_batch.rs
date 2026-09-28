//! LCV-144 — `create_drawing`: one validated JSON batch is one dispatch, one
//! command, one revision (ADR 0010).
//!
//! Every haystack of a source scan is a `src/` file cut at its first column-0
//! `#[cfg(test)]`, so a needle quoted by a test module cannot satisfy or defeat
//! the scan.

use std::path::Path;

fn implementation(relative: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
    let src = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let end = src.find("\n#[cfg(test)]").unwrap_or(src.len());
    src[..end].to_owned()
}

/// AC 11 — the eight narration helpers are defined in `agent_narrate.rs` and
/// not in `agent_apply.rs` (ADR 0010 §9). The positive control is `apply`,
/// which must stay in `agent_apply.rs`, so a wrong path or an empty read fails.
#[test]
fn ac11_the_narration_helpers_live_in_agent_narrate() {
    let apply = implementation("src/app/agent_apply.rs");
    let narrate = implementation("src/app/agent_narrate.rs");
    assert!(
        apply.contains("pub fn apply("),
        "positive control: agent_apply.rs keeps `apply`"
    );
    for helper in [
        "pt",
        "sweep",
        "kind",
        "geometry",
        "describe",
        "bed_line",
        "list_entities",
        "list_selection",
    ] {
        let needle = format!("fn {helper}(");
        assert!(
            narrate.contains(&needle),
            "agent_narrate.rs must define `{needle}`"
        );
        assert!(
            !apply.contains(&needle),
            "agent_apply.rs must not define `{needle}`"
        );
    }
}
