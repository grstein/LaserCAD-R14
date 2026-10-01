//! The `.expected` line format of the SVG corpus (LCV-170 AC 1).
//!
//! One record per line; `#` at line start, or followed by a blank or the end
//! of the line, starts a comment (so `#rrggbb` is a color, not a comment).
//! Numbers are world millimetres, angles degrees.
//!
//! ```text
//! bed 300 180
//! layer "Cut" #ff0000 output=1 current=1   # document order; index = position;
//!                                            # exactly one layer has current=1
//! line 0 x1 y1 x2 y2                        # first field = layer index
//! circle 0 cx cy r
//! arc 1 cx cy r start_deg end_deg ccw|cw
//! ellipse 0 cx cy rx ry rotation_deg [start_deg end_deg ccw|cw]
//!                                            # LCV-176: span angles parametric
//! error MalformedLayer                      # alone: import must fail so
//! ignored 2 path (unsupported data)         # LCV-171: one import-report entry,
//!                                            # count first, label to end of line;
//!                                            # in report order; none = empty report
//! ```

use lasercad::document::layer::parse_color_hex;

/// The `SvgImportError` variant names an `error` record may name.
pub const ERROR_VARIANTS: [&str; 5] = [
    "XmlParse",
    "NoSvgRoot",
    "MalformedAttribute",
    "MalformedBedDimension",
    "MalformedLayer",
];

/// One expected layer, in document order.
#[derive(Clone, Debug, PartialEq)]
pub struct ExpLayer {
    pub name: String,
    pub color: [u8; 3],
    pub output: bool,
    pub current: bool,
}

/// One expected entity; `layer` is an index into [`Expected::Doc::layers`].
/// Arc angles are radians.
#[derive(Clone, Debug, PartialEq)]
pub enum ExpEntity {
    Line {
        layer: usize,
        p: [f64; 4],
    },
    Circle {
        layer: usize,
        c: [f64; 2],
        r: f64,
    },
    Arc {
        layer: usize,
        c: [f64; 2],
        r: f64,
        start: f64,
        end: f64,
        ccw: bool,
    },
    /// LCV-176: `span` is `(start, end, ccw)` in parametric radians.
    Ellipse {
        layer: usize,
        c: [f64; 2],
        rx: f64,
        ry: f64,
        rotation: f64,
        span: Option<(f64, f64, bool)>,
    },
}

/// A parsed `.expected` file.
#[derive(Clone, Debug, PartialEq)]
pub enum Expected {
    /// Import succeeds and yields exactly this document.
    Doc {
        bed: [f64; 2],
        layers: Vec<ExpLayer>,
        entities: Vec<ExpEntity>,
        /// The import report, `(label, count)` in order (LCV-171).
        report: Vec<(String, usize)>,
    },
    /// Import fails with this `SvgImportError` variant.
    Error(String),
}

