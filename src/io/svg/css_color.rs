//! CSS `<color>` values for a layer's stroke on import (LCV-156 AC 16,
//! ADR 0012 §4).
//!
//! Accepted, case-insensitive: the 148 named keywords of CSS Color 4, `#rgb`,
//! `#rrggbb`, and `rgb()`/`rgba()`/`hsl()`/`hsla()` in the comma and the
//! space (`/ alpha`) syntax. Channels are clamped to their range; an alpha is
//! checked and dropped, since a layer color is opaque. Anything else —
//! `none`, `currentColor`, `transparent`, `#rgba`, `#rrggbbaa`, `url(…)` — is
//! `None`, which the caller refuses as a malformed layer.
//!
//! Kernel-pure: MUST NOT import `egui`, `eframe`, or `rfd`.

use std::f64::consts::PI;

/// The named colors, `name:rrggbb`, sorted (CSS Color 4 §6.1).
const NAMED: &str = "\
aliceblue:f0f8ff antiquewhite:faebd7 aqua:00ffff aquamarine:7fffd4 azure:f0ffff \
beige:f5f5dc bisque:ffe4c4 black:000000 blanchedalmond:ffebcd blue:0000ff \
blueviolet:8a2be2 brown:a52a2a burlywood:deb887 cadetblue:5f9ea0 chartreuse:7fff00 \
chocolate:d2691e coral:ff7f50 cornflowerblue:6495ed cornsilk:fff8dc crimson:dc143c \
cyan:00ffff darkblue:00008b darkcyan:008b8b darkgoldenrod:b8860b darkgray:a9a9a9 \
darkgreen:006400 darkgrey:a9a9a9 darkkhaki:bdb76b darkmagenta:8b008b \
darkolivegreen:556b2f darkorange:ff8c00 darkorchid:9932cc darkred:8b0000 \
darksalmon:e9967a darkseagreen:8fbc8f darkslateblue:483d8b darkslategray:2f4f4f \
darkslategrey:2f4f4f darkturquoise:00ced1 darkviolet:9400d3 deeppink:ff1493 \
deepskyblue:00bfff dimgray:696969 dimgrey:696969 dodgerblue:1e90ff firebrick:b22222 \
floralwhite:fffaf0 forestgreen:228b22 fuchsia:ff00ff gainsboro:dcdcdc ghostwhite:f8f8ff \
gold:ffd700 goldenrod:daa520 gray:808080 green:008000 greenyellow:adff2f grey:808080 \
honeydew:f0fff0 hotpink:ff69b4 indianred:cd5c5c indigo:4b0082 ivory:fffff0 khaki:f0e68c \
lavender:e6e6fa lavenderblush:fff0f5 lawngreen:7cfc00 lemonchiffon:fffacd \
lightblue:add8e6 lightcoral:f08080 lightcyan:e0ffff lightgoldenrodyellow:fafad2 \
lightgray:d3d3d3 lightgreen:90ee90 lightgrey:d3d3d3 lightpink:ffb6c1 lightsalmon:ffa07a \
lightseagreen:20b2aa lightskyblue:87cefa lightslategray:778899 lightslategrey:778899 \
lightsteelblue:b0c4de lightyellow:ffffe0 lime:00ff00 limegreen:32cd32 linen:faf0e6 \
magenta:ff00ff maroon:800000 mediumaquamarine:66cdaa mediumblue:0000cd \
mediumorchid:ba55d3 mediumpurple:9370db mediumseagreen:3cb371 mediumslateblue:7b68ee \
mediumspringgreen:00fa9a mediumturquoise:48d1cc mediumvioletred:c71585 \
midnightblue:191970 mintcream:f5fffa mistyrose:ffe4e1 moccasin:ffe4b5 navajowhite:ffdead \
navy:000080 oldlace:fdf5e6 olive:808000 olivedrab:6b8e23 orange:ffa500 orangered:ff4500 \
orchid:da70d6 palegoldenrod:eee8aa palegreen:98fb98 paleturquoise:afeeee \
palevioletred:db7093 papayawhip:ffefd5 peachpuff:ffdab9 peru:cd853f pink:ffc0cb \
plum:dda0dd powderblue:b0e0e6 purple:800080 rebeccapurple:663399 red:ff0000 \
rosybrown:bc8f8f royalblue:4169e1 saddlebrown:8b4513 salmon:fa8072 sandybrown:f4a460 \
seagreen:2e8b57 seashell:fff5ee sienna:a0522d silver:c0c0c0 skyblue:87ceeb \
slateblue:6a5acd slategray:708090 slategrey:708090 snow:fffafa springgreen:00ff7f \
steelblue:4682b4 tan:d2b48c teal:008080 thistle:d8bfd8 tomato:ff6347 turquoise:40e0d0 \
violet:ee82ee wheat:f5deb3 white:ffffff whitesmoke:f5f5f5 yellow:ffff00 \
yellowgreen:9acd32";

