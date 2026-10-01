use super::*;
// ── AC 13: the fence ───────────────────────────────────────────────────

/// AC 13 — the revision the turn started at is the revision it expects,
/// and checking it repeatedly does not drift.
#[test]
fn a_matching_revision_passes() {
    let mut fence = TurnFence::new(10);
    assert_eq!(fence.check(10, true), Ok(()));
    assert_eq!(fence.check(10, true), Ok(()));
    assert!(!fence.is_tripped());
}

/// AC 13 — a revision that moved without the turn's knowledge is refused
/// with the ADR 0007 §D4 wording, character for character.
#[test]
fn a_foreign_revision_is_refused_with_the_adr_wording() {
    let mut fence = TurnFence::new(10);
    assert_eq!(fence.check(11, true), Err(AGENT_FENCE_REFUSAL.to_string()));
    assert!(AGENT_FENCE_REFUSAL.contains("Nothing was applied"));
    assert!(AGENT_FENCE_REFUSAL.contains("Undo is unaffected"));
}

/// AC 13 — the trip is **sticky**: after one mismatch, the revision the
/// fence used to want is refused too. A fence that recomputed per call
/// would answer `Ok` on the second line here (mutation (d)).
#[test]
fn a_tripped_fence_refuses_even_a_matching_revision() {
    let mut fence = TurnFence::new(10);
    assert!(fence.check(11, true).is_err());
    assert_eq!(fence.check(10, true), Err(AGENT_FENCE_REFUSAL.to_string()));
    assert_eq!(fence.check(11, true), Err(AGENT_FENCE_REFUSAL.to_string()));
    assert!(fence.is_tripped());
}

/// AC 13 — `advance` moves the expectation, so the revision the last apply
/// produced becomes the one the next action requires. Before the advance
/// that same revision would have tripped the fence.
#[test]
fn advance_moves_the_expectation() {
    let mut fence = TurnFence::new(10);
    fence.advance(11);
    assert_eq!(fence.check(11, true), Ok(()));
    assert!(
        fence.check(10, true).is_err(),
        "a rewound revision is still foreign"
    );
}

/// LCV-142 AC 9 (ADR 0007 §D14) — the second witness: a matching revision
/// with no group open trips the fence, and the trip is sticky even once a
/// group is open again. This is the revision-0 replacement hole.
#[test]
fn a_closed_group_trips_the_fence_even_on_a_matching_revision() {
    let mut fence = TurnFence::new(0);
    assert_eq!(fence.check(0, false), Err(AGENT_FENCE_REFUSAL.to_string()));
    assert!(fence.is_tripped());
    assert_eq!(fence.check(0, true), Err(AGENT_FENCE_REFUSAL.to_string()));
}

// ── AC 3: arming, and refusing to arm twice ──────────────────────────────

/// AC 3 — arming records the operator's words, raises the busy flag, parks
/// a live `Receiver` on `App`, and hands back the `Sender` that feeds it.
///
/// The `Sender` is exercised rather than merely returned: a channel whose
/// other half never reached `App` would still type-check here.
#[test]
fn arm_turn_records_the_user_row_and_arms_a_live_channel() {
    let mut app = App::default();
    let tx = arm_turn(&mut app, "draw a 20 mm square");

    assert_eq!(
        app.agent.chat,
        vec![("user".to_owned(), "draw a 20 mm square".to_owned())]
    );
    assert!(app.agent.busy);
    assert!(app.agent.rx.is_some());
    assert_eq!(app.agent.turn.tally.applied, 0);
    assert_eq!(app.agent.turn.label, "AI: draw a 20 mm square");
    assert!(app.history.group_open(), "the turn's group is open");

    tx.send(AgentEvent::done("hi"))
        .expect("the returned Sender must reach the Receiver on App");
    match app
        .agent
        .rx
        .as_ref()
        .expect("armed")
        .try_recv()
        .expect("and the event must arrive on the Receiver App is holding")
    {
        AgentEvent::Done(text, _) => assert_eq!(text, "hi"),
        _ => panic!("the event must arrive intact"),
    }
}

/// AC 3 — the fence is armed at the revision the turn starts from, not at
/// zero. A fence built on `0` would refuse the very first action of any
/// turn in a session that had ever drawn anything.
#[test]
fn arm_turn_anchors_the_fence_on_the_current_revision() {
    let mut app = App::default();
    app.commit(Box::new(crate::document::CreateLine::new(
        crate::geometry::Line::new(
            crate::geometry::Vec2::new(0.0, 0.0),
            crate::geometry::Vec2::new(1.0, 0.0),
        ),
    )));
    let revision = app.history.revision();
    assert_ne!(revision, 0, "the fixture must have moved the revision");

    let _tx = arm_turn(&mut app, "go on");
    assert_eq!(app.agent.turn.fence, TurnFence::new(revision));
    assert_eq!(app.agent.turn.fence.check(revision, true), Ok(()));
}

