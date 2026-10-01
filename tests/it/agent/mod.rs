//! The LLM agent: turns, panel, settings, prompt, memory, observations.

mod bench;
mod bench_score;
mod canvas_capture;
mod check_drawing;
mod checkpoints;
mod default_prompt;
mod drawing_batch;
mod entity_ids;
mod feedback_after_changes;
mod layers;
mod measure;
mod memory;
mod new_conversation;
mod panel_and_settings;
mod panel_width_and_settings;
mod progress_row;
mod prompt_editor;
mod reference_image;
mod system_prompt;
mod timeout_and_cancel;
mod transform_tools;
mod turn;
mod turn_group;
mod turn_metrics;
mod verify_before_reply;
