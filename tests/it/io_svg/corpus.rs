//! LCV-170 AC 1..4 — the SVG conformance corpus in `tests/fixtures/svg/`.
//!
//! Each `<name>.svg` is paired with a hand-written `<name>.expected` (format:
//! [`super::corpus_expected`]). The expectation is derived from the SVG text by hand, never
//! by running `import_svg`, so the corpus measures import against SVG 2 rather
//! than against LaserCAD's own exporter. The directory is listed at run time:
//! adding a fixture needs no code change. The import report is compared
//! too (LCV-171): an `.expected` with no `ignored` line expects none.

use std::collections::BTreeSet;
use std::f64::consts::TAU;
use std::path::{Path, PathBuf};

use super::corpus_expected::{self as expected, ExpEntity, ExpLayer, Expected};
use lasercad::document::{Document, Entity};
use lasercad::io::svg::{SvgImportError, import_svg};

/// Length tolerance, mm (AC 2).
const MM_TOL: f64 = 1e-6;
/// Angle tolerance, rad, compared modulo 2π (AC 2).
const RAD_TOL: f64 = 1e-9;

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/svg")
}

/// Import every `.svg` in `dir` and compare it with its `.expected`. Returns
/// one message per failure, each naming its file; empty means the corpus holds.
pub fn check_corpus(dir: &Path) -> Vec<String> {
    let Ok(read) = std::fs::read_dir(dir) else {
        return vec![format!("{}: cannot read corpus directory", dir.display())];
    };
    let mut stems = BTreeSet::new();
    for entry in read.flatten() {
        let path = entry.path();
        if let (Some(stem), Some(ext)) = (path.file_stem(), path.extension())
            && (ext == "svg" || ext == "expected")
        {
            stems.insert(stem.to_string_lossy().into_owned());
        }
    }
    let mut failures = Vec::new();
    for stem in &stems {
        let svg = std::fs::read_to_string(dir.join(format!("{stem}.svg")));
        let exp = std::fs::read_to_string(dir.join(format!("{stem}.expected")));
        match (svg, exp) {
            (Err(_), _) => failures.push(format!("{stem}.expected: no matching {stem}.svg")),
            (Ok(_), Err(_)) => failures.push(format!("{stem}.svg: no {stem}.expected")),
            (Ok(svg), Ok(exp)) => match expected::parse(&exp) {
                Err(e) => failures.push(format!("{stem}.expected: unparseable: {e}")),
                Ok(want) => failures.extend(
                    compare(&svg, &want)
                        .into_iter()
                        .map(|e| format!("{stem}.svg: {e}")),
                ),
            },
        }
    }
    failures
}

fn variant(e: &SvgImportError) -> &'static str {
    match e {
        SvgImportError::XmlParse(_) => "XmlParse",
        SvgImportError::NoSvgRoot => "NoSvgRoot",
        SvgImportError::MalformedAttribute { .. } => "MalformedAttribute",
        SvgImportError::MalformedPath(_) => "MalformedPath",
        SvgImportError::MalformedBedDimension { .. } => "MalformedBedDimension",
        SvgImportError::MalformedLayer { .. } => "MalformedLayer",
    }
}

/// Every mismatch between importing `svg` and `want`.
fn compare(svg: &str, want: &Expected) -> Vec<String> {
    let got = import_svg(svg).and_then(|imported| {
        let report = imported.report.clone();
        imported.into_document().map(|doc| (doc, report))
    });
    match (got, want) {
        (Err(e), Expected::Error(v)) if variant(&e) == v => Vec::new(),
        (Err(e), _) => vec![format!("import failed: {} ({e})", variant(&e))],
        (Ok(_), Expected::Error(v)) => vec![format!("import succeeded, expected {v}")],
        (
            Ok((doc, report)),
            Expected::Doc {
                bed,
                layers,
                entities,
                report: want_report,
            },
        ) => {
            let mut out = Vec::new();
            if !close(doc.bed_mm[0], bed[0]) || !close(doc.bed_mm[1], bed[1]) {
                out.push(format!("bed {:?}, expected {bed:?}", doc.bed_mm));
            }
            out.extend(compare_layers(&doc, layers));
            out.extend(compare_entities(&doc, entities));
            if &report != want_report {
                out.push(format!("report {report:?}, expected {want_report:?}"));
            }
            out
        }
    }
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= MM_TOL
}

fn same_angle(a: f64, b: f64) -> bool {
    let d = (a - b).rem_euclid(TAU);
    d.min(TAU - d) <= RAD_TOL
}

fn compare_layers(doc: &Document, want: &[ExpLayer]) -> Vec<String> {
    let got: Vec<ExpLayer> = doc
        .layers()
        .iter()
        .map(|l| ExpLayer {
            name: l.name.clone(),
            color: l.color,
            output: l.output,
            current: l.id == doc.current_layer(),
        })
        .collect();
    if got == want {
        Vec::new()
    } else {
        vec![format!("layers {got:?}, expected {want:?}")]
    }
}

