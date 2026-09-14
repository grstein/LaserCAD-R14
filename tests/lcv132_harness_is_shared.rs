//! LCV-132 — the shared harness itself, and the one thing a shared harness
//! cannot prove about itself.
//!
//! This file is the consumer that proves `tests/harness/` is reachable from a
//! test binary at all, and it is the only file whose subject *is* the harness.
//! Every other test in the repository asserts something about the product
//! through it.
//!
//! ## What is deliberately not here
//!
//! There is no scan asserting that `fn collect_text` appears once in
//! `tests/harness/` and in no test file. **That scan would pass on a helper
//! that is shared and broken**, which is the failure mode this demand is named
//! after. The proof that the consolidation preserved behaviour is the fifteen
//! mutations in LCV-132 AC 7, applied to the shipped code and reverted — not
//! anything written down here.
//!
//! What *is* here is the handful of claims about the harness that no caller
//! makes on its behalf:
//!
//! - the two scan helpers agree, and presence is genuinely derived from counts
//!   rather than re-filtered beside them (AC 1, AC 2);
//! - the collector keeps `pos` untruncated (AC 4). Worked through on the
//!   measured numbers, a rounding collector is invisible to every other test in
//!   the repository: LCV-133's straddle control reads `pos.y = 446.682`, which
//!   rounds to `447`, and `447 + 14.0` is still past the `460.000` clip bottom,
//!   so that control still passes. "The suite is green" is therefore not
//!   evidence that precision survived. This test is;
//! - the collector recurses into `Shape::Vec` and propagates the clip rect down
//!   (AC 5);
//! - empty and whitespace-only runs are dropped (AC 6);
//! - a declared-and-unused `mod harness;` fails (AC 8). The blanket
//!   `#![allow(dead_code)]` in `tests/harness/mod.rs` hides exactly that case,
//!   and it hid one in `tests/lcv133_shortcuts_dialog_fits.rs` until a reviewer
//!   read for it.
//!
//! No test here drives the product, reaches an endpoint, or asserts an absolute
//! coordinate: every position in this file is one the test itself put in.

mod harness;

use harness::paint::{collect_text, runs_in, Run};
use harness::scan::{files_containing, occurrences, rs_files};
use std::path::{Path, PathBuf};

// ── Fixtures this file creates and owns ─────────────────────────────────────

/// A directory under the system temp dir, created empty, owned by this test.
fn tempdir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lasercad_lcv132_{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the system temp dir must be writable");
    dir
}

