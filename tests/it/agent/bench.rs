//! LCV-200 — the agent evaluation bench: four laser-cutting requests under
//! `tests/fixtures/agent-bench/<task>/`, each run as one agent turn on a fresh
//! default document through the real loop and apply path
//! (`app::run_turn_inline`), and scored against its reference by
//! [`super::bench_score`].
//!
//! The gate replays every task from its recorded model replies, with no
//! network, and asserts the score equals the recorded `expected.json`. With
//! `LASERCAD_BENCH_RECORD=1` a replay writes `expected.json` instead — for the
//! commit that changes a tool's wire shape together with its fixtures.
//!
//! ADR 0002 §A2: `App::default()` only. ADR 0005: no dialog is armed.
//! ADR 0006: no per-user path is injected.

use super::bench_score::{Assertion, iou};
use lasercad::agent::{AgentError, AssistantMessage, ChatMessage};
use lasercad::app::{App, InlineTurn, TurnConfig, config_for, run_turn_inline};
use lasercad::document::Document;
use lasercad::io::svg::import_svg;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// The suite, in directory order.
const TASKS: [&str; 4] = ["box-face-tabs", "gear-outline", "plate-holes", "text-label"];

/// Replayed IoU must equal the recorded one within this.
const IOU_TOL: f64 = 1e-6;

fn bench_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/agent-bench")
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// One task of the suite, loaded from its folder.
struct Task {
    name: &'static str,
    dir: PathBuf,
    prompt: String,
    reference: Document,
    assertions: Vec<Assertion>,
}

fn load(name: &'static str) -> Task {
    let dir = bench_dir().join(name);
    let reference = import_svg(&read(&dir.join("reference.svg")))
        .and_then(|svg| svg.into_document())
        .unwrap_or_else(|e| panic!("{name}/reference.svg: {e}"));
    let assertions = serde_json::from_str(&read(&dir.join("assertions.json")))
        .unwrap_or_else(|e| panic!("{name}/assertions.json: {e}"));
    Task {
        name,
        prompt: read(&dir.join("prompt.txt")).trim().to_owned(),
        reference,
        assertions,
        dir,
    }
}

/// A task's score: the IoU and each assertion's pass/fail, in file order.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Score {
    iou: f64,
    assertions: Vec<bool>,
}

impl Score {
    fn passed(&self) -> usize {
        self.assertions.iter().filter(|a| **a).count()
    }
}

/// IoU rounded to the 6 decimals `expected.json` and the JSON line carry.
fn round6(x: f64) -> f64 {
    (x * 1e6).round() / 1e6
}

/// Run `task` as one turn on `app` and score the drawing it leaves.
fn run<F>(task: &Task, app: &mut App, config: &TurnConfig, send: &mut F) -> (Score, InlineTurn)
where
    F: FnMut(&[ChatMessage]) -> Result<AssistantMessage, AgentError>,
{
    let turn = run_turn_inline(app, &task.prompt, config, send);
    let score = Score {
        iou: round6(iou(&app.document, &task.reference)),
        assertions: task
            .assertions
            .iter()
            .map(|a| a.passes(&app.document))
            .collect(),
    };
    (score, turn)
}

/// The task's JSON line (AC 4).
fn json_line(task: &str, score: &Score, turn: &InlineTurn) -> String {
    let m = turn.metrics;
    serde_json::json!({
        "task": task,
        "iou": score.iou,
        "passed": score.passed(),
        "total": score.assertions.len(),
        "steps": m.steps,
        "applied": m.applied,
        "refused": m.refused,
        "repeated": m.repeated,
        "captures": m.captures,
        "replies": m.replies,
        "wall_ms": turn.wall.as_millis(),
    })
    .to_string()
}

/// Replay `task` from its recorded `replies` file on a fresh default app.
fn replay(task: &Task, replies: &str) -> (Score, InlineTurn) {
    let recorded: Vec<AssistantMessage> = serde_json::from_str(&read(&task.dir.join(replies)))
        .unwrap_or_else(|e| panic!("{}/{replies}: {e}", task.name));
    let count = recorded.len();
    let mut next = recorded.into_iter();
    let mut send = |_: &[ChatMessage]| {
        next.next()
            .ok_or_else(|| AgentError::Transport("the recording ran out of replies".to_owned()))
    };
    let mut app = App::default();
    let config = config_for(&app);
    let (score, turn) = run(task, &mut app, &config, &mut send);
    assert!(turn.result.is_ok(), "{}: {:?}", task.name, turn.result);
    assert_eq!(
        turn.metrics.replies as usize, count,
        "{}: every reply used",
        task.name
    );
    (score, turn)
}

