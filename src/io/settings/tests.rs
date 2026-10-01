use super::*;
use std::fs;

// ------------------------------------------------------------------
// Settings struct
// ------------------------------------------------------------------

/// AC#1 — default settings have an empty recent-files list.
#[test]
fn default_settings_have_empty_recent_files() {
    let s = Settings::default();
    assert!(s.recent_files.is_empty());
}

/// AC#3 — push_recent_file inserts at position 0 (most-recent first).
#[test]
fn push_recent_file_inserts_at_front() {
    let mut s = Settings::default();
    s.push_recent_file("a.lcad".into());
    s.push_recent_file("b.lcad".into());
    s.push_recent_file("c.lcad".into());
    assert_eq!(s.recent_files, vec!["c.lcad", "b.lcad", "a.lcad"]);
}

/// AC#2 — push_recent_file deduplicates: existing occurrence is removed
/// before re-inserting at the front.
#[test]
fn push_recent_file_deduplicates() {
    let mut s = Settings::default();
    s.push_recent_file("a.lcad".into());
    s.push_recent_file("b.lcad".into());
    // Push "a" again — it should move to the front, not appear twice.
    s.push_recent_file("a.lcad".into());
    assert_eq!(s.recent_files, vec!["a.lcad", "b.lcad"]);
}

/// AC#4 — push_recent_file caps the list at exactly 10 entries.
#[test]
fn push_recent_file_caps_at_ten() {
    let mut s = Settings::default();
    for i in 0..12u32 {
        s.push_recent_file(format!("file{i}.lcad"));
    }
    assert_eq!(s.recent_files.len(), 10);
    // Most-recently pushed is at the front.
    assert_eq!(s.recent_files[0], "file11.lcad");
}

// ------------------------------------------------------------------
// LCV-076 — agent settings fields
// ------------------------------------------------------------------

/// §T1 — AC 1 + AC 2 (LCV-076) / AC 5 + AC 6 (LCV-101) — agent field
/// defaults: OpenRouter endpoint, empty key, empty recent list.
#[test]
fn agent_field_defaults() {
    let s = Settings::default();
    assert_eq!(s.agent_endpoint, "https://openrouter.ai/api/v1");
    assert_eq!(s.agent_api_key, "");
    assert!(s.recent_files.is_empty());
}

/// §T2 — AC 4 (LCV-076) / AC 14 (LCV-101) — agent-field round-trip
/// preserves a non-default endpoint, proving persistence rather than
/// tautologically matching the (now identical) default.
#[test]
fn agent_fields_round_trip() {
    let tmp = std::env::temp_dir().join("lcv076_agent_roundtrip.json");

    let original = Settings {
        agent_endpoint: "https://api.openai.com/v1".into(),
        agent_api_key: "sk-test".into(),
        ..Settings::default()
    };

    save_to(&original, &tmp).unwrap();
    let loaded = load_from(&tmp);

    assert_eq!(original, loaded);
    assert_ne!(loaded.agent_endpoint, Settings::default().agent_endpoint);
    let _ = fs::remove_file(&tmp);
}

