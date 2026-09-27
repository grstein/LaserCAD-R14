//! LCV-125 — the panel shows what the agent did, and the settings dialog
//! reaches the fields that were already wired.
//!
//! ## What these tests can and cannot prove
//!
//! `egui` 0.29.1 has no way to assert a rendered *pixel* (`egui_kittest` needs
//! ≥ 0.30 and is out of scope, ADR 0002). It does have a way to assert a
//! rendered *string*: `Context::run` returns the paint list, and every
//! `Shape::Text` in it carries the exact text and the position it was laid out
//! at. So every criterion about appearance is covered in four parts, and the
//! test names say which part they are:
//!
//! - a **bounded source scan** in the module under test, pinning the call that
//!   produces the appearance — `src/agent/panel.rs`, `src/agent/settings_ui.rs`
//!   and `src/app/panels.rs` carry theirs inline;
//! - a **headless frame** here, proving the path runs on a real `App` without
//!   panicking and leaves the state it is supposed to leave;
//! - a **paint-list assertion** here, built on `tests/harness/paint.rs`,
//!   proving the call it pinned actually ran and put those strings on the
//!   screen in that order.
//!   A scan cannot do this: the loop between `agent.chat` and `draw_chat_row`
//!   is a call no scan observes, and wrapping it, reversing it or skipping a
//!   role leaves every scan in this demand green;
//! - the reviewer's eye and the manual smoke, which are the actual acceptance
//!   for "is it legible" — colour and font size are still not asserted here.
//!
//! No test here reaches the network: a turn is driven through LCV-123's
//! `arm_turn` seam, which hands back the `Sender` the worker thread would have
//! held. ADR 0002 §A2: `App::default()` only. ADR 0006 / ADR 0007 §D10: every
//! `App` leaves `settings_path` at `None`, except the one AC 12 test that
//! points it at a directory under the system temp dir that it creates and owns.

mod harness;

use harness::frame;
use harness::paint::{lines_on_surface_of, painted_runs, texts};
use harness::scan::rs_files;
use lasercad::agent::{AgentAction, AgentEvent, AgentOutcome};
use lasercad::app::{arm_turn, App};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};

/// A recognisable key that must never appear anywhere the operator can read.
/// Built with `concat!` so a grep for the whole string finds no copy of it.
const DUMMY_KEY: &str = concat!("sk-test-", "DO-NOT-LEAK");

/// A marker no other string in the application carries, so a painted run
/// carrying it can only have come from a transcript row this test pushed.
const ROW_MARK: &str = "LCV125ROW";

/// The six roles LCV-123 writes, plus one this build does not know.
const EVERY_ROLE: [&str; 7] = [
    "user",
    "tool",
    "refused",
    "assistant",
    "error",
    "note",
    "a-role-from-the-future",
];

// ── Plumbing ────────────────────────────────────────────────────────────────

/// A headless egui context and an `App` with no injected paths.
fn ctx_and_app() -> (egui::Context, App) {
    let ctx = egui::Context::default();
    ctx.set_pixels_per_point(1.0);
    let app = App::default();
    assert!(
        app.settings_path.is_none(),
        "ADR 0006: no real settings path"
    );
    assert!(
        app.autosave_path.is_none(),
        "ADR 0006: no real autosave path"
    );
    (ctx, app)
}

fn idle(ctx: &egui::Context, app: &mut App) {
    frame(ctx, app, Vec::new());
}

/// Push one `Act` and hand back the reply `Receiver`, the worker's half of
/// ADR 0007 §D3 written by hand.
fn push_act(tx: &Sender<AgentEvent>, action: AgentAction) -> Receiver<AgentOutcome> {
    let (reply, answer) = channel::<AgentOutcome>();
    tx.send(AgentEvent::Act { action, reply })
        .expect("the armed Receiver must still be on App");
    answer
}

fn roles(app: &App) -> Vec<&str> {
    app.agent.chat.iter().map(|(r, _)| r.as_str()).collect()
}

/// A private, empty directory under the system temp dir, named after the test
/// that owns it so parallel tests never share one.
fn tempdir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lcv125_{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the system temp dir must be writable");
    dir
}

// ── AC 1: every role renders, including one nobody wrote ────────────────────

