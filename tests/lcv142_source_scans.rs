//! LCV-142 AC 13 — the file split of ADR 0007 §D8 (amendment 7), with the field
//! paths of amendment 8.
//!
//! Every haystack is a `src/` file cut at its first column-0 `#[cfg(test)]`,
//! so a needle quoted by a test module cannot satisfy or defeat the scan. Every
//! absence is paired with a presence in the same haystack, so a wrong path or
//! an empty read fails loudly instead of passing.

use std::path::Path;

fn implementation(relative: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
    let src = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let end = src.find("\n#[cfg(test)]").unwrap_or(src.len());
    src[..end].to_owned()
}

/// AC 13 — `run_agent_turn`, `ask_ui` and `TurnConfig` are defined in
/// `agent_worker.rs` and not in `agent_turn.rs`, which keeps the fence, the
/// turn state, `arm_turn` and `start_turn`.
#[test]
fn ac13_the_worker_half_lives_in_agent_worker() {
    let worker = implementation("src/app/agent_worker.rs");
    let turn = implementation("src/app/agent_turn.rs");
    for needle in ["fn run_agent_turn", "fn ask_ui", "struct TurnConfig"] {
        assert!(
            worker.contains(needle),
            "agent_worker.rs must define `{needle}`"
        );
        assert!(
            !turn.contains(needle),
            "agent_turn.rs must not define `{needle}`"
        );
    }
    for needle in [
        "struct TurnFence",
        "struct TurnState",
        "fn arm_turn",
        "fn start_turn",
    ] {
        assert!(turn.contains(needle), "agent_turn.rs keeps `{needle}`");
        assert!(
            !worker.contains(needle),
            "agent_worker.rs must not define `{needle}`"
        );
    }
}

/// AC 13 — `AgentState` holds one `pub turn: TurnState` instead of the three
/// loose fields; `busy` (and `rx`) stay direct fields.
#[test]
fn ac13_agent_state_nests_the_turn() {
    let state = implementation("src/app/agent_state.rs");
    assert!(state.contains("pub busy:"), "positive control: `pub busy:`");
    assert!(state.contains("pub rx:"), "`rx` stays a direct field");
    assert!(state.contains("pub turn: TurnState"));
    for gone in ["pub fence:", "pub applied:", "pub turn_label:"] {
        assert!(
            !state.contains(gone),
            "`{gone}` must be folded into TurnState"
        );
    }
}
