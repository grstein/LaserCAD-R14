//! LCV-130 AC 4b — no loopback `http`/`https` URL literal survives in the tree.
//!
//! **This is a claim about loopback URL literals, not about every sendable
//! endpoint.** A planted `https://api.openai.com/v1` is real, sendable, and
//! invisible here — it parses, but its host is neither loopback nor
//! unspecified. AC 4a's per-file `Url::parse(THE_CONSTANT).is_err()` guard is
//! what catches that shape, wherever a constant carries it; this scan only
//! ever answers the narrower question its name promises.
//!
//! Unlike the bounded scans in `tests/it/repo/tree_scans.rs` and
//! `tests/it/repo/bridge_scans.rs`, this one does **not** stop at the bare
//! `#[cfg(test)]` marker: the original defect (LCV-124 §Problem) lived in test
//! code, in a constant a test itself configured, so a scan that only read
//! production code would have missed the very thing that caused a credential
//! leak. It walks `src/` and `tests/` in full — which means it scans its own
//! source text too, and the four rules in `tests/harness/scan.rs`'s module
//! header bind harder here than anywhere else in the tree.
//!
//! The walker is `tests/harness/scan.rs::rs_files` (LCV-132); the per-line
//! literal extraction and the loopback/unspecified judgment are this
//! demand's own decision and stay here, same as `implementation_sections` in
//! `tests/it/repo/tree_scans.rs`.
//!
//! ### The extractor, in one paragraph
//!
//! For each line that is not a whole-line `//` comment, split on `"` and hand
//! every run between quote characters to `reqwest::Url::parse`, judging only
//! the ones that parse. A raw string needs no special handling: the pieces a
//! naive split yields out of `r#"<svg xmlns="http://www.w3.org/2000/svg"…"#`
//! are `<svg xmlns=` (does not parse) and `http://www.w3.org/2000/svg` (parses,
//! not loopback) — the delimiter characters that make it a raw string never
//! appear between two `"` characters, so they cost this extractor nothing.
//! A literal is flagged only if it parses as an absolute `http`/`https` URL
//! *and* its host is `127.0.0.0/8`, `::1`, `0.0.0.0`, or the domain
//! `localhost`.
//!
//! ### On splitting a "bad" needle with `concat!`
//!
//! `reqwest`/`url` normalise a host leniently (WHATWG's IPv4 parser accepts
//! fewer than four dot-separated parts, folding the remainder into the last
//! one): `Url::parse("http://127.0.0.")` succeeds with host `127.0.0.0`, which
//! **is** in `127.0.0.0/8`. So splitting a bad needle on the last dot before
//! its final octet — the way `tests/it/cmdline/agent_routing.rs`'s
//! `concat!("http://127.0.0.", "1:1")` control does — leaves a *first* piece
//! that is itself a complete, flaggable loopback URL. Every bad needle below
//! instead splits **inside the scheme word**, `"ht" + "tp://…"`: `"ht"` fails
//! `Url::parse` outright, and `"tp://…"` parses with scheme `"tp"`, which this
//! extractor's own scheme check excludes regardless of what its host
//! resolves to. That is the only split shape verified safe against this
//! leniency for every host below, `127.0.0.0/8` included.

use crate::harness;

use harness::scan::rs_files;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::path::{Path, PathBuf};

/// Is `candidate` an absolute `http`/`https` URL whose host is loopback or
/// unspecified? `false` for anything that fails `Url::parse` at all — which is
/// most of what a naive quote-split yields — and for anything not `http`/
/// `https`, so a `ws://127.0.0.1` or a bare `file://` stays out of scope.
///
/// `reqwest::Url::host_str()` is used rather than `reqwest::Url::host()`: the
/// `url` crate that defines the latter's `Host` enum is a transitive
/// dependency reached only through `reqwest`'s re-export of `Url` itself, and
/// naming `url::Host` directly here would require declaring `url` as this
/// crate's own dependency for a type this test can avoid needing entirely.
fn is_loopback_or_unspecified_url(candidate: &str) -> bool {
    let Ok(url) = reqwest::Url::parse(candidate) else {
        return false;
    };
    if !matches!(url.scheme(), "http" | "https") {
        return false;
    }
    let Some(host) = url.host_str() else {
        return false;
    };
    if host.eq_ignore_ascii_case("localhost") {
        return true;
    }
    if let Ok(v4) = host.parse::<Ipv4Addr>() {
        return v4.is_loopback() || v4.is_unspecified();
    }
    // `Url::host_str` returns an IPv6 host wrapped in its literal brackets.
    if let Some(bracketed) = host.strip_prefix('[').and_then(|s| s.strip_suffix(']'))
        && let Ok(v6) = bracketed.parse::<Ipv6Addr>()
    {
        return v6.is_loopback() || v6.is_unspecified();
    }
    false
}