/// A path rendered from its components, joined with `/`.
///
/// Never `Path::display()`: that emits `\` on Windows, so a comparison against
/// a `/`-joined expectation passes on Linux and fails only in CI. This has
/// broken this repository's CI twice.
fn slashed(path: &Path) -> String {
    path.components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// A galley laid out at the default proportional font, for a hand-built shape.
fn galley(ctx: &egui::Context, text: &str) -> std::sync::Arc<egui::Galley> {
    ctx.fonts(|f| {
        f.layout_no_wrap(
            text.to_owned(),
            egui::FontId::proportional(14.0),
            egui::Color32::WHITE,
        )
    })
}

/// One `Shape::Text` at `pos`, carrying `text`.
fn text_shape(ctx: &egui::Context, pos: egui::Pos2, text: &str) -> egui::Shape {
    egui::Shape::Text(egui::epaint::TextShape::new(
        pos,
        galley(ctx, text),
        egui::Color32::WHITE,
    ))
}

/// A context whose positions are points, one to one, with its font atlas
/// built. Every test in this file that reads a position sets the first, per
/// LCV-132 §Test hygiene.
///
/// The throwaway frame is not ceremony: at egui 0.29.1 `Context::fonts` panics
/// with *"No fonts available until first call to Context::run()"* on a fresh
/// context, so a test that lays out a hand-built galley must drive one frame
/// before it can build one. Tests that drive the product through
/// `harness::paint` never meet this, because their first frame does it for
/// them.
fn ctx() -> egui::Context {
    let ctx = egui::Context::default();
    ctx.set_pixels_per_point(1.0);
    let _ = ctx.run(egui::RawInput::default(), |_| {});
    ctx
}

// ── AC 1: one `rs_files`, and it recurses ───────────────────────────────────

/// AC 1 — the shared walker returns every `.rs` file under a tree and nothing
/// else, including one two levels down.
///
/// The nesting is the point, not decoration: `src/io/svg/` and
/// `src/document/commands/` are two levels down, so a flattened walk would
/// scan neither while every wrapper's whole-tree positive control still
/// passed. The empty subdirectory is here because a walker that assumed at
/// least one entry per directory would trip on it.
#[test]
fn ac1_rs_files_returns_every_rs_file_in_the_tree_and_nothing_else() {
    let root = tempdir("rs_files");
    std::fs::create_dir_all(root.join("nested/deeper")).expect("writable");
    std::fs::create_dir_all(root.join("empty")).expect("writable");
    std::fs::write(root.join("top.rs"), "fn top() {}").expect("writable");
    std::fs::write(root.join("notes.md"), "# not rust").expect("writable");
    std::fs::write(root.join("nested/mid.rs"), "fn mid() {}").expect("writable");
    std::fs::write(root.join("nested/data.json"), "{}").expect("writable");
    std::fs::write(root.join("nested/deeper/low.rs"), "fn low() {}").expect("writable");

    let mut found = Vec::new();
    rs_files(&root, &mut found);
    let mut names: Vec<String> = found
        .iter()
        .map(|p| slashed(p.strip_prefix(&root).expect("walked from root")))
        .collect();
    names.sort();

    assert_eq!(
        names,
        ["nested/deeper/low.rs", "nested/mid.rs", "top.rs"],
        "the walker returns every .rs file at every depth, and no non-.rs file"
    );
    std::fs::remove_dir_all(&root).expect("the fixture is this test's to remove");
}

// ── AC 2: one matcher, counting, with presence derived from it ──────────────

/// A two-file fixture: one code hit, one comment hit, one file with neither.
fn matcher_fixture() -> Vec<(String, String)> {
    let needle = concat!("agent_", "busy = false");
    vec![
        (
            "writes_twice.rs".to_owned(),
            format!("fn a() {{ {needle}; }}\nfn b() {{ {needle}; }}\n"),
        ),
        (
            "mentions_it_in_prose.rs".to_owned(),
            format!("//! this file must never {needle}\nfn c() {{}}\n"),
        ),
        ("silent.rs".to_owned(), "fn d() {}\n".to_owned()),
    ]
}

/// AC 2 — `occurrences` counts code lines and skips comment lines.
///
/// A count rather than a flag: "`agent_busy = false` appears once, in
/// `agent_poll.rs`" is a strictly stronger claim than "it appears in
/// `agent_poll.rs`", and a second write smuggled into the owning file is
/// exactly the regression the weaker claim misses. LCV-129 AC 5 rests on that
/// difference.
#[test]
fn ac2_occurrences_counts_code_lines_and_skips_comments() {
    let hits = occurrences(&matcher_fixture(), concat!("agent_", "busy = false"));
    assert_eq!(
        hits,
        vec![("writes_twice.rs".to_owned(), 2)],
        "two code hits in one file, and the comment-only file is not a hit"
    );
}

/// AC 2 — `files_containing` is `occurrences` with the counts dropped, and is
/// asserted as such rather than merely agreeing with it today.
///
/// The rule about which lines count lives once. A second independent filter is
/// how the two drift apart, and a drifted comment rule is invisible to every
/// test that only ever calls one of them. *Mutation record:* giving
/// `files_containing` its own filter and changing the comment rule in only one
/// of the two turns this assertion red.
#[test]
fn ac2_files_containing_is_derived_from_occurrences() {
    let fixture = matcher_fixture();
    let needle = concat!("agent_", "busy = false");
    assert_eq!(
        files_containing(&fixture, needle),
        occurrences(&fixture, needle)
            .into_iter()
            .map(|(path, _)| path)
            .collect::<Vec<_>>(),
        "presence is the paths `occurrences` reported, in that order — not a \
         second filter that happens to agree"
    );
    assert_eq!(
        files_containing(&fixture, needle),
        ["writes_twice.rs"],
        "positive control: the fixture has exactly one file naming the needle \
         on a code line, so this is not an empty-set agreement"
    );
}

// ── AC 4: full precision at collection ──────────────────────────────────────

/// AC 4 — a run laid out at a fractional position comes back at exactly that
/// position, with the galley's own height.
///
/// This is the assertion that protects LCV-133's containment checks, the only
/// ones in the repository that can see a sliced row. Their margin is 0.682pt —
/// under a whole-point rounding step — so whether a rounding collector still
/// sees the straddle would be decided by which way the round fell rather than
/// by the geometry. And `pos.y + height <= clip.bottom()` cannot be rebuilt at
/// all from a shape that stored no height.
#[test]
fn ac4_collect_text_keeps_pos_untruncated_and_carries_the_height() {
    let ctx = ctx();
    let pos = egui::pos2(12.375, 446.682);
    let shape = text_shape(&ctx, pos, "straddle");
    let clip = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(400.0, 460.0));

    let mut out = Vec::new();
    collect_text(clip, &shape, &mut out);

    assert_eq!(out.len(), 1, "one text shape in, one run out");
    assert_eq!(
        out[0].pos, pos,
        "the collected position is the laid-out position, to the bit"
    );
    assert_eq!(
        out[0].height,
        galley(&ctx, "straddle").size().y,
        "and the height is the galley's own, not a constant"
    );
    assert_eq!(
        (out[0].y(), out[0].x()),
        (447, 12),
        "the rounded reading is derived at the point of use, and rounds to \
         nearest rather than truncating"
    );
}

