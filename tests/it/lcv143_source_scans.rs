//! LCV-143 AC 1 and AC 7 — source scans.
//!
//! Every haystack is a `src/` file cut at its first column-0 `#[cfg(test)]`,
//! so a needle quoted by a test module cannot satisfy or defeat the scan, and
//! every matcher is first run against a positive control built to trip it.

use std::path::Path;

fn implementation(relative: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
    let src = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let end = src.find("\n#[cfg(test)]").unwrap_or(src.len());
    src[..end].to_owned()
}

/// Logging and printing macros that could carry prompt or key text out.
const LOG_NEEDLES: &[&str] = &[
    "tracing",
    "log::",
    "print!(",
    "println!(",
    "eprint!(",
    "eprintln!(",
    "dbg!(",
    "trace!(",
    "debug!(",
    "info!(",
    "warn!(",
    "error!(",
];

/// The first non-comment line of `src` naming any of `needles`.
fn first_hit<'a>(src: &'a str, needles: &[&str]) -> Option<&'a str> {
    src.lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .find(|l| needles.iter().any(|n| l.contains(n)))
}

/// AC 1 — neither settings file names the agent module: the persisted field
/// is plain text with no knowledge of the default.
#[test]
fn ac1_settings_files_do_not_name_the_agent() {
    let needle = ["crate::", "agent"].concat();
    assert!(
        first_hit("use crate::agent::prompt;", &[&needle]).is_some(),
        "positive control: the needle matches a real import"
    );
    for file in ["src/io/settings.rs", "src/io/settings_store.rs"] {
        let src = implementation(file);
        assert!(src.contains("Settings"), "control: {file} was read and cut");
        assert_eq!(first_hit(&src, &[&needle]), None, "{file}");
    }
    assert!(
        implementation("src/io/settings.rs").contains("pub agent_system_prompt: Option<String>"),
        "the field is a plain Option<String>"
    );
}

/// AC 7 — no agent file, and neither half of the turn, logs or prints
/// anything: no prompt or credential text can reach a log.
#[test]
fn ac7_the_agent_path_never_logs() {
    let control = "        eprintln!(\"{} {}\", config.api_key, config.system_prompt);\n\
                   tracing::info!(prompt = %settings.agent_system_prompt);";
    for line in control.lines() {
        assert!(
            first_hit(line, LOG_NEEDLES).is_some(),
            "positive control: the matcher must fire on `{line}`"
        );
    }

    let agent = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/agent");
    let mut files: Vec<String> = std::fs::read_dir(&agent)
        .expect("src/agent is readable")
        .map(|e| e.expect("a readable entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "rs"))
        .map(|p| format!("src/agent/{}", p.file_name().unwrap().to_string_lossy()))
        .collect();
    assert!(
        files.iter().any(|f| f.ends_with("/prompt.rs")),
        "control: the listing includes prompt.rs, got {files:?}"
    );
    files.push("src/app/agent_turn.rs".into());
    files.push("src/app/agent_worker.rs".into());

    for file in &files {
        let src = implementation(file);
        assert!(!src.is_empty(), "control: {file} was read");
        assert_eq!(first_hit(&src, LOG_NEEDLES), None, "AC 7: {file} logs");
    }
}
