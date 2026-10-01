//! Outline fonts for SVG `<text>` import (LCV-179, ADR 0017): a lazily
//! built [`fontdb::Database`] and CSS `font-family` resolution.
//!
//! Generic families are set when the database is built: `sans-serif` is
//! the first installed of [`SANS`], else the alphabetically first family;
//! `serif` ([`SERIF`]) and `monospace` ([`MONO`]) likewise, else the
//! sans-serif choice; `cursive` and `fantasy` are the sans-serif choice.
//! Family names match ASCII case-insensitively; among a family's faces the
//! nearest weight and style win (fontdb's CSS matching).
//!
//! Kernel-pure: MUST NOT import `egui`, `eframe`, or `rfd`.

use std::path::PathBuf;
use std::sync::OnceLock;

/// Preferred `sans-serif` families, in order.
const SANS: [&str; 5] = [
    "Liberation Sans",
    "DejaVu Sans",
    "Noto Sans",
    "Arial",
    "Helvetica",
];

/// Preferred `serif` families, in order.
const SERIF: [&str; 5] = [
    "Liberation Serif",
    "DejaVu Serif",
    "Noto Serif",
    "Times New Roman",
    "Times",
];

/// Preferred `monospace` families, in order.
const MONO: [&str; 5] = [
    "Liberation Mono",
    "DejaVu Sans Mono",
    "Noto Sans Mono",
    "Courier New",
    "Courier",
];

/// Where a [`FontBook`] finds its faces.
#[derive(Debug, Clone)]
enum Origin {
    /// No faces at all.
    Empty,
    /// fontdb's fixed system font directories.
    System,
    /// These font files, read once.
    Files(Vec<PathBuf>),
}

/// A face in a [`FontBook`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FaceId(fontdb::ID);

/// The fonts SVG `<text>` import draws with, loaded on the first query.
#[derive(Debug)]
pub struct FontBook {
    origin: Origin,
    db: OnceLock<fontdb::Database>,
}

impl FontBook {
    /// A book with no faces: every query is `None` (ADR 0006: nothing read).
    pub fn empty() -> Self {
        Self::with(Origin::Empty)
    }

    /// The system fonts, scanned on the first query, not here.
    pub fn system() -> Self {
        Self::with(Origin::System)
    }

    /// The faces of `paths`, read on the first query; unreadable files are
    /// skipped.
    pub fn from_files(paths: &[PathBuf]) -> Self {
        Self::with(Origin::Files(paths.to_vec()))
    }

    fn with(origin: Origin) -> Self {
        Self {
            origin,
            db: OnceLock::new(),
        }
    }

    /// The face for the CSS `font-family` list `families` at `weight`
    /// (100–900) and `italic`, and whether it is a substitute: the first
    /// listed family installed (a generic maps to its default), else the
    /// `sans-serif` default with `true`. `None` only when the book is empty.
    pub fn face(&self, families: &str, weight: u16, italic: bool) -> Option<(FaceId, bool)> {
        let db = self.db();
        let style = if italic {
            fontdb::Style::Italic
        } else {
            fontdb::Style::Normal
        };
        let query = |family: fontdb::Family<'_>| {
            let families = [family];
            let q = fontdb::Query {
                families: &families,
                weight: fontdb::Weight(weight),
                stretch: fontdb::Stretch::Normal,
                style,
            };
            db.query(&q).map(FaceId)
        };
        for (name, quoted) in split_families(families) {
            let generic = match name.to_ascii_lowercase().as_str() {
                _ if quoted => None,
                "serif" => Some(fontdb::Family::Serif),
                "sans-serif" | "cursive" | "fantasy" => Some(fontdb::Family::SansSerif),
                "monospace" => Some(fontdb::Family::Monospace),
                _ => None,
            };
            let found = match generic {
                Some(g) => query(g),
                None => installed(db, &name).and_then(|n| query(fontdb::Family::Name(n))),
            };
            if let Some(id) = found {
                return Some((id, false));
            }
        }
        query(fontdb::Family::SansSerif).map(|id| (id, true))
    }

    /// Run `f` on the parsed face `id`; `None` when its data is unreadable.
    pub fn with_face<T>(
        &self,
        id: FaceId,
        f: impl FnOnce(&ttf_parser::Face<'_>) -> T,
    ) -> Option<T> {
        self.db()
            .with_face_data(id.0, |data, index| {
                ttf_parser::Face::parse(data, index)
                    .ok()
                    .map(|face| f(&face))
            })
            .flatten()
    }

    fn db(&self) -> &fontdb::Database {
        self.db.get_or_init(|| build(&self.origin))
    }
}