/// AC 1 — **headless frame**: one row of each of the six roles plus one role
/// this build has never heard of, rendered through the real panel in the real
/// frame. What it proves is that no arm panics and that the `_` fallback is
/// reached; what the six *look like* is the inline scans plus the smoke.
///
/// It also pins the seam from the rendering side: the panel appends nothing.
/// `agent.chat` comes out of the frame exactly as it went in.
#[test]
fn ac1_every_role_including_an_unknown_one_renders_headless_frame() {
    let (ctx, mut app) = ctx_and_app();
    app.agent.panel_open = true;
    for role in EVERY_ROLE {
        app.agent
            .chat
            .push((role.to_owned(), format!("a {role} row")));
    }
    let before = app.agent.chat.clone();

    idle(&ctx, &mut app);
    idle(&ctx, &mut app);

    assert_eq!(
        app.agent.chat, before,
        "LCV-125 renders LCV-123's rows and writes none of its own"
    );
    assert!(app.agent.panel_open, "the panel stayed open");
    assert!(app.error_message.is_none(), "no modal was raised");
}

/// AC 2 / AC 4 / AC 5 — **painted output**: the seven rows really reach the
/// screen, one line each, in `agent.chat` order, directly under the panel
/// heading, carrying the markers the tool and refused rows are supposed to
/// carry.
///
/// This is the assertion the source scans cannot make. Every scan in this
/// demand pins a call that is *written*; the loop at `src/agent/panel.rs` that
/// feeds `draw_chat_row` is a call nothing observed, and wrapping it in a
/// `horizontal_wrapped`, iterating it with `.rev()`, or skipping a role inside
/// it leaves the whole suite green. Each of those turns this vector into a
/// different vector.
///
/// Anchored on the heading rather than on the row text, so a line appearing
/// *between* two rows fails too.
#[test]
fn ac2_ac4_ac5_the_seven_rows_are_painted_one_line_each_in_order() {
    let (ctx, mut app) = ctx_and_app();
    app.agent.panel_open = true;
    for role in EVERY_ROLE {
        app.agent
            .chat
            .push((role.to_owned(), format!("{ROW_MARK}-{role}")));
    }

    // Two frames: egui sizes a layout on the first and paints it settled on the
    // second.
    let _ = painted_runs(&ctx, &mut app);
    let runs = painted_runs(&ctx, &mut app);
    let marked = runs.iter().filter(|r| r.text.contains(ROW_MARK)).count();
    assert_eq!(
        marked, 7,
        "seven rows went in, so exactly seven marked runs must be painted"
    );

    let lines = lines_on_surface_of(&runs, "AI Assistant");
    assert_eq!(
        lines.len(),
        9,
        "the panel paints its heading, seven rows and the prompt row, and \
         nothing else: {:?}",
        texts(&lines)
    );
    assert_eq!(
        lines[0].1.first().map(String::as_str),
        Some("AI Assistant"),
        "the heading is the top line"
    );

    let rows = &lines[1..8];
    assert_eq!(
        texts(rows),
        [
            vec![format!("{ROW_MARK}-user")],
            vec![format!("▸ {ROW_MARK}-tool")],
            vec![format!("⚠ {ROW_MARK}-refused")],
            vec![format!("{ROW_MARK}-assistant")],
            vec![format!("{ROW_MARK}-error")],
            vec![format!("{ROW_MARK}-note")],
            vec![format!("{ROW_MARK}-a-role-from-the-future")],
        ],
        "each row on its own line, directly under the heading, in transcript \
         order, carrying its own marker"
    );

    // AC 4 — the note is set off by a separator, so the gap above it is wider
    // than the gap between two ordinary rows. Compared, never hard-coded: the
    // absolute numbers are a function of the theme's font size.
    let gap = |a: usize, b: usize| rows[b].0 - rows[a].0;
    assert!(
        gap(4, 5) > gap(3, 4),
        "the note must sit below a separator: gaps were {} then {}",
        gap(3, 4),
        gap(4, 5)
    );
}

