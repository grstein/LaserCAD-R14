//! The two [`App`] constructors (LCV-115 file split).
//!
//! Moved verbatim out of `src/app/mod.rs`, which was at 296 of the 300-LOC
//! implementation cap and could not absorb LCV-115's `export_preset` field
//! otherwise (ADR 0004 leaves this file's seam to the demand that crosses).
//! Pure relocation: no behaviour change.
//!
//! The pair belongs together and is the half of `mod.rs` that could move:
//! ADR 0002 §A2 defines them *as* a pair — [`App::default`] is the **test**
//! constructor and touches no filesystem, [`App::new`] is **boot-only** and
//! reads the platform config and data directories — and they are also what
//! grows when `struct App` grows, since every new field adds an initialiser
//! here. The struct itself stays in `mod.rs` with the module's public
//! surface, where it cannot move without moving [`App`].
//!
//! The `app_default_*` value assertions stay in `mod.rs`'s test module, next
//! to the field declarations whose defaults they mirror. The one test that
//! reads `App::new`'s own source moved here with the function.
//!
//! MUST NOT import `eframe` or `rfd`.

use super::{AgentState, App, DocumentTitleState, UnsavedGuard};
use crate::cmdline::CommandHistory;
use crate::document::{Document, History};
use crate::io::settings::Settings;
use crate::render::Camera;
use crate::tools::ToolManager;

/// The test constructor (ADR 0002 §A2). Touches no filesystem, and — since
/// LCV-119 / ADR 0006 — *cannot*: `settings_path` and `autosave_path` are
/// both `None`, so every persistence call the resulting `App` can reach is a
/// no-op. `settings` is `Settings::default()` and `document` is never
/// replaced with an autosaved one. Safe to call from any `#[cfg(test)]`
/// context; a test that wants real persistence points the two path fields at
/// a temporary directory it owns. Boot code must use [`App::new`] instead,
/// which resolves the two real platform locations.
impl Default for App {
    fn default() -> Self {
        let mut app = Self {
            document: Document::default(),
            history: History::default(),
            camera: Camera::default(),
            last_cursor_world: None,
            preview_entities: Vec::new(),
            active_snap: None,
            tool_manager: ToolManager::default(),
            settings: Settings::default(),
            settings_path: None,
            autosave_path: None,
            dirty_since: None,
            last_synced_revision: 0,
            last_autosave_at: None,
            about_open: false,
            shortcuts_open: false,
            agent_settings_open: false,
            bed_dialog: None,
            command_line_input: String::new(),
            command_history: CommandHistory::default(),
            command_feedback: String::new(),
            focus_command_line: false,
            command_line_focused: false,
            snap_enabled: true,
            grid_enabled: true,
            ortho_enabled: false,
            agent: AgentState::default(),
            current_file: None,
            title: DocumentTitleState::default(),
            error_message: None,
            guard: UnsavedGuard::default(),
        };
        // Pre-seed the title cache to what this fresh state already computes
        // (LCV-138): a test `App` never backs a real OS window, so there is
        // no native title to correct, and every one of egui's
        // `ctx.send_viewport_cmd` calls costs *two* frames of
        // `repaint_delay == 0` internally (`egui-0.29.1`
        // `context.rs::request_repaint_after`'s "each request results in two
        // repaints" `outstanding` counter) — without this, a freshly
        // constructed, untouched `App::default()` would fail
        // `tests/it/app/idle_repaint.rs::an_idle_app_asks_for_no_repaint` on
        // its first two supposedly-idle frames, purely from the title cache
        // starting `None`. `App::new()` deliberately does **not** do this:
        // the real native window was already told the static `APP_TITLE` by
        // `ViewportBuilder::with_title` before the first frame runs, so that
        // first frame must still send the real correction.
        app.title.last_title = Some(app.display_title());
        app
    }
}

impl App {
    /// Construct the application: boot-only. **The one place in the tree
    /// that resolves a real per-user filesystem location** (LCV-119,
    /// ADR 0006): it calls the two `platform_path` resolvers, stores their
    /// results in `settings_path` / `autosave_path`, and every later read or
    /// write goes through those fields. MUST NOT be called from tests
    /// (ADR 0002 §A2); tests use [`App::default`].
    ///
    /// Calls [`Self::default()`] for all fields, then:
    /// - loads persisted settings from the resolved config path — recent
    ///   files, agent endpoint, agent API key — overwriting the default
    ///   `settings`;
    /// - overwrites `document` with the autosaved one from the resolved data
    ///   path, if present and schema-compatible (LCV-059 AC#1) — the
    ///   envelope's own `bed_mm` wins, and `title.recovered_from_autosave` is
    ///   set (LCV-138 AC 4) — the only place in the tree that sets it;
    /// - otherwise seeds the blank document's bed from
    ///   `settings.default_bed_mm` (LCV-114 AC 11).
    ///
    /// A platform that supplies no config or data directory degrades to
    /// defaults, never to a panic: an unresolved path is the same `None` the
    /// test constructor carries, and simply means this process does not
    /// persist.
    ///
    /// Performs no write of its own: no settings write, no autosave write, no
    /// file created on the startup path.
    pub fn new() -> Self {
        let settings_path = crate::io::settings::platform_path();
        let autosave_path = crate::io::autosave::platform_path();
        let mut app = Self {
            settings: settings_path
                .as_deref()
                .map(crate::io::settings::load_from)
                .unwrap_or_default(),
            settings_path,
            autosave_path,
            ..Self::default()
        };
        let recovered = app
            .autosave_path
            .as_deref()
            .and_then(crate::io::autosave::load_autosave_from);
        if let Some(recovered) = recovered {
            app.document = recovered;
            app.title.recovered_from_autosave = true;
        } else {
            app.document.bed_mm = app.settings.clamped_default_bed_mm();
        }
        app
    }
}