/// AC 3 — a second turn is refused while one is in flight, and refused
/// *silently*: `start_turn` touches nothing. One `agent.rx` and one fence
/// mean one turn, and re-arming would strand the running thread.
///
/// Asserted on a busy `App` whose channel was armed by hand, so no thread
/// and no socket are involved.
#[test]
fn start_turn_on_a_busy_app_changes_nothing() {
    let mut app = App::default();
    let _tx = arm_turn(&mut app, "the first prompt");
    let chat = app.agent.chat.clone();
    let fence = app.agent.turn.fence.clone();
    let label = app.agent.turn.label.clone();
    let armed = app.agent.rx.as_ref().map(std::ptr::from_ref);

    start_turn(&mut app, "the second prompt");

    assert_eq!(app.agent.chat, chat, "no row for a turn that never started");
    assert_eq!(
        app.agent.turn.fence, fence,
        "the running turn keeps its fence"
    );
    assert_eq!(app.agent.turn.label, label);
    assert_eq!(
        app.agent.rx.as_ref().map(std::ptr::from_ref),
        armed,
        "the running turn keeps its receiver"
    );
    assert!(app.agent.busy);
}

// ── AC 18: the budget is read and clamped here ───────────────────────────

/// AC 18 / LCV-142 AC 2 — the stored budget is clamped at this read site,
/// and the armed turn snapshots the clamped value.
#[test]
fn the_budget_is_clamped_where_it_is_read() {
    for (stored, expected) in [
        (0, 1),
        (1, 1),
        (12, 12),
        (256, 256),
        (4096, 4096),
        (4097, 4096),
        (u32::MAX, 4096),
    ] {
        let mut app = App::default();
        app.settings.agent_step_budget = stored;
        assert_eq!(effective_step_limit(&app.settings), expected, "{stored}");
        assert_eq!(turn_config(&app.settings).step_limit, expected);
        let _tx = arm_turn(&mut app, "x");
        assert_eq!(app.agent.turn.limit, expected, "stored {stored}");
    }
}

/// LCV-142 AC 11 — the config is a snapshot: built from a budget of 7, it
/// still says 7 after the settings change to 9, and so does the armed
/// turn's limit.
#[test]
fn the_turn_config_is_a_turn_start_snapshot() {
    let mut app = App::default();
    app.settings.agent_step_budget = 7;
    app.settings.agent_model = "m/one".to_owned();
    let config = turn_config(&app.settings);
    let _tx = arm_with_limit(&mut app, "x", config.step_limit);

    app.settings.agent_step_budget = 9;
    app.settings.agent_model = "m/two".to_owned();

    assert_eq!(config.step_limit, 7);
    assert_eq!(config.model, "m/one");
    assert_eq!(app.agent.turn.limit, 7);
    assert_eq!(turn_config(&app.settings).step_limit, 9, "the next turn");
}

/// LCV-150 AC 4 — after a clear, the next armed turn's config carries no
/// memory, so the worker sends `[system, user]` (LCV-153 AC 11).
#[test]
fn a_turn_after_a_clear_carries_no_memory() {
    use crate::agent::wire::ChatMessage;
    let mut app = App::default();
    for prompt in ["one", "two"] {
        app.agent.memory.push_turn(vec![
            ChatMessage::user(prompt),
            ChatMessage::assistant("ok"),
        ]);
    }
    assert_eq!(config_for(&app).memory.len(), 4, "remembered before");

    app.agent.clear_conversation();
    let _tx = arm_turn(&mut app, "fresh");

    assert!(config_for(&app).memory.is_empty());
}

/// LCV-145 AC 2 — `vision` is on only when both opt-ins are on at arm
/// time, and a later flip leaves the armed turn's snapshot alone.
#[test]
fn the_turn_config_vision_flag_needs_both_opt_ins() {
    let mut app = App::default();
    for (allow, supports) in [(false, false), (true, false), (false, true), (true, true)] {
        app.settings.agent_allow_canvas_capture = allow;
        app.settings.agent_model_supports_vision = supports;
        assert_eq!(turn_config(&app.settings).vision, allow && supports);
    }
    let armed = turn_config(&app.settings);
    app.settings.agent_model_supports_vision = false;
    assert!(armed.vision, "the armed turn keeps its snapshot");
    assert!(
        !turn_config(&app.settings).vision,
        "the next turn sees the flip"
    );
}

/// LCV-143 AC 5 — the prompt is resolved once, into the config: built
/// with override A, the config still carries A verbatim after the
/// settings move to B, and later configs carry B, then the default.
#[test]
fn the_turn_config_snapshots_the_resolved_prompt() {
    let mut app = App::default();
    let a = "  prompt A\n\twith edges  ";
    app.settings.agent_system_prompt = Some(a.to_owned());
    let first = turn_config(&app.settings);

    app.settings.agent_system_prompt = Some("prompt B".to_owned());
    let second = turn_config(&app.settings);
    app.settings.agent_system_prompt = None;
    let third = turn_config(&app.settings);

    assert_eq!(first.system_prompt, a, "the armed turn keeps A verbatim");
    assert_eq!(second.system_prompt, "prompt B");
    assert_eq!(third.system_prompt, crate::agent::DEFAULT_PROMPT);
    app.settings.agent_system_prompt = Some(String::new());
    assert_eq!(
        turn_config(&app.settings).system_prompt,
        "",
        "blank is kept"
    );
}