// ── AC 5: nesting, and the clip that comes down with it ─────────────────────

/// AC 5 — a `Shape::Text` nested two `Shape::Vec` deep is found, and it carries
/// the clip rect of the `ClippedShape` it was nested inside.
///
/// `Shape::Vec` is the only variant at egui 0.29.1 that contains other shapes,
/// which is why one recursion reaches everything; an egui bump has this test
/// and the collector's doc comment as its two named places to revisit. The
/// nesting is not hypothetical — a `Button`'s label is inside one, so a
/// collector that did not recurse would never see `Cancel`, and LCV-129 AC 8
/// would pass on a dialog with no way out of a running turn.
#[test]
fn ac5_collect_text_recurses_and_propagates_the_clip() {
    let ctx = ctx();
    let clip = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(300.0, 200.0));
    let nested = egui::Shape::Vec(vec![egui::Shape::Vec(vec![text_shape(
        &ctx,
        egui::pos2(11.5, 21.5),
        "two deep",
    )])]);

    let mut out = Vec::new();
    collect_text(clip, &nested, &mut out);

    assert_eq!(
        out.iter().map(|r| r.text.as_str()).collect::<Vec<_>>(),
        ["two deep"],
        "the collector descends through every Shape::Vec, not just the first"
    );
    assert_eq!(
        out[0].clip, clip,
        "the clip rect comes from the ClippedShape the run was nested inside — \
         a nested run has no clip of its own, and membership of a surface is \
         clip containment, so a collector that lost this would put every run \
         on every surface"
    );
}

// ── AC 6: the empty-run rule ────────────────────────────────────────────────

/// AC 6 — an empty galley and a whitespace-only galley are both dropped.
///
/// An empty `TextEdit` emits a `Shape::Text` with an empty galley, and it
/// carries nothing an operator can read. Dropping it is what makes LCV-125's
/// line `["API Key"]` mean *the key field shows nothing at all*. The
/// whitespace-only case is the one the four private collectors disagreed
/// about: LCV-125 dropped on `is_empty()` where the other three dropped on
/// `trim().is_empty()`, so a blank-looking run survived in one file and died
/// in three.
#[test]
fn ac6_empty_and_whitespace_only_runs_are_dropped() {
    let ctx = ctx();
    let clip = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(200.0, 100.0));
    let shapes: Vec<egui::epaint::ClippedShape> = ["", "   ", "\t", "real"]
        .into_iter()
        .map(|text| egui::epaint::ClippedShape {
            clip_rect: clip,
            shape: text_shape(&ctx, egui::pos2(1.0, 2.0), text),
        })
        .collect();

    let runs: Vec<String> = runs_in(&shapes).into_iter().map(|r| r.text).collect();
    assert_eq!(
        runs,
        ["real"],
        "only the run an operator could read survives the filter"
    );
}

/// AC 6 — the driver's positive control is what stops an app that painted
/// nothing from reading as "the string is absent, as expected".
///
/// A harness helper that asserts is unusual and deliberate. This test pins the
/// threshold's reason rather than the threshold: a frame carrying fewer runs
/// than a real one is not a frame any absence assertion may be made against.
/// *Mutation record:* deleting the `runs.len() > 10` assertion from the driver
/// and making `update_ui` paint nothing for one frame makes every converted
/// absence assertion pass cleanly — the false green the control exists to
/// prevent.
#[test]
fn ac6_a_frame_that_paints_almost_nothing_is_not_a_frame_to_assert_absence_on() {
    let ctx = ctx();
    let clip = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(200.0, 100.0));
    let sparse: Vec<egui::epaint::ClippedShape> = (0..3)
        .map(|i| egui::epaint::ClippedShape {
            clip_rect: clip,
            shape: text_shape(&ctx, egui::pos2(1.0, i as f32 * 10.0), "row"),
        })
        .collect();
    assert!(
        runs_in(&sparse).len() <= 10,
        "the threshold the driver enforces is above a hand-built handful: a \
         real frame of this app paints many more"
    );
}

// ── AC 8: a `mod harness;` that nobody uses ─────────────────────────────────

/// The declaration and the use, spliced at compile time so no whole copy of
/// either literal sits in this file's own text. The scan below reads every
/// `.rs` file directly in `tests/`, and this file is one of them.
const DECLARES: &str = concat!("mod ", "harness;");
const USES: &str = concat!("harness", "::");