/// Replay `replies`, print its JSON line, and assert the score equals the
/// recorded `expected` (or record it, under `LASERCAD_BENCH_RECORD=1`).
#[expect(
    clippy::print_stdout,
    reason = "AC 4: a replay reports its task's JSON line on stdout"
)]
fn assert_replay(name: &'static str, replies: &str, expected: &str) -> Score {
    let task = load(name);
    let (score, turn) = replay(&task, replies);
    println!("{}", json_line(name, &score, &turn));
    let path = task.dir.join(expected);
    if std::env::var_os("LASERCAD_BENCH_RECORD").is_some() {
        let json = serde_json::to_string_pretty(&score).expect("a score serializes");
        std::fs::write(&path, json + "\n").expect("expected.json is writable");
    }
    let want: Score = serde_json::from_str(&read(&path)).expect("a recorded score");
    assert!(
        (score.iou - want.iou).abs() <= IOU_TOL,
        "{name}: IoU {} != recorded {}",
        score.iou,
        want.iou
    );
    assert_eq!(score.assertions, want.assertions, "{name}: assertions");
    score
}

/// AC 1 — the suite is exactly the four tasks, each a prompt, a reference
/// SVG with geometry, at least one assertion and its recordings.
#[test]
fn ac1_the_suite_holds_exactly_the_four_tasks() {
    let mut dirs: Vec<String> = std::fs::read_dir(bench_dir())
        .expect("the bench directory")
        .map(|e| {
            e.expect("an entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    dirs.sort();
    assert_eq!(dirs, TASKS);
    for name in TASKS {
        let task = load(name);
        assert!(!task.prompt.is_empty(), "{name}: a prompt");
        assert!(!task.reference.entities.is_empty(), "{name}: a reference");
        assert!(!task.assertions.is_empty(), "{name}: assertions");
        for file in ["replies.json", "expected.json"] {
            assert!(task.dir.join(file).is_file(), "{name}: {file}");
        }
    }
}

/// AC 4 — the JSON line carries the task, its score and the turn's counts,
/// and nothing else.
#[test]
fn ac4_the_json_line_has_the_score_and_the_turn_counts() {
    let task = load("plate-holes");
    let (score, turn) = replay(&task, "replies.json");
    let line: serde_json::Value =
        serde_json::from_str(&json_line(task.name, &score, &turn)).expect("one JSON line");
    let keys: Vec<&str> = line
        .as_object()
        .expect("an object")
        .keys()
        .map(String::as_str)
        .collect();
    let mut want = [
        "task", "iou", "passed", "total", "steps", "applied", "refused", "repeated", "captures",
        "replies", "wall_ms",
    ];
    want.sort_unstable();
    assert_eq!(keys, want, "serde_json sorts keys");
    assert_eq!(line["task"], "plate-holes");
    assert_eq!(line["passed"], 4);
    assert_eq!(line["total"], 4);
    assert_eq!(line["applied"], 1);
    assert_eq!(line["replies"], 3);
}

/// AC 2, AC 5 — `box-face-tabs` replays to its recorded score.
#[test]
fn ac5_box_face_tabs_replays_to_its_recorded_score() {
    assert_replay("box-face-tabs", "replies.json", "expected.json");
}

/// AC 2, AC 5 — `gear-outline` replays to its recorded score.
#[test]
fn ac5_gear_outline_replays_to_its_recorded_score() {
    assert_replay("gear-outline", "replies.json", "expected.json");
}

/// AC 2, AC 5 — `plate-holes` replays to its recorded score.
#[test]
fn ac5_plate_holes_replays_to_its_recorded_score() {
    assert_replay("plate-holes", "replies.json", "expected.json");
}

/// AC 2, AC 5 — `text-label` replays to its recorded score.
#[test]
fn ac5_text_label_replays_to_its_recorded_score() {
    assert_replay("text-label", "replies.json", "expected.json");
}

/// AC 6 — a recording that leaves one hole out scores below the clean one,
/// on IoU and on assertions passed both, and replays to its own record.
#[test]
fn ac6_a_missing_hole_scores_below_the_clean_recording() {
    let clean = assert_replay("plate-holes", "replies.json", "expected.json");
    let defect = assert_replay("plate-holes", "replies-defect.json", "expected-defect.json");
    assert!(
        defect.iou < clean.iou,
        "IoU {} vs {}",
        defect.iou,
        clean.iou
    );
    assert!(defect.passed() < clean.passed(), "{defect:?} vs {clean:?}");
}