/// The database of `origin` with its generic families set.
fn build(origin: &Origin) -> fontdb::Database {
    let mut db = fontdb::Database::new();
    match origin {
        Origin::Empty => {}
        Origin::System => db.load_system_fonts(),
        Origin::Files(paths) => {
            for data in paths.iter().filter_map(|p| std::fs::read(p).ok()) {
                db.load_font_data(data);
            }
        }
    }
    let first = db
        .faces()
        .filter_map(|f| f.families.first().map(|(name, _)| name.clone()))
        .min();
    let pick = |list: &[&str]| {
        list.iter()
            .find_map(|n| installed(&db, n).map(str::to_owned))
    };
    let sans = pick(&SANS).or(first).unwrap_or_default();
    let serif = pick(&SERIF).unwrap_or_else(|| sans.clone());
    let mono = pick(&MONO).unwrap_or_else(|| sans.clone());
    db.set_sans_serif_family(sans);
    db.set_serif_family(serif);
    db.set_monospace_family(mono);
    db
}

/// The installed spelling of family `name` (ASCII case-insensitive).
fn installed<'a>(db: &'a fontdb::Database, name: &str) -> Option<&'a str> {
    db.faces()
        .flat_map(|f| f.families.iter())
        .map(|(family, _)| family.as_str())
        .find(|family| family.eq_ignore_ascii_case(name))
}

/// The family names of a CSS `font-family` value with whether each was
/// quoted: quoted names as written, unquoted ones with inner whitespace
/// collapsed (only those can be generic keywords).
fn split_families(value: &str) -> Vec<(String, bool)> {
    value
        .split(',')
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(|t| {
            let quoted = ['"', '\'']
                .iter()
                .find_map(|&q| t.strip_prefix(q).and_then(|s| s.strip_suffix(q)));
            match quoted {
                Some(name) => (name.to_owned(), true),
                None => (t.split_whitespace().collect::<Vec<_>>().join(" "), false),
            }
        })
        .collect()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// The bundled OFL test font, "LCV Test Sans" Regular and Bold.
    pub(crate) fn book() -> FontBook {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fonts");
        FontBook::from_files(&[
            dir.join("LCVTestSans-Regular.ttf"),
            dir.join("LCVTestSans-Bold.ttf"),
        ])
    }

    fn bold(book: &FontBook, id: FaceId) -> bool {
        book.with_face(id, |f| f.is_bold()).unwrap()
    }

    /// AC 2 — the first installed family of a list, case-insensitively.
    #[test]
    fn first_installed_family_of_a_list_wins() {
        let book = book();
        for list in [
            "Nope, LCV Test Sans",
            "'Nope' , \"lcv test sans\"",
            "LCV   Test Sans",
        ] {
            let (id, substituted) = book.face(list, 400, false).unwrap();
            assert!(!substituted, "{list}");
            assert!(!bold(&book, id), "{list}");
        }
    }

    /// AC 2 — generics map to the database default, not a substitution.
    #[test]
    fn generic_families_map_to_the_default() {
        let book = book();
        for list in ["sans-serif", "Nope, serif", "MONOSPACE", "cursive"] {
            let (_, substituted) = book.face(list, 400, false).unwrap();
            assert!(!substituted, "{list}");
        }
        // A quoted generic keyword is a family name, and not installed.
        assert!(book.face("'serif'", 400, false).unwrap().1);
    }

    /// AC 2 — weight and style pick the nearest face.
    #[test]
    fn weight_and_style_pick_the_nearest_face() {
        let book = book();
        let pick = |w, italic| bold(&book, book.face("LCV Test Sans", w, italic).unwrap().0);
        assert!(pick(700, false));
        assert!(pick(900, false));
        assert!(pick(600, false));
        assert!(!pick(400, false));
        assert!(!pick(100, false));
        assert!(!pick(400, true), "no italic face: the regular one");
        assert!(pick(700, true), "no bold italic face: the bold one");
    }

    /// AC 3 — no listed family installed: the sans-serif default, substituted.
    #[test]
    fn unknown_family_is_substituted_by_the_default() {
        let book = book();
        let (id, substituted) = book.face("Nope, 'Also Nope'", 700, false).unwrap();
        assert!(substituted);
        assert!(bold(&book, id));
        assert!(book.face("", 400, false).unwrap().1);
    }

    /// AC 3 — an empty book has no face at all.
    #[test]
    fn empty_book_has_no_face() {
        assert_eq!(FontBook::empty().face("sans-serif", 400, false), None);
        let missing = FontBook::from_files(&[PathBuf::from("/nonexistent/font.ttf")]);
        assert_eq!(missing.face("serif", 400, false), None);
    }
}
