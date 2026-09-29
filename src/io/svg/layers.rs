//! Layer `<g>` attributes in the mother SVG (LCV-156, ADR 0012 §4).
//!
//! Written: `<g data-layer="<name>" stroke="#rrggbb" stroke-width="0.1"
//! data-output="1|0"[ data-current="1"]>`, the name XML-escaped, no `id`.
//! Read: a `<g>` with `data-layer` defines a layer in document order; `stroke`
//! must be `#rrggbb` (either case), `data-output` `0`/`1` (absent = `1`);
//! duplicate name keys or colors are refused. Anything else is
//! [`SvgImportError::MalformedLayer`].
//!
//! Kernel-pure: MUST NOT import `egui`, `eframe`, or `rfd`.

use super::import::SvgImportError;
use crate::document::layer::{check_fields, color_hex, parse_color_hex};
use crate::document::{Layer, LayerId};

/// Stroke width of every layer group, in mm.
pub(super) const STROKE_WIDTH: &str = "0.1";

/// Append the opening tag of `layer`'s group, newline included.
pub(super) fn open_group(out: &mut String, layer: &Layer, current: bool) {
    out.push_str("<g data-layer=\"");
    out.push_str(&xml_escape(&layer.name));
    out.push_str("\" stroke=\"");
    out.push_str(&color_hex(layer.color));
    out.push_str("\" stroke-width=\"");
    out.push_str(STROKE_WIDTH);
    out.push_str(if layer.output {
        "\" data-output=\"1\""
    } else {
        "\" data-output=\"0\""
    });
    if current {
        out.push_str(" data-current=\"1\"");
    }
    out.push_str(">\n");
}

/// Escape the four characters that can break a double-quoted attribute.
fn xml_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

/// The layers read so far from one file, in document order.
#[derive(Debug, Default)]
pub(super) struct LayerReader {
    layers: Vec<Layer>,
    current: Option<LayerId>,
}

impl LayerReader {
    /// When `node` is a `<g data-layer>`, validate and record the layer and
    /// return its id; otherwise `Ok(None)`.
    pub(super) fn enter(
        &mut self,
        node: roxmltree::Node<'_, '_>,
    ) -> Result<Option<LayerId>, SvgImportError> {
        if node.tag_name().name() != "g" {
            return Ok(None);
        }
        let Some(name) = node.attribute("data-layer") else {
            return Ok(None);
        };
        let bad = |reason: String| SvgImportError::MalformedLayer {
            name: name.to_owned(),
            reason,
        };
        let stroke = node.attribute("stroke").unwrap_or("");
        let color = parse_color_hex(stroke)
            .ok_or_else(|| bad(format!("stroke {stroke:?} is not #rrggbb")))?;
        let output = match node.attribute("data-output") {
            None | Some("1") => true,
            Some("0") => false,
            Some(other) => return Err(bad(format!("data-output {other:?} is not 0 or 1"))),
        };
        check_fields(&self.layers, None, name, color).map_err(|e| bad(e.to_string()))?;
        let id = LayerId(self.layers.len() as u32);
        let name = name.to_owned();
        self.layers.push(Layer {
            id,
            name,
            color,
            output,
        });
        if self.current.is_none() && node.attribute("data-current") == Some("1") {
            self.current = Some(id);
        }
        Ok(Some(id))
    }

    /// The layers and current layer; the default `Cut` layer when the file
    /// declared none. The first layer is always `LayerId(0)`, which is where
    /// geometry outside any layer group belongs.
    pub(super) fn finish(self) -> (Vec<Layer>, LayerId) {
        let layers = if self.layers.is_empty() {
            vec![Layer::default_cut()]
        } else {
            self.layers
        };
        let first = layers[0].id; // Non-empty by construction just above.
        (layers, self.current.unwrap_or(first))
    }
}

/// The layer geometry outside any layer group lands on (ADR 0012 §4).
pub(super) const STRAY_LAYER: LayerId = LayerId(0);