/// AC 18 — the settings module knows nothing about the agent. The clamp
/// belongs to the reader, not to the store: `io/settings.rs` deserialises
/// whatever the JSON says and hands it over unjudged, which is why the
/// clamp above has to exist.
#[test]
fn the_settings_module_does_not_import_the_agent() {
    let src = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/io/settings.rs"));
    let at = src
        .find("\n#[cfg(test)]")
        .expect("settings.rs must have a bare #[cfg(test)] marker");
    let implementation = &src[..at];
    assert!(
        implementation.contains(concat!("pub agent_step", "_budget: u32")),
        "positive control: settings must still hold the stored budget"
    );
    let needle = concat!("crate::", "agent");
    assert!(
        concat!("use crate::", "agent::clamp_step_budget;").contains(needle),
        "control: the needle must match a real import"
    );
    let hit = implementation
        .lines()
        .find(|l| l.contains(needle) && !l.trim_start().starts_with("//"));
    assert!(
        hit.is_none(),
        "AC 18: io/settings.rs must not reach into the agent: {hit:?}"
    );
}

// ── AC 10: the undo label ────────────────────────────────────────────────

/// AC 10 — the label is `AI:` plus the trimmed prompt, cut at 40
/// characters with an `…` only when something was actually cut.
#[test]
fn the_turn_label_trims_and_truncates_at_forty_characters() {
    assert_eq!(turn_label("  draw a square  "), "AI: draw a square");
    // Exactly 40 characters: kept whole, no ellipsis.
    let forty = "a".repeat(40);
    assert_eq!(turn_label(&forty), format!("AI: {forty}"));
    // Forty-one: forty kept, one ellipsis.
    let forty_one = "b".repeat(41);
    assert_eq!(turn_label(&forty_one), format!("AI: {}…", "b".repeat(40)));
}

/// AC 10 — the cut counts `char`s, not bytes. Slicing this prompt at byte
/// 40 lands inside a multi-byte character and panics.
#[test]
fn the_turn_label_cuts_on_character_boundaries() {
    let prompt = "desenhe um quadrado de vinte milímetros no canto";
    let label = turn_label(prompt);
    assert!(label.ends_with('…'), "{label}");
    assert_eq!(label.chars().count(), "AI: ".len() + 40 + 1);
    assert!(
        label.starts_with("AI: desenhe um quadrado de vinte milí"),
        "{label}"
    );
}

/// AC 12 — arming, spawning and finishing a turn open no dialog and block
/// nothing. The operator keeps drawing while the agent works.
///
/// Every `*_open` flag on `App` is a modal or a panel; a turn that set one
/// would take the canvas away, which ADR 0007 §D4 names a non-goal. The
/// scan covers every file that runs turn code.
#[test]
fn no_turn_function_opens_a_dialog() {
    let witness = "app.agent_settings_open = true; app.agent.panel_open = false;";
    for (name, src) in [
        (
            "agent_turn.rs",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/app/agent_turn.rs"
            )),
        ),
        (
            "agent_worker.rs",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/app/agent_worker.rs"
            )),
        ),
        (
            "agent_poll.rs",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/app/agent_poll.rs"
            )),
        ),
        (
            "agent_poll/turn_end.rs",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/app/agent_poll/turn_end.rs"
            )),
        ),
    ] {
        // `turn_end.rs` has no test module: all of it is implementation.
        let at = src.find("\n#[cfg(test)]").unwrap_or(src.len());
        let needle = concat!("_open", " =");
        assert!(
            witness.contains(needle),
            "control: `{needle}` must match a real write"
        );
        let hit = src[..at]
            .lines()
            .find(|l| l.contains(needle) && !l.trim_start().starts_with("//"));
        assert!(
            hit.is_none(),
            "AC 12: {name} must not write a dialog flag: {hit:?}"
        );
    }
}

/// AC 15 / ADR 0007 §D4 — the fence stays where the thread cannot reach
/// it, and this file stays out of the UI toolkit.
///
/// Bounded at the bare `#[cfg(test)]` at column 0, needles built with
/// `concat!`. The positive control names the one declaration that must be
/// here, so a mis-sliced haystack fails instead of passing vacuously.
#[test]
fn the_fence_is_declared_here_and_this_file_imports_no_ui() {
    let src = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/app/agent_turn.rs"
    ));
    let at = src
        .find("\n#[cfg(test)]")
        .expect("agent_turn.rs must have a bare #[cfg(test)] marker");
    let implementation = &src[..at];

    assert!(
        implementation.contains(concat!("pub struct ", "TurnFence")),
        "positive control: the fence must be declared in this file"
    );
    for forbidden in [
        concat!("e", "frame"),
        concat!("r", "fd"),
        concat!("use e", "gui"),
    ] {
        let hit = implementation
            .lines()
            .find(|l| l.contains(forbidden) && !l.trim_start().starts_with("//"));
        assert!(
            hit.is_none(),
            "agent_turn.rs must not name `{forbidden}`: {hit:?}"
        );
    }
}