/// Parse a `.expected` text; the `Err` names the offending line number.
pub fn parse(text: &str) -> Result<Expected, String> {
    let mut bed = None;
    let mut layers = Vec::new();
    let mut entities = Vec::new();
    let mut report = Vec::new();
    let mut error = None;
    let mut records = 0;
    for (i, raw) in text.lines().enumerate() {
        let n = i + 1;
        let toks = tokens(raw).map_err(|e| format!("line {n}: {e}"))?;
        let Some((head, rest)) = toks.split_first() else {
            continue;
        };
        records += 1;
        let at = |e: String| format!("line {n}: {e}");
        match head.as_str() {
            "bed" if bed.is_none() => {
                let v = numbers(rest, 2).map_err(at)?;
                bed = Some([v[0], v[1]]);
            }
            "layer" => layers.push(layer(rest).map_err(at)?),
            "line" => {
                let (l, v) = indexed(rest, 4).map_err(at)?;
                entities.push(ExpEntity::Line {
                    layer: l,
                    p: [v[0], v[1], v[2], v[3]],
                });
            }
            "circle" => {
                let (l, v) = indexed(rest, 3).map_err(at)?;
                entities.push(ExpEntity::Circle {
                    layer: l,
                    c: [v[0], v[1]],
                    r: v[2],
                });
            }
            "arc" => entities.push(arc(rest).map_err(at)?),
            "ellipse" => entities.push(ellipse(rest).map_err(at)?),
            "ignored" => report.push(ignored(rest).map_err(at)?),
            "error" if rest.len() == 1 && ERROR_VARIANTS.contains(&rest[0].as_str()) => {
                error = Some(rest[0].clone());
            }
            other => return Err(at(format!("unknown or malformed record {other:?}"))),
        }
    }
    if let Some(variant) = error {
        return if records == 1 {
            Ok(Expected::Error(variant))
        } else {
            Err("an `error` record must be the only record".to_owned())
        };
    }
    let bed = bed.ok_or("no `bed` record")?;
    if layers.is_empty() {
        return Err("no `layer` record".to_owned());
    }
    if layers.iter().filter(|l| l.current).count() != 1 {
        return Err("exactly one layer must have current=1".to_owned());
    }
    let layer_of = |e: &ExpEntity| match e {
        ExpEntity::Line { layer, .. }
        | ExpEntity::Circle { layer, .. }
        | ExpEntity::Arc { layer, .. }
        | ExpEntity::Ellipse { layer, .. } => *layer,
    };
    if let Some(e) = entities.iter().find(|e| layer_of(e) >= layers.len()) {
        return Err(format!("entity names missing layer {}", layer_of(e)));
    }
    Ok(Expected::Doc {
        bed,
        layers,
        entities,
        report,
    })
}

/// `ignored <count> <label…>`: a positive count, then the label as the rest
/// of the line (tokens joined by one space).
fn ignored(toks: &[String]) -> Result<(String, usize), String> {
    let (count, label) = toks.split_first().ok_or("ignored takes <count> <label>")?;
    let count = count
        .parse::<usize>()
        .ok()
        .filter(|&n| n > 0)
        .ok_or_else(|| format!("{count:?} is not a positive count"))?;
    if label.is_empty() {
        return Err("ignored needs a label".to_owned());
    }
    Ok((label.join(" "), count))
}

/// Split one line into tokens; `"…"` is one token (`\"` and `\\` escaped).
fn tokens(line: &str) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    let mut chars = line.chars().peekable();
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
        } else if c == '"' {
            chars.next();
            let mut s = String::from('"');
            loop {
                match chars.next() {
                    None => return Err("unterminated quoted name".to_owned()),
                    Some('"') => break,
                    Some('\\') => s.push(chars.next().ok_or("dangling `\\`")?),
                    Some(ch) => s.push(ch),
                }
            }
            out.push(s);
        } else {
            let mut s = String::new();
            while let Some(&ch) = chars.peek().filter(|ch| !ch.is_whitespace()) {
                s.push(ch);
                chars.next();
            }
            if s == "#" || (s.starts_with('#') && out.is_empty()) {
                break;
            }
            out.push(s);
        }
    }
    Ok(out)
}

fn number(tok: &str) -> Result<f64, String> {
    tok.parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
        .ok_or_else(|| format!("{tok:?} is not a finite number"))
}

fn numbers(toks: &[String], count: usize) -> Result<Vec<f64>, String> {
    if toks.len() != count {
        return Err(format!("expected {count} numbers, found {}", toks.len()));
    }
    toks.iter().map(|t| number(t)).collect()
}

fn index(tok: &str) -> Result<usize, String> {
    tok.parse()
        .map_err(|_| format!("{tok:?} is not a layer index"))
}

fn indexed(toks: &[String], count: usize) -> Result<(usize, Vec<f64>), String> {
    let (first, rest) = toks.split_first().ok_or("missing layer index")?;
    Ok((index(first)?, numbers(rest, count)?))
}

fn flag(tok: Option<&String>, key: &str) -> Result<bool, String> {
    match tok
        .and_then(|t| t.strip_prefix(key))
        .and_then(|t| t.strip_prefix('='))
    {
        Some("1") => Ok(true),
        Some("0") => Ok(false),
        _ => Err(format!("expected {key}=0|1")),
    }
}

