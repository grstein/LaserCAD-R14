//! Layer `<g>` attributes in the mother SVG (LCV-156, ADR 0012 §4).
//!
//! Written: `<g data-layer="<name>" stroke="#rrggbb" stroke-width="0.1"
//! data-output="1|0"[ data-current="1"]>`, the name XML-escaped, no `id`.
//! Read: a `<g>` with `data-layer` defines a layer in document order; its
//! stroke is any CSS color (`super::css_color`), from `style="stroke:…"` over
//! the `stroke` attribute, else inherited from the nearest `<g>`/`<svg>`
//! ancestor (AC 16); `data-output` is `0`/`1` (absent = `1`);
//! duplicate name keys or colors are refused. Anything else is
//! [`SvgImportError::MalformedLayer`]. Geometry outside every layer group is
//! placed by color in [`LayerReader::finish`] (LCV-175).
//!
//! Kernel-pure: MUST NOT import `egui`, `eframe`, or `rfd`.

use super::css_color::parse_css_color;
use super::import::SvgImportError;
use super::import::report::style_decls;
use crate::document::layer::{check_fields, color_hex, name_key};
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

/// A layer group's stroke value: its own, else the nearest `<g>`/`<svg>`
/// ancestor's (ADR 0012 §4).
fn layer_stroke<'a>(node: roxmltree::Node<'a, '_>) -> Option<&'a str> {
    node.ancestors()
        .filter(|n| matches!(n.tag_name().name(), "g" | "svg"))
        .find_map(own_stroke)
}

/// The stroke `node` declares itself: the last `stroke` in `style` wins over
/// the `stroke` attribute; `inherit` or no declaration is `None`.
fn own_stroke<'a>(node: roxmltree::Node<'a, '_>) -> Option<&'a str> {
    let styled = style_decls(node)
        .rfind(|(prop, _)| prop.eq_ignore_ascii_case("stroke"))
        .map(|(_, value)| value);
    styled
        .or_else(|| node.attribute("stroke"))
        .filter(|value| !value.trim().eq_ignore_ascii_case("inherit"))
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
        let stroke = layer_stroke(node).ok_or_else(|| bad("no stroke color".to_owned()))?;
        let color = parse_css_color(stroke)
            .ok_or_else(|| bad(format!("stroke {stroke:?} is not a supported CSS color")))?;
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

    /// The layers, the current layer and each slot's layer (ADR 0012 §4,
    /// LCV-175 AC 8–10). The default `Cut` layer comes first when the file
    /// declared none and some slot is [`Slot::First`] or no slot has a
    /// color; each [`Slot::Color`] takes the first layer of that exact color,
    /// else a new `#rrggbb` layer (Output on) appended in order of first
    /// appearance. The first layer is `LayerId(0)` (ids are positions).
    pub(super) fn finish(self, slots: &[Slot]) -> (Vec<Layer>, LayerId, Vec<LayerId>) {
        let mut layers = self.layers;
        let first = slots.contains(&Slot::First);
        let colored = slots.iter().any(|s| matches!(s, Slot::Color(_)));
        if layers.is_empty() && (first || !colored) {
            layers.push(Layer::default_cut());
        }
        let mut ids = Vec::with_capacity(slots.len());
        for slot in slots {
            ids.push(match *slot {
                Slot::Layer(id) => id,
                Slot::First => LayerId(0),
                Slot::Color(rgb) => color_layer(&mut layers, rgb),
            });
        }
        (layers, self.current.unwrap_or(LayerId(0)), ids)
    }
}

/// Where one imported entity belongs, resolved by [`LayerReader::finish`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Slot {
    /// Inside a `<g data-layer>`: that layer.
    Layer(LayerId),
    /// Outside any layer group, with this stroke (else fill) color.
    Color([u8; 3]),
    /// Outside any layer group, uncolored: the first layer.
    First,
}

/// The first layer colored `rgb`, else a new one named `#rrggbb` (then
/// `#rrggbb 2`, … while the name key is taken) with Output on.
fn color_layer(layers: &mut Vec<Layer>, rgb: [u8; 3]) -> LayerId {
    if let Some(layer) = layers.iter().find(|l| l.color == rgb) {
        return layer.id;
    }
    let hex = color_hex(rgb);
    let taken = |name: &str| layers.iter().any(|l| name_key(&l.name) == name_key(name));
    let mut name = hex.clone();
    for n in 2.. {
        if !taken(&name) {
            break;
        }
        name = format!("{hex} {n}");
    }
    let id = LayerId(layers.len() as u32);
    layers.push(Layer {
        id,
        name,
        color: rgb,
        output: true,
    });
    id
}