#[cfg(test)]
mod tests {
    /// LCV-119 AC 2 / ADR 0006 — `App::new` is the **only** place that
    /// resolves a real per-user filesystem location, and it must fill *both*
    /// path fields. If it ever stops filling one, the app silently stops
    /// persisting that half and no behavioural test can notice, because no
    /// test may call `App::new` (ADR 0002 §A2). This scan is the only guard
    /// against that failure mode, so it is deliberately literal.
    ///
    /// The haystack stops at the test module and is further narrowed to
    /// `App::new`'s body, and each needle is assembled with `concat!` so it
    /// never appears whole in this test's own source: the scan cannot match
    /// itself.
    #[test]
    fn boot_resolves_and_stores_both_real_user_paths() {
        let body = app_new_body();
        assert!(
            body.contains(concat!("crate::io::settings::", "platform_path()")),
            "App::new must resolve the settings path (AC 2)"
        );
        assert!(
            body.contains(concat!("crate::io::autosave::", "platform_path()")),
            "App::new must resolve the autosave path (AC 2)"
        );
        for field in [concat!("settings", "_path,"), concat!("autosave", "_path,")] {
            assert!(
                body.contains(field),
                "App::new must store `{field}` on the App it returns (AC 2)"
            );
        }
        assert!(
            body.contains("..Self::default()"),
            "positive control: App::new still builds on the test constructor"
        );
    }

    /// LCV-119 AC 11 — boot performs no write of its own. A settings write on
    /// the startup path would rewrite the operator's file with whatever
    /// parsed (or failed to parse), before they have touched anything.
    #[test]
    fn boot_performs_no_write_of_its_own() {
        let body = app_new_body();
        assert!(
            body.contains(concat!("crate::io::settings::", "load_from")),
            "positive control: boot reads the settings file"
        );
        for writer in [
            concat!("save", "_to("),
            concat!("persist", "_settings()"),
            concat!("write", "_autosave()"),
        ] {
            assert!(
                !body.contains(writer),
                "App::new must not `{writer}` on the startup path (AC 11)"
            );
        }
    }