/// Every double-quoted run on `line`. Not a lexer: it does not know what a
/// raw-string delimiter or an escaped quote is, and does not need to — see the
/// module header for why a naive split is enough for this judgment.
fn quoted_runs(line: &str) -> impl Iterator<Item = &str> {
    line.split('"')
}

/// Every code line of `src`: whole-line `//` comments are dropped, same rule
/// as `tests/harness/scan.rs::occurrences`, so a rule spelled out in a doc
/// comment cannot flag itself.
fn code_lines(src: &str) -> impl Iterator<Item = &str> {
    src.lines()
        .filter(|line| !line.trim_start().starts_with("//"))
}

/// Every flagged literal in `src`, verbatim, in the order they appear.
fn flagged_literals(src: &str) -> Vec<String> {
    code_lines(src)
        .flat_map(quoted_runs)
        .filter(|candidate| is_loopback_or_unspecified_url(candidate))
        .map(str::to_owned)
        .collect()
}

/// AC 4b, the "plant it, prove red" half — from the same extractor the real
/// scan below calls. Every needle splits inside the scheme word (see the
/// module header): the *whole* needle is a real bad spelling at runtime, but
/// neither of its two source-text pieces is, so this file's own presence in
/// the tree scan's haystack cannot trip it.
#[test]
fn the_extractor_flags_the_known_bad_spellings() {
    for bad in [
        concat!("ht", "tp://127.0.0.1:1"),
        concat!("ht", "tp://localhost:3000"),
        concat!("ht", "tp://127.0.0.2:9"),
    ] {
        assert!(
            is_loopback_or_unspecified_url(bad),
            "must flag {bad} as a loopback/unspecified URL literal"
        );
    }
}

/// AC 4b's false-positive control, through the same extractor: a scan that
/// flagged any of these would be weakened until it flagged nothing, which is
/// how a guard dies (demand §Expected tests). Every one of these is a real,
/// unconcatenated spelling that already lives in the tree today.
#[test]
fn the_extractor_leaves_the_known_good_spellings_alone() {
    for good in [
        "https://openrouter.ai/api/v1",
        "http://www.w3.org/2000/svg",
        "127.0.0.1:0",
        "http://{}",
    ] {
        assert!(
            !is_loopback_or_unspecified_url(good),
            "must not flag {good}"
        );
    }
}

/// `src/` and `tests/`, as (relative path, whole file text) pairs, sorted on
/// the rendered path. Whole file, not the implementation section: see the
/// module header for why this scan reads inline `#[cfg(test)]` blocks too.
fn whole_tree() -> (PathBuf, Vec<(String, String)>) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    rs_files(&root.join("src"), &mut files);
    rs_files(&root.join("tests"), &mut files);
    assert!(
        files.len() > 40,
        "positive control: the walk must see the whole tree, saw {}",
        files.len()
    );

    let mut sections: Vec<(String, String)> = files
        .iter()
        .map(|path| {
            let relative = path
                .strip_prefix(root)
                .expect("every walked file is under the manifest dir")
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            let src = std::fs::read_to_string(path).expect("a readable source file");
            (relative, src)
        })
        .collect();
    sections.sort();
    (root.to_path_buf(), sections)
}

/// AC 4b — the real claim: nothing in `src/` or `tests/` names a sendable
/// loopback endpoint. Runs by default (LCV-147): the one control that used to
/// trip this scan — `tests/it/cmdline/agent_routing.rs`'s
/// `the_test_endpoint_cannot_reach_a_proxy` — now splits inside the scheme
/// word, same as this file's own needles above, so no piece of it parses as a
/// loopback URL.
#[test]
fn no_loopback_http_literal_survives_in_the_tree() {
    let (root, sections) = whole_tree();
    assert!(
        !sections.is_empty(),
        "positive control: the scanned tree must not be empty"
    );

    let mut flagged: Vec<(String, String)> = Vec::new();
    for (path, src) in &sections {
        for literal in flagged_literals(src) {
            flagged.push((path.clone(), literal));
        }
    }
    flagged.sort();
    assert!(
        flagged.is_empty(),
        "AC 4b: loopback/unspecified URL literal(s) found under {}: {flagged:?}",
        root.display()
    );
}