/// AC 7 — a settings file that stores an explicit (non-default)
/// endpoint keeps it: the fallback default must not clobber a value the
/// user actually set.
#[test]
fn explicit_endpoint_in_file_is_preserved() {
    let tmp = std::env::temp_dir().join("lcv101_explicit_endpoint.json");
    fs::write(&tmp, r#"{"agent_endpoint":"https://api.openai.com/v1"}"#).unwrap();

    let result = load_from(&tmp);
    assert_eq!(result.agent_endpoint, "https://api.openai.com/v1");

    let _ = fs::remove_file(&tmp);
}

/// §T2b — AC 3 (LCV-076) / AC 8 (LCV-101) — legacy JSON (no agent
/// fields) deserialises to the OpenRouter default.
#[test]
fn legacy_json_without_agent_fields_uses_defaults() {
    let tmp = std::env::temp_dir().join("lcv076_legacy_compat.json");
    fs::write(&tmp, r#"{"recent_files":[]}"#).unwrap();

    let result = load_from(&tmp);
    assert_eq!(result.agent_endpoint, "https://openrouter.ai/api/v1");
    assert_eq!(result.agent_api_key, "");

    let _ = fs::remove_file(&tmp);
}

/// LCV-114 AC 11 — the seed defaults to the 400 mm pair, not to the
/// field's own `[0.0, 0.0]`, which would be a zero-sized bed.
#[test]
fn default_bed_mm_is_the_default_bed() {
    assert_eq!(
        Settings::default().default_bed_mm,
        [DEFAULT_BED_WIDTH_MM, DEFAULT_BED_HEIGHT_MM]
    );
}

/// LCV-114 AC 11 — a settings file written before this demand still loads
/// with a usable bed seed rather than a zero-sized one.
#[test]
fn legacy_json_without_default_bed_mm_uses_the_default_bed() {
    let tmp = std::env::temp_dir().join("lcv114_legacy_bed.json");
    fs::write(&tmp, r#"{"recent_files":[]}"#).unwrap();

    let result = load_from(&tmp);
    assert_eq!(
        result.default_bed_mm,
        [DEFAULT_BED_WIDTH_MM, DEFAULT_BED_HEIGHT_MM]
    );

    let _ = fs::remove_file(&tmp);
}

/// LCV-114 AC 11 — a chosen seed survives a save/load cycle.
#[test]
fn default_bed_mm_round_trips_through_the_file() {
    let tmp = std::env::temp_dir().join("lcv114_bed_seed.json");
    let s = Settings {
        default_bed_mm: [300.0, 180.0],
        ..Settings::default()
    };
    save_to(&s, &tmp).unwrap();

    assert_eq!(load_from(&tmp).default_bed_mm, [300.0, 180.0]);

    let _ = fs::remove_file(&tmp);
}

/// LCV-114 AC 11 — the file is operator-editable, so every reader goes
/// through the clamp rather than trusting the stored pair.
#[test]
fn clamped_default_bed_mm_holds_each_axis_in_range() {
    let mut s = Settings {
        default_bed_mm: [0.0, 5000.0],
        ..Settings::default()
    };
    assert_eq!(s.clamped_default_bed_mm(), [1.0, 2000.0]);
    s.default_bed_mm = [300.0, 180.0];
    assert_eq!(s.clamped_default_bed_mm(), [300.0, 180.0]);
    s.default_bed_mm = [f64::NAN, 180.0];
    assert_eq!(s.clamped_default_bed_mm(), [DEFAULT_BED_WIDTH_MM, 180.0]);
}

// ------------------------------------------------------------------
// LCV-121 — agent model and step budget
// ------------------------------------------------------------------

/// AC 12 — the two new fields default to the OpenRouter-compatible model
/// id and (LCV-142 AC 2) to a budget of 256.
#[test]
fn agent_model_and_step_budget_defaults() {
    let s = Settings::default();
    assert_eq!(s.agent_model, "anthropic/claude-sonnet-4.6");
    assert_eq!(s.agent_step_budget, 256);
}

/// LCV-142 AC 2 — the serde literal is the loop's constant. The import is
/// in the test section only; the implementation still names no agent path.
#[test]
fn the_default_budget_literal_is_the_agent_constant() {
    assert_eq!(
        default_agent_step_budget(),
        crate::agent::AGENT_STEP_BUDGET_DEFAULT
    );
    let s: Settings = serde_json::from_str("{}").expect("an empty object parses");
    assert_eq!(s.agent_step_budget, crate::agent::AGENT_STEP_BUDGET_DEFAULT);
}

/// AC 12 — a settings file written before this demand loads with both new
/// fields defaulted and every older field intact. Field-by-field defaults,
/// not a whole-struct fallback: the endpoint and the recent list below are
/// what prove the file was really read.
#[test]
fn settings_json_predating_this_demand_loads_with_both_defaults() {
    let json = r#"{"recent_files":["foo.lcad","bar.lcad"],
                       "agent_endpoint":"https://api.openai.com/v1",
                       "agent_api_key":"",
                       "default_bed_mm":[300.0,180.0]}"#;
    let s: Settings = serde_json::from_str(json).expect("legacy JSON must still parse");

    assert_eq!(s.agent_model, "anthropic/claude-sonnet-4.6");
    assert_eq!(
        s.agent_step_budget, 256,
        "LCV-142 AC 2: a missing field is 256"
    );
    assert_eq!(s.recent_files, vec!["foo.lcad", "bar.lcad"]);
    assert_eq!(s.agent_endpoint, "https://api.openai.com/v1");
    assert_eq!(s.default_bed_mm, [300.0, 180.0]);
}

/// LCV-153 AC 8 — the context size defaults to the agent constant, and a
/// file without the key loads with it; a stored value is kept verbatim.
#[test]
fn the_context_tokens_default_and_load() {
    assert_eq!(
        default_agent_context_tokens(),
        crate::agent::memory::CONTEXT_TOKENS_DEFAULT
    );
    assert_eq!(Settings::default().agent_context_tokens, 128_000);
    let old: Settings = serde_json::from_str(r#"{"agent_step_budget":12}"#).expect("parses");
    assert_eq!(old.agent_context_tokens, 128_000);
    assert_eq!(old.agent_step_budget, 12);
    let s: Settings = serde_json::from_str(r#"{"agent_context_tokens":5}"#).expect("parses");
    assert_eq!(s.agent_context_tokens, 5);
}

/// AC 13 — the stored budget is kept exactly as written, however silly.
/// Clamping belongs to the reader (`agent::clamp_step_budget`); doing it
/// here would hide what the file actually says. LCV-142 AC 2: an older
/// file's `12` stays 12, and `4096` loads unchanged.
#[test]
fn stored_step_budget_is_kept_verbatim() {
    for (json, expected) in [
        (r#"{"agent_step_budget":12}"#, 12u32),
        (r#"{"agent_step_budget":4096}"#, 4096),
        (r#"{"agent_step_budget":5000}"#, 5000),
        (r#"{"agent_step_budget":0}"#, 0),
        (r#"{"agent_step_budget":7}"#, 7),
    ] {
        let s: Settings = serde_json::from_str(json).expect("must parse");
        assert_eq!(s.agent_step_budget, expected, "{json}");
    }
}

/// AC 12 — both fields survive a serialise/parse cycle with non-default
/// values, so persistence is proved rather than the default being matched
/// tautologically.
#[test]
fn agent_model_and_step_budget_round_trip() {
    let original = Settings {
        agent_model: "some/other-model".into(),
        agent_step_budget: 3,
        ..Settings::default()
    };
    let json = serde_json::to_string(&original).unwrap();
    let loaded: Settings = serde_json::from_str(&json).unwrap();

    assert_eq!(loaded, original);
    assert_ne!(loaded.agent_model, Settings::default().agent_model);
    assert_ne!(
        loaded.agent_step_budget,
        Settings::default().agent_step_budget
    );
}

/// AC 13 — this module must not reach into the agent module: that edge
/// would invert the layering, and it is the reason the default is written
/// as a literal here. Bounded to the implementation section, needle built
/// with `concat!` so the scan cannot match its own source, and carrying a
/// positive control so the absence cannot pass vacuously.
#[test]
fn settings_does_not_import_the_agent() {
    let src = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/io/settings.rs"));
    let at = src
        .find("\n#[cfg(test)]")
        .expect("settings.rs must have a bare #[cfg(test)] marker");
    let implementation = &src[..at];

    assert!(
        implementation.contains(concat!("agent_step", "_budget")),
        "positive control: the scanned slice must contain the new field"
    );
    assert!(
        !implementation.contains(concat!("crate::", "agent")),
        "io::settings must not import the agent module (ADR 0007 §D7)"
    );
}

/// LCV-145 AC 2 — a settings file written before the two canvas opt-ins
/// existed loads both as `false`; set values survive a round trip.
#[test]
fn canvas_opt_ins_default_off_and_round_trip() {
    let old: Settings =
        serde_json::from_str(r#"{"agent_model":"m","agent_step_budget":9}"#).unwrap();
    assert!(!old.agent_allow_canvas_capture);
    assert!(!old.agent_model_supports_vision);
    assert!(!Settings::default().agent_allow_canvas_capture);
    assert!(!Settings::default().agent_model_supports_vision);

    let on = Settings {
        agent_allow_canvas_capture: true,
        agent_model_supports_vision: true,
        ..Settings::default()
    };
    let back: Settings = serde_json::from_str(&serde_json::to_string(&on).unwrap()).unwrap();
    assert_eq!(back, on);
}

/// LCV-195 AC 6 — a settings file written before `Feedback after changes`
/// existed loads it off, as does the default; a set value round-trips.
#[test]
fn feedback_after_changes_defaults_off_and_round_trips() {
    let old: Settings =
        serde_json::from_str(r#"{"agent_model":"m","agent_allow_canvas_capture":true}"#).unwrap();
    assert!(!old.agent_feedback_after_changes);
    assert!(!Settings::default().agent_feedback_after_changes);

    let on = Settings {
        agent_feedback_after_changes: true,
        ..Settings::default()
    };
    let json = serde_json::to_string(&on).unwrap();
    assert!(
        json.contains(r#""agent_feedback_after_changes":true"#),
        "{json}"
    );
    let back: Settings = serde_json::from_str(&json).unwrap();
    assert_eq!(back, on);
}

// ------------------------------------------------------------------
// LCV-161 — per-kind object snaps
// ------------------------------------------------------------------

/// LCV-161 AC10 — a settings file without `object_snaps` loads the default
/// kinds (all on except Nearest).
#[test]
fn legacy_json_without_object_snaps_loads_default_kinds() {
    use crate::geometry::SnapKinds;
    let s: Settings = serde_json::from_str(r#"{"recent_files":[],"agent_step_budget":12}"#)
        .expect("legacy JSON must still parse");
    assert_eq!(s.object_snaps, SnapKinds::default());
    // A partial `object_snaps` object fills the missing kinds with defaults.
    let s: Settings =
        serde_json::from_str(r#"{"object_snaps":{"nearest":true}}"#).expect("partial parses");
    assert!(s.object_snaps.nearest);
    assert!(s.object_snaps.endpoint);
}

/// LCV-161 AC8 — a toggled kind survives a save/load round-trip.
#[test]
fn object_snaps_round_trip_keeps_a_toggled_kind() {
    use crate::geometry::SnapKind;
    let tmp = std::env::temp_dir().join("lcv161_object_snaps_roundtrip.json");
    let mut original = Settings::default();
    original.object_snaps.set(SnapKind::Tangent, false);
    original.object_snaps.set(SnapKind::Nearest, true);
    save_to(&original, &tmp).unwrap();
    let loaded = load_from(&tmp);
    assert!(!loaded.object_snaps.contains(SnapKind::Tangent));
    assert!(loaded.object_snaps.contains(SnapKind::Nearest));
    assert_eq!(loaded, original);
    let _ = fs::remove_file(&tmp);
}