/// Every `.rs` file directly in `tests/`, as `(name, body)`.
///
/// Flat on purpose: `tests/harness/` is the module itself, not a consumer, and
/// only files directly in `tests/` are compiled as test binaries.
fn test_binaries() -> Vec<(String, String)> {
    let mut sections: Vec<(String, String)> = std::fs::read_dir("tests")
        .expect("the tests directory must be readable")
        .filter_map(|entry| {
            let path = entry.expect("a readable directory entry").path();
            (path.is_file() && path.extension().is_some_and(|e| e == "rs")).then(|| {
                (
                    slashed(path.file_name().expect("a file has a name").as_ref()),
                    std::fs::read_to_string(&path).expect("a readable test file"),
                )
            })
        })
        .collect();
    sections.sort();
    assert!(
        sections.len() > 20,
        "positive control: this repository has many test binaries, saw {}",
        sections.len()
    );
    sections
}

/// The files that declare the harness on a code line without naming it on one.
///
/// Both needles go through `harness::scan::files_containing`, so the rule about
/// which lines count is the same one every other scan in the repository uses —
/// and the comment-skipping is load-bearing here rather than incidental: a
/// module header that *names* the harness in prose is not a use of it, and
/// `tests/lcv118_grid_and_snap.rs` is a live example.
fn orphan_includes(sections: &[(String, String)]) -> Vec<String> {
    let uses = files_containing(sections, USES);
    files_containing(sections, DECLARES)
        .into_iter()
        .filter(|path| !uses.contains(path))
        .collect()
}

/// AC 8 — no test binary declares `mod harness;` without using it.
///
/// This exists because the blanket `#![allow(dead_code)]` in
/// `tests/harness/mod.rs` hides precisely this case: the module compiles, every
/// item in it is allowed to be unused, and an include that buys nothing sits
/// there looking like a dependency. One did, in
/// `tests/lcv133_shortcuts_dialog_fits.rs`, until a reviewer read for it. This
/// demand widens that blanket's reach, so it pays for it here.
#[test]
fn ac8_no_test_binary_declares_the_harness_without_using_it() {
    let sections = test_binaries();
    assert_eq!(
        orphan_includes(&sections),
        Vec::<String>::new(),
        "these files declare `{DECLARES}` on a code line and never name \
         `{USES}` on one; the blanket allow(dead_code) in the harness means \
         nothing else will tell you"
    );
    assert!(
        !files_containing(&sections, DECLARES).is_empty(),
        "positive control: some test binary does include the harness, so the \
         assertion above is not over an empty haystack"
    );
}

/// AC 8 — the guard discriminates, proven on a fixture this test builds.
///
/// Three files: one that declares and uses, one that declares and only mentions
/// the harness in a `//!` line, and one that does neither. Only the middle one
/// is an orphan. Run through the same helper as the real scan, because an
/// absence assertion over a mis-sliced haystack passes for the wrong reason and
/// passes silently. *Mutation record:* disabling the comment-skipping in
/// `harness::scan::occurrences` makes the middle file read as a use, and this
/// test goes red naming it.
#[test]
fn ac8_the_orphan_guard_discriminates_on_a_synthetic_fixture() {
    let fixture = vec![
        (
            "uses_it.rs".to_owned(),
            format!("{DECLARES}\nfn a() {{ {USES}scan::rs_files(); }}\n"),
        ),
        (
            "declares_and_only_talks_about_it.rs".to_owned(),
            format!("//! built on {USES}paint, one day\n{DECLARES}\nfn b() {{}}\n"),
        ),
        ("unrelated.rs".to_owned(), "fn c() {}\n".to_owned()),
    ];
    assert_eq!(
        orphan_includes(&fixture),
        ["declares_and_only_talks_about_it.rs"],
        "a prose mention is not a use, and a file that neither declares nor \
         uses the harness is not an orphan"
    );
}

/// AC 8 — `Run` is reachable and constructible from a test binary at all.
///
/// The dullest assertion in the file and the reason it is the LCV-132 consumer:
/// if `tests/harness/` were placed anywhere a test binary could not see it —
/// behind `#[cfg(test)]` under `src/`, say, which integration tests link with
/// that cfg **unset** — this file would not compile.
#[test]
fn ac8_the_harness_types_are_reachable_from_a_test_binary() {
    let run = Run {
        clip: egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(10.0, 10.0)),
        pos: egui::pos2(1.5, 2.5),
        height: 14.0,
        text: "reachable".to_owned(),
    };
    assert_eq!((run.x(), run.y()), (2, 3), "and its accessors round");
}