fn layer(toks: &[String]) -> Result<ExpLayer, String> {
    if !(3..=4).contains(&toks.len()) {
        return Err("layer takes \"name\" #rrggbb output=0|1 [current=0|1]".to_owned());
    }
    let name = toks[0]
        .strip_prefix('"')
        .ok_or("layer name must be quoted")?;
    let color = parse_color_hex(&toks[1]).ok_or_else(|| format!("{:?} is not #rrggbb", toks[1]))?;
    let output = flag(toks.get(2), "output")?;
    let current = match toks.get(3) {
        None => false,
        some => flag(some, "current")?,
    };
    Ok(ExpLayer {
        name: name.to_owned(),
        color,
        output,
        current,
    })
}

fn arc(toks: &[String]) -> Result<ExpEntity, String> {
    let (dir, rest) = toks.split_last().ok_or("missing arc fields")?;
    let ccw = match dir.as_str() {
        "ccw" => true,
        "cw" => false,
        other => return Err(format!("{other:?} is not ccw|cw")),
    };
    let (layer, v) = indexed(rest, 5)?;
    Ok(ExpEntity::Arc {
        layer,
        c: [v[0], v[1]],
        r: v[2],
        start: v[3].to_radians(),
        end: v[4].to_radians(),
        ccw,
    })
}

/// `ellipse <layer> cx cy rx ry rotation_deg`, then optionally
/// `start_deg end_deg ccw|cw`.
fn ellipse(toks: &[String]) -> Result<ExpEntity, String> {
    let (span, rest) = match toks.split_last() {
        Some((dir, rest)) if toks.len() == 9 => {
            let ccw = match dir.as_str() {
                "ccw" => true,
                "cw" => false,
                other => return Err(format!("{other:?} is not ccw|cw")),
            };
            (Some(ccw), rest)
        }
        _ => (None, toks),
    };
    let (layer, v) = indexed(rest, if span.is_some() { 7 } else { 5 })?;
    Ok(ExpEntity::Ellipse {
        layer,
        c: [v[0], v[1]],
        rx: v[2],
        ry: v[3],
        rotation: v[4].to_radians(),
        span: span.map(|ccw| (v[5].to_radians(), v[6].to_radians(), ccw)),
    })
}

/// LCV-176: a full ellipse and an elliptical arc record.
#[test]
fn parses_ellipse_records() {
    let text = "bed 10 10\nlayer \"Cut\" #ff0000 output=1 current=1\n\
        ellipse 0 1 2 3 4 30\n\
        ellipse 0 1 2 3 4 -90 180 0 cw\n";
    let Expected::Doc { entities, .. } = parse(text).unwrap() else {
        panic!("expected a document");
    };
    let rad = f64::to_radians;
    assert_eq!(
        entities,
        [
            ExpEntity::Ellipse {
                layer: 0,
                c: [1.0, 2.0],
                rx: 3.0,
                ry: 4.0,
                rotation: rad(30.0),
                span: None,
            },
            ExpEntity::Ellipse {
                layer: 0,
                c: [1.0, 2.0],
                rx: 3.0,
                ry: 4.0,
                rotation: rad(-90.0),
                span: Some((rad(180.0), 0.0, false)),
            },
        ]
    );
    assert!(parse("bed 1 1\nlayer \"C\" #ff0000 output=1 current=1\nellipse 0 1 2 3\n").is_err());
}