    /// The source text of `App::new`'s body, bounded to the implementation
    /// section (everything before the bare `#[cfg(test)]` anchor) and then to
    /// the function itself.
    fn app_new_body() -> &'static str {
        let src = include_str!("init.rs");
        let cfg_test_at = src
            .find("\n#[cfg(test)]")
            .expect("init.rs must have a test module to bound the scan");
        let implementation = &src[..cfg_test_at];
        let start = implementation
            .find("pub fn new() -> Self {")
            .expect("App::new must exist");
        &implementation[start..]
    }

    /// LCV-138 review finding — `App::new()` must never assign
    /// `title.last_title` itself. Only [`Default::default`]'s pre-seed does
    /// that (see its own doc comment: a test `App` never backs a real OS
    /// window, so there is nothing for the first frame to correct). A real
    /// window, by contrast, was only ever told the *static* `APP_TITLE`
    /// (`ViewportBuilder::with_title`, `src/lib.rs`) before its first frame
    /// ran — so that first frame's `update_title` comparison must find the
    /// cache still `None` and send the real `ViewportCommand::Title`
    /// correction (recovered document, real `current_file`, or neither).
    /// Copying `Self::default()`'s pre-seed forward into `App::new` (e.g.
    /// appending `app.title.last_title = Some(app.display_title());` to its
    /// body) would pre-fill the cache with a blank-document title that has
    /// nothing to do with what actually booted, so that first-frame
    /// comparison would see no change and the real window would keep
    /// showing the static title forever. Bounded to `App::new`'s own body
    /// (`app_new_body()`, the same helper the two `recovered_from_autosave`
    /// scans below reuse), so `Self::default()`'s own, deliberate
    /// pre-seed line cannot satisfy this scan.
    #[test]
    fn app_new_never_assigns_the_title_cache() {
        let body = app_new_body();
        assert!(
            !body.contains("title.last_title"),
            "App::new must not assign title.last_title itself: doing so \
             would suppress the real window's first-frame title correction"
        );
    }

    /// Review finding on LCV-138 — `App::new`'s body must never call
    /// `mark_saved`. `mark_saved` is the **only** clearer of
    /// `title.recovered_from_autosave` (`src/app/file_ops.rs::mark_saved`),
    /// so a stray call anywhere in `App::new` — even after the recovered
    /// branch sets the flag a few lines above — would silently erase the
    /// "this document was recovered from autosave" signal before the first
    /// frame ever renders, and no behavioural test could catch it (ADR 0002
    /// §A2: nothing may drive `App::new` directly). Bounded to `App::new`'s
    /// own body via `app_new_body()`, so this test's own source, and
    /// `Default::default`'s unrelated pre-seed, cannot satisfy it.
    #[test]
    fn app_new_never_calls_mark_saved() {
        let body = app_new_body();
        assert!(
            !body.contains("mark_saved("),
            "App::new must not call mark_saved: doing so would clear \
             title.recovered_from_autosave before the first frame renders"
        );
    }

    /// LCV-114 AC 11, boot half — a cold start with no autosave seeds the
    /// blank document from the settings default; a recovered autosave keeps
    /// its own bed.
    ///
    /// A bounded source scan, because `App::new` is the one constructor tests
    /// may not call (ADR 0002 §A2): it reads the platform config and data
    /// directories. The seed logic itself is covered behaviourally by
    /// `io::file_actions::tests::new_document_seeds_bed_from_settings` and the
    /// recovery half by `io::autosave::tests::envelope_round_trips_bed_mm`.
    ///
    /// The haystack stops at the test module, so this test's own body cannot
    /// satisfy it.
    #[test]
    fn boot_seeds_the_bed_only_when_no_autosave_is_recovered() {
        let src = include_str!("init.rs");
        let cfg_test_at = src
            .find("\n#[cfg(test)]")
            .expect("init.rs must have a test module to bound the scan");
        let implementation = &src[..cfg_test_at];
        let start = implementation
            .find("pub fn new() -> Self {")
            .expect("App::new must exist");
        let body = &implementation[start..];
        let recovered = body
            .find("app.document = recovered;")
            .expect("the autosave branch must install the recovered document");
        let seed = body
            .find("app.document.bed_mm = app.settings.clamped_default_bed_mm();")
            .expect("the cold-start branch must seed the bed from settings");
        assert!(
            recovered < seed && body[recovered..seed].contains("} else {"),
            "the seed must be the else-branch: a recovered envelope keeps its own bed"
        );
    }

    /// LCV-138 AC 4 — `title.recovered_from_autosave` is set to `true`
    /// **inside** the recovered branch, right alongside
    /// `app.document = recovered;` and strictly before the `} else {` that
    /// closes it — the same bounded-scan technique
    /// `boot_seeds_the_bed_only_when_no_autosave_is_recovered` uses just
    /// above, since `App::new` cannot be driven behaviourally (ADR 0002 §A2).
    #[test]
    fn recovered_from_autosave_is_set_inside_the_recovered_branch() {
        let body = app_new_body();
        let recovered = body
            .find("app.document = recovered;")
            .expect("the autosave branch must install the recovered document");
        let flag = body
            .find(concat!("app.title.recovered_from_", "autosave = true;"))
            .expect("the recovered branch must set title.recovered_from_autosave");
        let else_at = body
            .find("} else {")
            .expect("positive control: the branch must close with an else");
        assert!(
            recovered < flag && flag < else_at,
            "the flag must be set inside the recovered arm, before the else"
        );
    }

    /// LCV-138 AC 4 — the same fact, proved the other way round: nowhere else
    /// in the whole of `src/` sets the recovery flag to `true`. Combined with
    /// the scan above, this is what "set only there" means — not merely "set
    /// in this branch", but "set in no other branch either".
    #[test]
    fn recovered_from_autosave_is_set_nowhere_else_in_src() {
        let needle = concat!("title.recovered_from_", "autosave = true");
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = Vec::new();
        walk(&root, &mut files);
        assert!(
            files.len() > 40,
            "positive control: the walk must see the whole tree, saw {}",
            files.len()
        );

        // Total *occurrences*, not files: a second site in the same file
        // must still be caught, so this counts `matches(needle)` per file and
        // sums them, rather than asking only whether a file contains the
        // needle at all.
        let mut total = 0usize;
        let mut carrying_files = Vec::new();
        for path in &files {
            let n = std::fs::read_to_string(path)
                .expect("readable source")
                .matches(needle)
                .count();
            if n > 0 {
                total += n;
                carrying_files.push((path.clone(), n));
            }
        }
        assert_eq!(
            total, 1,
            "expected exactly one occurrence of the flag being set, found {carrying_files:?}"
        );
        assert_eq!(
            carrying_files[0].0.file_name().and_then(|n| n.to_str()),
            Some("init.rs"),
            "the one site must be src/app/init.rs, found {:?}",
            carrying_files[0].0
        );
    }

    /// Every `.rs` file under `dir`, recursively — mirrors
    /// `src/ui/statusbar/tests.rs`'s own `walk_src` helper (each file that needs a
    /// whole-tree scan keeps its own private copy; nothing under `src/` may
    /// import `#[cfg(test)]` plumbing from a sibling module).
    fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("readable directory") {
            let path = entry.expect("readable entry").path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") && !path.ends_with("tests.rs") {
                out.push(path);
            }
        }
    }
}
