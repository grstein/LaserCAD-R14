//! LCV-187 — `capture_canvas`'s schema and argument parse, split out of
//! `tools.rs` for the LOC cap. The schema stays flat and provider-safe (ADR
//! 0010 §2): an optional `frame` enum plus four optional corner numbers.
//!
//! Only the shape is checked here. Whether the frame has an area — an empty
//! drawing, a degenerate region — is the apply site's question, because only
//! it can see the drawing (ADR 0007 §D2a).
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`.

use serde_json::{Value, json};

use super::ToolCallError;
use crate::agent::bridge::CaptureFrame;

const TOOL: &str = "capture_canvas";

/// The region corners, in the order a missing one is reported.
const CORNERS: [&str; 4] = ["x0", "y0", "x1", "y1"];

/// `capture_canvas`: an optional `frame`, and the corners for `"region"`.
pub(super) fn definition() -> Value {
    let mm = json!({"type":"number"});
    json!({"type":"function","function":{"name":TOOL,
      "description":"Look at the drawing: returns a grayscale picture of the bed outline (grey) and every entity (black), with its mm mapping. frame \"view\" (default) is the operator's viewport; \"drawing\" is every entity plus a 5% margin; \"region\" is the rectangle between corners x0,y0 and x1,y1 in mm. \"drawing\" and \"region\" render the longest edge at 1024 px, so zoom in on details with a small region. No grid, selection or UI. Use query_entities for exact numbers.",
      "parameters":{"type":"object","properties":{
        "frame":{"type":"string","enum":["view","drawing","region"]},
        "x0":mm,"y0":mm,"x1":mm,"y1":mm},"required":[]}}})
}

/// `args[key]` unless absent or JSON `null`.
fn present<'a>(args: &'a Value, key: &str) -> Option<&'a Value> {
    args.get(key).filter(|v| !v.is_null())
}

fn invalid(field: &'static str, reason: &str) -> ToolCallError {
    ToolCallError::InvalidArg {
        tool: TOOL,
        field,
        reason: reason.to_owned(),
    }
}

/// The frame a `capture_canvas` call asks for. No `frame` (or `null`) is
/// `"view"`; a corner is required by `"region"` and refused by the others,
/// `null` counting as absent; any other key is ignored, as before LCV-187.
pub(super) fn parse(args: &Value) -> Result<CaptureFrame, ToolCallError> {
    let frame = match present(args, "frame") {
        None => "view",
        Some(v) => v
            .as_str()
            .filter(|name| ["view", "drawing", "region"].contains(name))
            .ok_or_else(|| invalid("frame", r#"must be "view", "drawing" or "region""#))?,
    };
    if frame == "region" {
        let corner = |field: &'static str| {
            args.get(field)
                .and_then(Value::as_f64)
                .ok_or(ToolCallError::MissingField { tool: TOOL, field })
        };
        return Ok(CaptureFrame::Region {
            x0: corner("x0")?,
            y0: corner("y0")?,
            x1: corner("x1")?,
            y1: corner("y1")?,
        });
    }
    if let Some(field) = CORNERS.into_iter().find(|f| present(args, f).is_some()) {
        return Err(invalid(field, r#"only used with frame "region""#));
    }
    Ok(if frame == "drawing" {
        CaptureFrame::Drawing
    } else {
        CaptureFrame::View
    })
}
