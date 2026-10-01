//! LCV-165 AC 3 — every waiting tool speaks one prompt grammar:
//! `VERB  Specify <thing> [Opt/Opt] <default>:`, and the picking tools
//! `VERB  Select object…:`. The table is driven by typed input through the
//! real frame, one tool state per step (TEXT's start point is clicked).

use crate::harness;

use harness::{frame, raw_input, submit_command};
use lasercad::app::App;
use lasercad::document::Entity;
use lasercad::geometry::{Line, Vec2};

/// The one non-typed input: a click at the canvas centre. TEXT takes its
/// start point from the pointer only, so its first step is clicked.
const CLICK: &str = "<click>";

/// A fresh app with one selected line, so the modify tools have a subject,
/// and the canvas rect.
fn boot() -> (egui::Context, App, egui::Rect) {
    let ctx = egui::Context::default();
    let mut app = App::default();
    let mut canvas = egui::Rect::NOTHING;
    let _ = ctx.run(raw_input(vec![]), |c| {
        app.update_ui(c);
        canvas = c.available_rect();
    });
    let line = Line::new(Vec2::new(10.0, 0.0), Vec2::new(20.0, 5.0));
    let layer = app.document.current_layer();
    app.document.push_entity(Entity::Line(line), layer);
    app.document.selection.add(0);
    (ctx, app, canvas)
}

/// A primary click at `pos`, after its own hover frame.
fn click(ctx: &egui::Context, app: &mut App, pos: egui::Pos2) {
    frame(ctx, app, vec![egui::Event::PointerMoved(pos)]);
    let button = |pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    frame(ctx, app, vec![button(true), button(false)]);
}

/// Type each line of `inputs` in turn on a fresh app; after each, the
/// prompt must read the matching entry of `prompts`.
fn drive(inputs: &[&str], prompts: &[&str]) -> Vec<String> {
    let (ctx, mut app, canvas) = boot();
    let mut seen = Vec::new();
    for (input, expected) in inputs.iter().zip(prompts) {
        match *input {
            CLICK => click(&ctx, &mut app, canvas.center()),
            typed => submit_command(&ctx, &mut app, typed),
        }
        let prompt = app.tool_manager.active_status_text().into_owned();
        assert_eq!(prompt, *expected, "after typing {input:?}");
        seen.push(prompt);
    }
    seen
}

/// The prompt table (plan.md; DESIGN.md §7): typed inputs and the prompt
/// after each.
const TABLE: &[(&[&str], &[&str])] = &[
    (
        &["l", "0,0"],
        &[
            "LINE  Specify first point:",
            "LINE  Specify next point <Enter to finish>:",
        ],
    ),
    (
        &["p", "0,0"],
        &[
            "PLINE  Specify first point:",
            "PLINE  Specify next point <Enter to finish>:",
        ],
    ),
    (
        &["r", "0,0"],
        &[
            "RECT  Specify first corner:",
            "RECT  Specify opposite corner:",
        ],
    ),
    (
        &["c", "0,0"],
        &["CIRCLE  Specify center point:", "CIRCLE  Specify radius:"],
    ),
    (
        &["a", "0,0", "10,0"],
        &[
            "ARC  Specify start point:",
            "ARC  Specify end point:",
            "ARC  Specify point on arc:",
        ],
    ),
    (
        &["text", CLICK, "HELLO"],
        &[
            "TEXT  Specify start point:",
            "TEXT  Specify text:",
            "TEXT  Specify height <5>:",
        ],
    ),
    (
        &["dist", "0,0"],
        &["DIST  Specify first point:", "DIST  Specify second point:"],
    ),
    (
        &["m", "0,0"],
        &[
            "MOVE  Specify base point:",
            "MOVE  Specify destination point:",
        ],
    ),
    (
        &["copy", "0,0"],
        &["COPY  Specify base point:", "COPY  Specify second point:"],
    ),
    (
        &["ro", "0,0"],
        &[
            "ROTATE  Specify base point:",
            "ROTATE  Specify rotation angle:",
        ],
    ),
    (
        &["sc", "0,0"],
        &["SCALE  Specify base point:", "SCALE  Specify scale factor:"],
    ),
    (
        &["mi", "0,0", "0,10"],
        &[
            "MIRROR  Specify first point of mirror line:",
            "MIRROR  Specify second point of mirror line:",
            "MIRROR  Erase source objects? [Yes/No] <N>:",
        ],
    ),
    (&["t"], &["TRIM  Select object to trim:"]),
    (&["extend"], &["EXTEND  Select object to extend:"]),
    (&["e"], &["ERASE  Select objects:"]),
];

/// AC 3 — each tool state shows its row of the prompt table.
#[test]
fn ac3_every_tool_state_shows_its_table_prompt() {
    for (inputs, prompts) in TABLE {
        drive(inputs, prompts);
    }
}

/// Whether `prompt` has the form `VERB  <request> [Opt/Opt] <default>:`
/// with the request a `Specify …` or `Select …` — or, for MIRROR's one
/// yes/no question, a question ending in `?` followed by its options.
fn follows_the_grammar(prompt: &str) -> bool {
    let Some((verb, rest)) = prompt.split_once("  ") else {
        return false;
    };
    let Some(body) = rest.strip_suffix(':') else {
        return false;
    };
    let verb_ok = verb.len() >= 2 && verb.chars().all(|c| c.is_ascii_uppercase());
    // The request runs up to the first ` [` or ` <`; the tail holds the
    // options and the default, each bracketed.
    let cut = [body.find(" ["), body.find(" <")]
        .into_iter()
        .flatten()
        .min()
        .unwrap_or(body.len());
    let (request, tail) = body.split_at(cut);
    let request_ok =
        request.starts_with("Specify ") || request.starts_with("Select ") || request.ends_with('?');
    let tail_ok = tail.split_inclusive([']', '>']).all(|t| {
        let t = t.trim_start();
        (t.starts_with('[') && t.ends_with(']')) || (t.starts_with('<') && t.ends_with('>'))
    });
    verb_ok && request_ok && tail_ok
}

/// AC 3 — the grammar holds for every prompt the table reaches, and the
/// check rejects the pre-LCV-165 dialects (negative controls).
#[test]
fn ac3_every_prompt_follows_the_grammar() {
    for (inputs, prompts) in TABLE {
        for prompt in drive(inputs, prompts) {
            assert!(follows_the_grammar(&prompt), "{prompt:?}");
        }
    }
    for old in [
        "LINE Specify first point:",
        "TRIM: Click on a segment to trim",
        "ERASE",
        "EXTEND: Click near a line or arc endpoint to extend it",
    ] {
        assert!(!follows_the_grammar(old), "control: {old:?}");
    }
}

/// AC 3 — Select at rest keeps R14's idle prompt.
#[test]
fn ac3_select_keeps_command() {
    let (_ctx, app, _) = boot();
    assert_eq!(app.tool_manager.active_status_text(), "Command:");
}