/// AC 5 — **headless frame**: a 300-character outcome goes through the real
/// panel without panicking. That it *wraps* rather than widening the panel is
/// asserted for real in `src/agent/panel.rs`'s
/// `ac5_a_three_hundred_character_row_wraps_within_the_width`, which measures
/// the row against a bounded width.
#[test]
fn ac5_a_three_hundred_character_row_renders_headless_frame() {
    let (ctx, mut app) = ctx_and_app();
    app.agent.panel_open = true;
    let long = "outcome ".repeat(38);
    assert!(long.len() >= 300);
    app.agent.chat.push(("tool".to_owned(), long.clone()));
    app.agent.chat.push(("tool".to_owned(), long.clone()));

    idle(&ctx, &mut app);

    assert_eq!(app.agent.chat.len(), 2, "two rows in, two rows out");
}

// ── AC 6: the key reaches nothing the operator reads ────────────────────────

/// AC 6 — a whole turn with a recognisable key set, driven through `arm_turn`
/// with the panel open so every row is really rendered: the key appears in no
/// transcript row. No socket, no thread, no endpoint — LCV-123's seam is the
/// whole of the wiring.
#[test]
fn ac6_the_api_key_reaches_no_transcript_row() {
    let (ctx, mut app) = ctx_and_app();
    app.settings.agent_api_key = DUMMY_KEY.to_owned();
    app.agent.panel_open = true;

    let tx = arm_turn(&mut app, "draw a 20 mm line and delete entity 5");
    let answer = push_act(
        &tx,
        AgentAction::CreateLine {
            x1: 0.0,
            y1: 0.0,
            x2: 20.0,
            y2: 0.0,
        },
    );
    idle(&ctx, &mut app);
    assert!(
        matches!(answer.recv(), Ok(AgentOutcome::Ok(_))),
        "the line must really land"
    );

    let answer = push_act(&tx, AgentAction::Delete { index: 5 });
    idle(&ctx, &mut app);
    assert!(
        matches!(answer.recv(), Ok(AgentOutcome::Refused(_))),
        "index 5 of a one-entity drawing is a refusal"
    );

    tx.send(AgentEvent::Done("Drew one line.".to_owned()))
        .expect("the armed Receiver must still be on App");
    idle(&ctx, &mut app);

    assert!(!app.agent.busy, "the turn ended");
    assert_eq!(
        roles(&app),
        ["user", "tool", "refused", "assistant", "note"],
        "the turn really produced the rows this demand renders"
    );
    for (role, content) in &app.agent.chat {
        assert!(
            !content.contains(DUMMY_KEY),
            "the key leaked into a `{role}` row: {content}"
        );
    }
    assert!(
        !app.command_feedback.contains(DUMMY_KEY),
        "and none into the command line"
    );
}

// ── AC 8 / AC 9 / AC 10: the settings dialog draws its four fields ──────────

/// AC 8 / AC 9 / AC 10 — **headless frame**: the four-field form draws inside
/// the real `Agent Settings` window, through `src/app/panels.rs`, and an idle
/// frame changes nothing. Which fields exist and what the warning says are the
/// bounded scans in `src/agent/settings_ui.rs`.
#[test]
fn ac8_ac9_ac10_the_settings_dialog_draws_headless_frame() {
    let (ctx, mut app) = ctx_and_app();
    app.agent_settings_open = true;
    let before = app.settings.clone();

    idle(&ctx, &mut app);
    idle(&ctx, &mut app);

    assert!(app.agent_settings_open, "the window stayed open");
    assert_eq!(
        app.settings, before,
        "an idle frame must not edit the operator's settings"
    );
}