#[test]
fn parses_every_record_kind_and_comments() {
    let text = "# header comment\n\
        bed 300 180\n\
        layer \"Cut \\\"x\\\"\" #FF0000 output=1 current=1 # trailing\n\
        layer \"Off\" #0000ff output=0\n\
        \n\
        line 0 1 2 3 4.5\n\
        circle 1 5 6 7\n\
        arc 0 10 20 5 90 180 cw\n";
    let Expected::Doc {
        bed,
        layers,
        entities,
        report,
    } = parse(text).unwrap()
    else {
        panic!("expected a document");
    };
    assert_eq!(bed, [300.0, 180.0]);
    assert!(report.is_empty(), "no `ignored` line is an empty report");
    assert_eq!(layers.len(), 2);
    assert_eq!(
        (
            layers[0].name.as_str(),
            layers[0].color,
            layers[0].output,
            layers[0].current
        ),
        ("Cut \"x\"", [255, 0, 0], true, true)
    );
    assert_eq!(
        (
            layers[1].name.as_str(),
            layers[1].color,
            layers[1].output,
            layers[1].current
        ),
        ("Off", [0, 0, 255], false, false)
    );
    assert_eq!(
        entities[0],
        ExpEntity::Line {
            layer: 0,
            p: [1.0, 2.0, 3.0, 4.5]
        }
    );
    assert_eq!(
        entities[1],
        ExpEntity::Circle {
            layer: 1,
            c: [5.0, 6.0],
            r: 7.0
        }
    );
    let ExpEntity::Arc {
        layer,
        c,
        r,
        start,
        end,
        ccw,
    } = entities[2]
    else {
        panic!("expected an arc");
    };
    assert_eq!((layer, c, r, ccw), (0, [10.0, 20.0], 5.0, false));
    assert!((start - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
    assert!((end - std::f64::consts::PI).abs() < 1e-12);
}

#[test]
fn parses_a_lone_error_record() {
    let text = "# refused\nerror MalformedLayer\n";
    assert_eq!(
        parse(text),
        Ok(Expected::Error("MalformedLayer".to_owned()))
    );
}

#[test]
fn malformed_lines_are_errors_naming_the_line() {
    let base = "bed 300 180\nlayer \"Cut\" #ff0000 output=1 current=1\n";
    for (bad, line) in [
        ("bed 300\n", 1),
        ("layer Cut #ff0000 output=1\n", 3),
        ("layer \"Cut #ff0000 output=1\n", 3),
        ("layer \"X\" red output=1\n", 3),
        ("layer \"X\" #00ff00 output=2\n", 3),
        ("line 0 1 2 3\n", 3),
        ("line x 1 2 3 4\n", 3),
        ("circle 0 1 2 inf\n", 3),
        ("arc 0 1 2 3 4 5 up\n", 3),
        ("ignored image\n", 3),
        ("ignored 0 image\n", 3),
        ("ignored 2\n", 3),
        ("polygon 0 1 2\n", 3),
        ("error NotAVariant\n", 3),
    ] {
        let text = if line == 1 {
            bad.to_owned()
        } else {
            format!("{base}{bad}")
        };
        let err = parse(&text).expect_err(bad);
        assert!(
            err.starts_with(&format!("line {line}:")),
            "{bad:?} -> {err}"
        );
    }
}

#[test]
fn incomplete_files_are_errors() {
    for bad in [
        "",
        "# only a comment\n",
        "layer \"Cut\" #ff0000 output=1\n",
        "bed 300 180\n",
        "bed 300 180\nlayer \"Cut\" #ff0000 output=1\n",
        "bed 300 180\nlayer \"Cut\" #ff0000 output=1 current=1\nline 1 0 0 1 1\n",
        "bed 300 180\nerror MalformedLayer\n",
    ] {
        assert!(parse(bad).is_err(), "{bad:?}");
    }
}

/// LCV-171 — `ignored` lines keep their order; the label runs to the end of
/// the line and a trailing comment is not part of it.
#[test]
fn parses_ignored_lines_in_order() {
    let text = "bed 300 180\n\
        layer \"Cut\" #ff0000 output=1 current=1\n\
        ignored 1 defs\n\
        ignored 2 path (unsupported data)  # two paths\n\
        ignored 1 transform\n";
    let Ok(Expected::Doc { report, .. }) = parse(text) else {
        panic!("expected a document");
    };
    let want = [
        ("defs", 1),
        ("path (unsupported data)", 2),
        ("transform", 1),
    ]
    .map(|(label, count)| (label.to_owned(), count));
    assert_eq!(report, want);
}