fn compare_entities(doc: &Document, want: &[ExpEntity]) -> Vec<String> {
    if doc.entities.len() != want.len() {
        return vec![format!(
            "{} entities, expected {}",
            doc.entities.len(),
            want.len()
        )];
    }
    let layer_index = |i: usize| {
        doc.entity_layer(i)
            .and_then(|id| doc.layers().iter().position(|l| l.id == id))
    };
    let mut out = Vec::new();
    for (i, (got, want)) in doc.entities.iter().zip(want).enumerate() {
        let ok = match (got, want) {
            (Entity::Line(l), ExpEntity::Line { layer, p }) => {
                layer_index(i) == Some(*layer)
                    && [l.p1.x, l.p1.y, l.p2.x, l.p2.y]
                        .iter()
                        .zip(p)
                        .all(|(a, b)| close(*a, *b))
            }
            (Entity::Circle(c), ExpEntity::Circle { layer, c: ctr, r }) => {
                layer_index(i) == Some(*layer)
                    && close(c.center.x, ctr[0])
                    && close(c.center.y, ctr[1])
                    && close(c.r, *r)
            }
            (
                Entity::Arc(a),
                ExpEntity::Arc {
                    layer,
                    c,
                    r,
                    start,
                    end,
                    ccw,
                },
            ) => {
                layer_index(i) == Some(*layer)
                    && close(a.center.x, c[0])
                    && close(a.center.y, c[1])
                    && close(a.r, *r)
                    && same_angle(a.start_angle, *start)
                    && same_angle(a.end_angle, *end)
                    && a.ccw == *ccw
            }
            _ => false,
        };
        if !ok {
            out.push(format!(
                "entity {i}: {got:?} on layer {:?}, expected {want:?}",
                layer_index(i)
            ));
        }
    }
    out
}

fn scratch_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lcv170_{}_{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

const PAIR_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="50mm" viewBox="0 0 100 50"><line x1="1" y1="2" x2="3" y2="4"/></svg>"#;
const PAIR_EXPECTED: &str =
    "bed 100 50\nlayer \"Cut\" #ff0000 output=1 current=1\nline 0 1 48 3 46\n";

/// AC 2 — every corpus file imports to its hand-written expectation.
#[test]
fn corpus_matches_expected() {
    let failures = check_corpus(&fixtures_dir());
    assert!(
        failures.is_empty(),
        "corpus failures:\n{}",
        failures.join("\n")
    );
}

/// AC 2 — the runner itself accepts a matching pair and rejects a mismatch.
#[test]
fn runner_accepts_a_match_and_names_a_mismatch() {
    let dir = scratch_dir("match");
    std::fs::write(dir.join("ok.svg"), PAIR_SVG).unwrap();
    std::fs::write(dir.join("ok.expected"), PAIR_EXPECTED).unwrap();
    assert_eq!(check_corpus(&dir), Vec::<String>::new());
    let noisy = PAIR_SVG.replace("<line", "<image/><line");
    std::fs::write(dir.join("noisy.svg"), &noisy).unwrap();
    std::fs::write(
        dir.join("noisy.expected"),
        format!("{PAIR_EXPECTED}ignored 1 image\n"),
    )
    .unwrap();
    std::fs::write(dir.join("quiet.svg"), noisy).unwrap();
    std::fs::write(dir.join("quiet.expected"), PAIR_EXPECTED).unwrap();
    let failures = check_corpus(&dir);
    assert_eq!(failures.len(), 1, "{failures:?}");
    assert!(failures[0].starts_with("quiet.svg: report"), "{failures:?}");
    std::fs::remove_file(dir.join("quiet.svg")).unwrap();
    std::fs::remove_file(dir.join("quiet.expected")).unwrap();
    let off = PAIR_EXPECTED.replace("line 0 1 48", "line 0 1 47.9999");
    std::fs::write(dir.join("off.svg"), PAIR_SVG).unwrap();
    std::fs::write(dir.join("off.expected"), off).unwrap();
    let failures = check_corpus(&dir);
    assert_eq!(failures.len(), 1, "{failures:?}");
    assert!(failures[0].starts_with("off.svg: entity 0"), "{failures:?}");
}

/// AC 3 — an `.svg` with no `.expected`, or an unparseable one, fails the
/// corpus and names the file; so does an `.expected` with no `.svg`.
#[test]
fn an_svg_without_expected_fails_naming_it() {
    let dir = scratch_dir("orphans");
    std::fs::write(dir.join("lonely.svg"), PAIR_SVG).unwrap();
    std::fs::write(dir.join("garbled.svg"), PAIR_SVG).unwrap();
    std::fs::write(dir.join("garbled.expected"), "bed 100\n").unwrap();
    std::fs::write(dir.join("ghost.expected"), PAIR_EXPECTED).unwrap();
    let failures = check_corpus(&dir);
    assert_eq!(failures.len(), 3, "{failures:?}");
    assert!(failures[0].starts_with("garbled.expected: unparseable: line 1"));
    assert!(failures[1].starts_with("ghost.expected: no matching ghost.svg"));
    assert!(failures[2].starts_with("lonely.svg: no lonely.expected"));
}