/// AC 8 / AC 9 / AC 10 — **painted output**: the form's four controls, both
/// long sentences and every default value really reach the screen, in order,
/// verbatim, directly under the window title.
///
/// Two of these are invisible to every source scan. `warn_label(…)` and the
/// step-budget label are unconditional calls, so a guard in front of either —
/// `if !settings.agent_api_key.is_empty()` is the obvious one — keeps the call
/// written, keeps the scan green, and takes the sentence off the screen on
/// exactly the frame the operator is about to paste a key into an empty field.
/// AC 10 says *always visible*, and this is what says it.
///
/// The second phase is AC 8's masking: a key that is set paints as bullets, and
/// the key itself is painted nowhere at all.
///
/// **LCV-141 note**: the expected set below gained its last two lines — the
/// live-edit sentence and the Done button — the two additions that demand
/// makes to this exact dialog (its AC 6). ADR 0009 decision 2 is why the fix
/// is here rather than around it: a paint assertion that claims a surface
/// shows *nothing else* must be updated the moment intentional content is
/// added, or it certifies the old screen as still correct.
#[test]
fn ac8_ac9_ac10_the_form_paints_its_fields_and_both_sentences() {
    let (ctx, mut app) = ctx_and_app();
    app.agent_settings_open = true;

    let _ = painted_runs(&ctx, &mut app);
    let runs = painted_runs(&ctx, &mut app);
    // Anchored on a field label, not on the window title: a `Window`'s title is
    // clipped to the whole screen, so it names no surface.
    let form = lines_on_surface_of(&runs, "Endpoint URL");

    assert_eq!(
        texts(&form),
        [
            vec!["Endpoint URL", "https://openrouter.ai/api/v1"],
            vec!["Model", "anthropic/claude-sonnet-4.6"],
            // The key field is empty, so nothing readable is painted beside it.
            vec!["API Key"],
            vec![concat!(
                "The API key is stored in plain text in settings.json. ",
                "Anyone who can read that file can read your key."
            )],
            vec!["Steps per turn", "12"],
            vec![concat!(
                "How many tool calls one prompt may make. More steps means a ",
                "bigger drawing per prompt, and more API calls."
            )],
            vec!["Changes apply immediately and are saved when this window closes."],
            vec!["Done"],
        ],
        "the form paints its labels, its values and both sentences, in order, \
         and paints nothing else"
    );

    // AC 8 — a key that is set is masked, and is painted nowhere at all.
    let (ctx, mut app) = ctx_and_app();
    app.agent_settings_open = true;
    app.settings.agent_api_key = DUMMY_KEY.to_owned();

    let _ = painted_runs(&ctx, &mut app);
    let runs = painted_runs(&ctx, &mut app);
    for run in &runs {
        assert!(
            !run.text.contains(DUMMY_KEY),
            "the key must never be painted: {:?}",
            run.text
        );
    }
    let form = lines_on_surface_of(&runs, "API Key");
    let key_row = &form[2].1;
    assert_eq!(
        key_row.len(),
        2,
        "the key field paints a masked value: {key_row:?}"
    );
    assert!(
        key_row[1].chars().count() == DUMMY_KEY.chars().count()
            && !key_row[1].chars().any(|c| DUMMY_KEY.contains(c)),
        "and what it paints is a mask, one glyph per character: {:?}",
        key_row[1]
    );
}

