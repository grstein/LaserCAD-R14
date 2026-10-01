# LaserCAD 1.0 — parity with LaserCAD R14 v1

One row per v1 capability listed in [`README.md`](README.md) §Scope target (the v1 source is not
kept beside this repository, so that list is the reference). Each row names where the
capability lives in v2 and at least one test that proves it. The table is checked by
`tests/it/repo/release_1_0.rs`: one row per bullet, no empty cell, every cited test exists.
Commands and menus are described in [`../user-guide.md`](../user-guide.md).

| v1 capability | v2 command or menu | Tests |
|---|---|---|
| Drawing tools: Line, Polyline, Rect, Circle, Arc | `LINE` (`L`), `PLINE` (`P`), `RECT` (`R`), `CIRCLE` (`C`), `ARC` (`A`); rail and Tools menu | `roadmap_sequence_draws_two_lines`, `typed_polyline_commits_exact_coordinates`, `typed_rect_commits_exact_coordinates`, `mouse_free_circle_commits`, `typed_arc_commits_exact_geometry` |
| Modify tools: Select (point, window, crossing), Move, Trim, Extend, Delete | `SELECT` (`S`), `MOVE` (`M`), `TRIM` (`T`), `EXTEND` (`X`), `ERASE` / Delete (`E`, Del); rail and Tools menu | `pick_closest_returns_index_when_within_radius`, `entity_in_window_line_inside_vs_crossing`, `typed_move_relocates_the_selection`, `trim_line_line_keep_left_side`, `extend_line_line_to_intersection`, `delete_commits_one_undoable_command` |
| Snaps: endpoint, midpoint, center, intersection | Snap (F3, `snap`), View > Object Snap | `snap_endpoint_of_line`, `snap_midpoint_of_line`, `snap_center_of_circle`, `snap_intersection_of_two_lines` |
| Undo/Redo (200-deep) | Edit > Undo / Redo (Ctrl+Z / Ctrl+Y) | `depth_cap_evicts_oldest`, `history_constructors_match_depth_constant`, `undo_marks_document_dirty` |
| Command line: absolute `X,Y`, relative `@X,Y`, distance, tool aliases, toggles, agent prefix | Command line: `X,Y`, `@X,Y`, `@d<a`, distances, command words, `snap` / `grid` / `ortho`, `:` and `/ai` | `parses_absolute_points`, `parses_relative_offsets`, `parses_bare_distances`, `parses_tool_aliases`, `parses_toggles`, `a_turn_started_from_the_command_line_draws_on_the_real_bed` |
| SVG export: cut / mark / engrave presets, LaserGRBL-compatible | Layers replace the presets (ADR 0012): Format > Layers…, File > Save writes the mother SVG, File > Export Layers writes one file per Output layer | `mother_svg_writes_one_group_per_layer_in_order`, `layer_export_is_byte_exact`, `contract_document_exports_the_frozen_bytes` |
| SVG import: strict subset (what v2 emits, plus tolerant whitespace) | File > Open…, Open Recent; reads every LaserCAD file and general SVG | `mother_svg_round_trips_layers_membership_and_current`, `the_v03_mother_seed_reopens_exactly`, `contract_fixture_reimports_within_format_tolerance` |
| TEXT command: ASCII strings as engravable line geometry via Hershey font | `TEXT` (`D`, `text`) | `text_flow_commits_ten_millimetre_text_in_one_undo_step`, `enter_at_the_height_prompt_uses_five_millimetres` |
| Autosave: debounced 800 ms, restore on boot | Automatic; restored at start-up | `autosave_debounce_is_800ms`, `recovered_from_autosave_is_set_inside_the_recovered_branch`, `autosave_round_trips_layers_current_and_membership` |
| Native dialogs (Open / Save As) | File > Open… (Ctrl+O), Save As… (Ctrl+Shift+S) | `dialogs_are_disarmed_by_default`, `run_arms_native_dialogs_as_its_first_statement`, `open_via_the_dialog_adopts_the_file_bed_and_leaves_the_seed_alone` |
| Recent files list | File > Open Recent | `recent_files_with_entries`, `open_recent_promotes_to_front` |
| Agent harness: OpenAI-compatible LLM (default OpenRouter), opt-in, multi-turn loop with iteration cap | AI panel (rail **AI** button), Help > AI Settings…, `:` / `/ai` prefix | `agent_model_and_step_budget_defaults`, `without_a_key_a_prefixed_line_is_refused_verbatim`, `the_next_request_carries_the_user_message_as_memory_keeps_it`, `a_relentless_endpoint_stops_at_the_budget` |