/// Parse one CSS color value (surrounding whitespace allowed).
pub(super) fn parse_css_color(text: &str) -> Option<[u8; 3]> {
    let text = text.trim().to_ascii_lowercase();
    if let Some(hex) = text.strip_prefix('#') {
        return parse_hex(hex);
    }
    if let Some((name, args)) = text.strip_suffix(')').and_then(|t| t.split_once('(')) {
        let channels = channels(args)?;
        return match name {
            "rgb" | "rgba" => rgb(channels),
            "hsl" | "hsla" => hsl(channels),
            _ => None,
        };
    }
    let entry = NAMED
        .split_whitespace()
        .find(|e| e.split_once(':').is_some_and(|(n, _)| n == text))?;
    parse_hex(entry.split_once(':')?.1)
}

/// `rgb` or `rrggbb` hex digits.
fn parse_hex(hex: &str) -> Option<[u8; 3]> {
    if !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let digit = |i: usize| u8::from_str_radix(&hex[i..=i], 16).ok();
    let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
    match hex.len() {
        3 => Some([digit(0)? * 17, digit(1)? * 17, digit(2)? * 17]),
        6 => Some([byte(0)?, byte(2)?, byte(4)?]),
        _ => None,
    }
}

/// The three channel tokens of a color function's arguments: `a, b, c[, α]`
/// or `a b c[ / α]`. A present alpha must be a number or percentage.
fn channels(args: &str) -> Option<[&str; 3]> {
    let (main, alpha): (Vec<&str>, Option<&str>) = if args.contains(',') {
        let mut parts: Vec<&str> = args.split(',').map(str::trim).collect();
        let alpha = (parts.len() == 4).then(|| parts.pop()).flatten();
        (parts, alpha)
    } else {
        let (main, alpha) = match args.split_once('/') {
            Some((main, alpha)) => (main, Some(alpha.trim())),
            None => (args, None),
        };
        (main.split_whitespace().collect(), alpha)
    };
    if let Some(alpha) = alpha {
        number_or_percent(alpha)?;
    }
    main.try_into().ok()
}

/// A finite number.
fn number(token: &str) -> Option<f64> {
    token.parse::<f64>().ok().filter(|n| n.is_finite())
}

/// `n` or `n%`, as `(value, is_percent)`.
fn number_or_percent(token: &str) -> Option<(f64, bool)> {
    match token.strip_suffix('%') {
        Some(n) => Some((number(n)?, true)),
        None => Some((number(token)?, false)),
    }
}

/// `rgb()`: each channel 0..=255 or 0%..=100%, clamped and rounded.
fn rgb(channels: [&str; 3]) -> Option<[u8; 3]> {
    let mut out = [0; 3];
    for (slot, token) in out.iter_mut().zip(channels) {
        let (n, percent) = number_or_percent(token)?;
        let value = if percent { n * 255.0 / 100.0 } else { n };
        *slot = value.clamp(0.0, 255.0).round() as u8;
    }
    Some(out)
}

/// `hsl()`: hue in degrees (or `deg`/`grad`/`rad`/`turn`), saturation and
/// lightness as percentages (CSS Color 4 §7.1 `hslToRgb`).
fn hsl([h, s, l]: [&str; 3]) -> Option<[u8; 3]> {
    let units = [
        ("deg", 1.0),
        ("grad", 0.9),
        ("rad", 180.0 / PI),
        ("turn", 360.0),
    ];
    let hue = match units
        .iter()
        .find_map(|(u, k)| Some((h.strip_suffix(u)?, k)))
    {
        Some((n, k)) => number(n)? * k,
        None => number(h)?,
    }
    .rem_euclid(360.0);
    let fraction = |t: &str| Some((number_or_percent(t)?.0 / 100.0).clamp(0.0, 1.0));
    let (s, l) = (fraction(s)?, fraction(l)?);
    let a = s * l.min(1.0 - l);
    let channel = |n: f64| {
        let k = (n + hue / 30.0) % 12.0;
        let c = l - a * (k - 3.0).min(9.0 - k).clamp(-1.0, 1.0);
        (c * 255.0).round() as u8
    };
    Some([channel(0.0), channel(8.0), channel(4.0)])
}
