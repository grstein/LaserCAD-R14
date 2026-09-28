//! The one integration-test binary (LCV-152).
//!
//! Every file beside this one is a module of this crate rather than its own
//! test binary, so the suite links against egui once instead of once per file.
//! The shared harness stays at `tests/harness/` and is compiled here exactly
//! once; each module that needs it says `use crate::harness;`.

#[path = "../harness/mod.rs"]
mod harness;

mod app_tool;
mod docs_examples_svg_roundtrip;
mod geometry;
mod lcv057_svg_import;
mod lcv070;
mod lcv100_svg_orientation;
mod lcv102;
mod lcv103;
mod lcv104;
mod lcv110;
mod lcv111;
mod lcv112;
mod lcv113;
mod lcv114_bed_dialog;
mod lcv114_bed_roundtrip;
mod lcv115_preset_roundtrip;
mod lcv115_preset_ui;
mod lcv116_autosave_repaint;
mod lcv116_shortcuts_dialog;
mod lcv116_statusbar;
mod lcv118;
mod lcv120_idle_repaint;
mod lcv121_source_scans;
mod lcv122_source_scans;
mod lcv123_agent_turn;
mod lcv124_command_line_routing;
mod lcv125_agent_panel_and_settings;
mod lcv126_command_line_group;
mod lcv128_normative_enumerations;
mod lcv129_agent_timeout_and_cancel;
mod lcv130_no_loopback_url_literals;
mod lcv131_command_words;
mod lcv132_harness_is_shared;
mod lcv133_shortcuts_dialog_fits;
mod lcv136_discard_dialog_pointer_click;
mod lcv137_viewport_grid_and_coordinates;
mod lcv138_document_title_and_file_feedback;
mod lcv139_command_line_context_row;
mod lcv140_compact_r14_chrome_and_action_hints;
mod lcv141_agent_panel_width_and_settings;
mod lcv142_progress_row;
mod lcv142_source_scans;
mod lcv142_turn_group;
mod lcv143_prompt_editor;
mod lcv143_source_scans;
mod lcv143_system_prompt;
mod lcv144_drawing_batch;
mod lcv145_canvas_capture;
mod lcv148_changelog_unreleased_clause;
mod lcv151_default_prompt;
mod lcv152_single_test_binary;
mod skeleton;