/// AC 12 — the hand-edited file, end to end: a settings file saying `200` is
/// clamped to `32` by opening the dialog, and the clamped value is what the
/// close path writes back. The read site still agrees.
///
/// The close is `App::persist_settings`, called here directly because that is
/// the whole of what `src/app/panels.rs::agent_settings_dialog` runs when the
/// window closes — the branch above it (`was_open && !agent_settings_open`)
/// needs a click on egui's × in the window's title bar, whose position is a
/// function of the window's laid-out rect. Asserting on that geometry would be
/// a brittle test of egui's chrome, not of this demand.
///
/// The only test in this file with an injected `settings_path`, and it points
/// at a directory it created under the system temp dir (ADR 0006). The API key
/// stays empty: nothing here writes a credential to disk.
#[test]
fn ac12_a_hand_edited_budget_is_clamped_by_opening_the_dialog() {
    let dir = tempdir("budget_clamp");
    let path = dir.join("settings.json");
    let ctx = egui::Context::default();
    ctx.set_pixels_per_point(1.0);
    let mut app = App {
        settings_path: Some(path.clone()),
        agent_settings_open: true,
        ..App::default()
    };
    app.settings.agent_step_budget = 200;
    assert!(app.settings.agent_api_key.is_empty(), "no key is written");

    // Opened…
    idle(&ctx, &mut app);
    idle(&ctx, &mut app);
    assert_eq!(
        app.settings.agent_step_budget, 32,
        "the slider shows the clamped value"
    );

    // …and closed, which is what persists it (LCV-119).
    app.agent_settings_open = false;
    app.persist_settings();

    let written = std::fs::read_to_string(&path).expect("closing the dialog must write the file");
    let parsed: serde_json::Value =
        serde_json::from_str(&written).expect("the settings file must be JSON");
    assert_eq!(
        parsed["agent_step_budget"], 32,
        "the file keeps the clamped value, not the 200 it was hand-edited to"
    );
    assert_eq!(
        lasercad::agent::clamp_step_budget(app.settings.agent_step_budget),
        32,
        "and the read site agrees (LCV-123 AC 18)"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// ── AC 14: the two egui files under src/agent/ are still exactly two ────────

/// Every file under `src/agent/` as (relative path, implementation code)
/// pairs, sorted on the rendered path.
///
/// The implementation section stops at the bare `#[cfg(test)]` at column 0 and
/// comment lines are dropped, so these scans are about what the compiler sees:
/// `mod.rs`'s header names `eframe` and `rfd` precisely to say that no file
/// here may import them, and a scan that counted prose would force the rule to
/// go undocumented to stay true.
///
/// Paths are rebuilt from `components()` joined with `/` — `Path::display()`
/// emits `\` on Windows and has broken this repository's CI twice.
fn agent_sections() -> Vec<(String, String)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("agent");
    let mut files = Vec::new();
    rs_files(&root, &mut files);
    assert!(
        files.len() >= 9,
        "positive control: the walk over src/agent must see the whole module, saw {}",
        files.len()
    );

    let mut out: Vec<(String, String)> = files
        .iter()
        .map(|path| {
            let relative = path
                .strip_prefix(&root)
                .expect("every walked file is under src/agent")
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            let src = std::fs::read_to_string(path).expect("a readable source file");
            let implementation = match src.find("\n#[cfg(test)]") {
                Some(at) => &src[..at],
                None => &src[..],
            };
            let code = implementation
                .lines()
                .filter(|l| !l.trim_start().starts_with("//"))
                .collect::<Vec<_>>()
                .join("\n");
            (relative, code)
        })
        .collect();
    out.sort();
    out
}

fn files_naming(sections: &[(String, String)], needle: &str) -> Vec<String> {
    sections
        .iter()
        .filter(|(_, code)| code.contains(needle))
        .map(|(path, _)| path.clone())
        .collect()
}

/// AC 14 — **source scan** over the whole `src/agent/` tree: the set of files
/// that name `egui` is exactly the two ADR 0007 §D8 allows, and no file names
/// `eframe` or `rfd` at all.
///
/// An equality on the set rather than a grandfather list: a new file under
/// `src/agent/` that reaches for `egui` fails here by name, and so does a file
/// that stops needing it.
#[test]
fn ac14_only_panel_and_settings_ui_import_egui_source_scan() {
    let sections = agent_sections();
    let egui = concat!("eg", "ui");
    assert_eq!(
        files_naming(&sections, egui),
        ["panel.rs", "settings_ui.rs"],
        "exactly two files under src/agent/ may import {egui}"
    );

    let witness = "use eframe::egui; let file = rfd::FileDialog::new();";
    for forbidden in [concat!("efr", "ame"), concat!("rf", "d")] {
        assert!(
            witness.contains(forbidden),
            "control: `{forbidden}` must be a needle that can match something"
        );
        assert_eq!(
            files_naming(&sections, forbidden),
            Vec::<String>::new(),
            "no file under src/agent/ may name `{forbidden}`"
        );
    }
}

/// AC 14 — the 300-LOC cap, counted the one way ADR 0004 allows: total lines
/// minus the inline `#[cfg(test)] mod tests` block, anchored on a **bare**
/// `#[cfg(test)]` at column 0. Never `wc -l`, which has produced a false
/// blocking finding on this repository.
#[test]
fn ac14_both_egui_files_are_under_the_loc_cap() {
    for (name, src) in [
        ("panel.rs", include_str!("../src/agent/panel.rs")),
        (
            "settings_ui.rs",
            include_str!("../src/agent/settings_ui.rs"),
        ),
    ] {
        let at = src
            .find("\n#[cfg(test)]")
            .unwrap_or_else(|| panic!("{name} must have a bare #[cfg(test)] marker"));
        let loc = src[..at].lines().count();
        assert!(loc > 0, "positive control: {name} has an implementation");
        assert!(
            loc <= 300,
            "{name} is {loc} implementation LOC, over the cap"
        );
    }
}
