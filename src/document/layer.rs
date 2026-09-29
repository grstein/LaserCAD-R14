//! Document layers (LCV-156, ADR 0012 §1): [`LayerId`], [`Layer`], [`LayerError`]
//! and the file-name key every layer name is compared and exported by.
//!
//! A layer is a named, colored bucket of entities with an Output flag. Two
//! layers may not share a name key ([`name_key`]) or a color, so a per-layer
//! export file name (`<mother>-<file_key>.svg`) never collides and the color
//! alone identifies the layer on the canvas.
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`. Part of the pure-Rust kernel.

use serde::{Deserialize, Serialize};

/// Maximum length, in characters, of a [`file_key`].
pub const FILE_KEY_MAX_CHARS: usize = 64;

/// Stable identity of a layer inside one document. Never shown to the operator.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LayerId(pub u32);

/// One layer: display name, stroke color and whether `File > Export layers`
/// writes a file for it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Layer {
    /// Stable identity.
    pub id: LayerId,
    /// Operator-facing name, as typed.
    pub name: String,
    /// Stroke color, `[r, g, b]`.
    pub color: [u8; 3],
    /// `true` when `File > Export layers` writes a file for this layer.
    pub output: bool,
}

impl Layer {
    /// The layer every new document starts with (AC 1): `Cut`, `#ff0000`,
    /// Output on, id 0.
    pub fn default_cut() -> Self {
        Self {
            id: LayerId(0),
            name: "Cut".to_owned(),
            color: [0xff, 0, 0],
            output: true,
        }
    }
}

/// Why a layer change was refused. `Display` is the operator-facing message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LayerError {
    /// Another layer already has this name (compared by [`name_key`]).
    DuplicateName(String),
    /// Another layer already has this color.
    DuplicateColor(String),
    /// The name has no character usable in a file name.
    EmptyName,
    /// The layer still has entities.
    NotEmpty(String),
    /// The layer is the only one left.
    LastLayer,
    /// No layer has this id or name.
    UnknownLayer(String),
}

impl std::fmt::Display for LayerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateName(n) => write!(f, "A layer named \"{n}\" already exists."),
            Self::DuplicateColor(c) => write!(f, "Another layer already uses color {c}."),
            Self::EmptyName => f.write_str("A layer name needs at least one letter or digit."),
            Self::NotEmpty(n) => write!(f, "Layer \"{n}\" still has entities; move them first."),
            Self::LastLayer => f.write_str("The drawing needs at least one layer."),
            Self::UnknownLayer(n) => write!(f, "No layer named \"{n}\"."),
        }
    }
}

impl std::error::Error for LayerError {}

/// The file-name form of a layer name: letters, digits, `-` and `_` are kept,
/// every other run of characters becomes one `_`, `_`/`-` are trimmed at both
/// ends, and the result is capped at [`FILE_KEY_MAX_CHARS`] characters.
/// An empty result means the name is unusable ([`LayerError::EmptyName`]).
pub fn file_key(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut in_run = false;
    for c in name.chars() {
        if c.is_alphanumeric() || c == '-' || c == '_' {
            out.push(c);
            in_run = false;
        } else if !in_run {
            out.push('_');
            in_run = true;
        }
    }
    let trim = |s: &str| s.trim_matches(['_', '-']).to_owned();
    let capped: String = trim(&out).chars().take(FILE_KEY_MAX_CHARS).collect();
    trim(&capped)
}

/// The key two layer names are compared by: [`file_key`], lowercased.
pub fn name_key(name: &str) -> String {
    file_key(name).to_lowercase()
}

/// `#rrggbb`, lowercase.
pub fn color_hex(color: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", color[0], color[1], color[2])
}

/// Parse `#rrggbb` (either case). Anything else is `None`.
pub fn parse_color_hex(text: &str) -> Option<[u8; 3]> {
    let hex = text.strip_prefix('#')?;
    if hex.len() != 6 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
    Some([byte(0)?, byte(2)?, byte(4)?])
}

/// Refuse `name`/`color` for a layer when it is unusable or clashes with any
/// layer in `layers` other than `skip` (LCV-156 AC 6).
pub fn check_fields(
    layers: &[Layer],
    skip: Option<LayerId>,
    name: &str,
    color: [u8; 3],
) -> Result<(), LayerError> {
    let key = name_key(name);
    if key.is_empty() {
        return Err(LayerError::EmptyName);
    }
    for other in layers.iter().filter(|l| Some(l.id) != skip) {
        if name_key(&other.name) == key {
            return Err(LayerError::DuplicateName(other.name.clone()));
        }
        if other.color == color {
            return Err(LayerError::DuplicateColor(color_hex(color)));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_layer_is_cut_red_with_output() {
        let l = Layer::default_cut();
        assert_eq!(
            (l.name.as_str(), l.color, l.output),
            ("Cut", [255, 0, 0], true)
        );
    }

    #[test]
    fn file_key_keeps_word_chars_and_collapses_runs() {
        assert_eq!(file_key("Cut"), "Cut");
        assert_eq!(file_key("Fine engrave #2"), "Fine_engrave_2");
        assert_eq!(file_key("a / b"), "a_b");
        assert_eq!(file_key("  --x--  "), "x");
        assert_eq!(file_key("Gravação"), "Gravação");
        assert_eq!(file_key("my_layer-1"), "my_layer-1");
    }

    #[test]
    fn file_key_empty_for_names_without_word_chars() {
        assert_eq!(file_key(""), "");
        assert_eq!(file_key("   "), "");
        assert_eq!(file_key("#/-_"), "");
    }

    #[test]
    fn file_key_caps_at_64_chars_and_retrims() {
        let long = "a".repeat(100);
        assert_eq!(file_key(&long).chars().count(), 64);
        let tail = format!("{}_b", "a".repeat(63));
        assert_eq!(file_key(&tail), "a".repeat(63));
    }

    #[test]
    fn name_key_is_case_insensitive() {
        assert_eq!(name_key("Cut Out"), name_key("cut/out"));
        assert_ne!(name_key("Cut"), name_key("Cuts"));
    }

    #[test]
    fn color_hex_round_trips_either_case() {
        assert_eq!(color_hex([255, 0, 0xaa]), "#ff00aa");
        assert_eq!(parse_color_hex("#FF00aa"), Some([255, 0, 0xaa]));
        for bad in [
            "ff00aa", "#ff00a", "#ff00aag", "#ff00aa0", "", "#gggggg", "#+f+f+f",
        ] {
            assert_eq!(parse_color_hex(bad), None, "{bad}");
        }
    }

    #[test]
    fn errors_say_why() {
        let msg = LayerError::DuplicateName("Cut".into()).to_string();
        assert!(msg.contains("Cut") && msg.contains("already"));
        assert!(
            LayerError::LastLayer
                .to_string()
                .contains("at least one layer")
        );
        assert!(
            LayerError::NotEmpty("Mark".into())
                .to_string()
                .contains("entities")
        );
    }
}
