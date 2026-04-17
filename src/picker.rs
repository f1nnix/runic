use anyhow::{Context, Result};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

pub enum Action {
    Target { name: String, source: PathBuf },
    Ssh(String),
}

pub struct Entry {
    pub display: String,
    pub action: Action,
}

pub fn pick(entries: Vec<Entry>, height: &str) -> Result<Option<Action>> {
    if entries.is_empty() {
        return Ok(None);
    }

    let input = build_input(&entries);

    let mut child = Command::new("fzf")
        .args([
            &format!("--height={}", height),
            "--reverse",
            "--no-info",
            "--ansi",
            "--prompt=▶ ",
            "--with-nth=2..",
            "--delimiter=\t",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .context("spawn fzf (install via `brew install fzf`)")?;

    child
        .stdin
        .as_mut()
        .expect("fzf stdin was piped")
        .write_all(input.as_bytes())?;

    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Ok(None);
    }

    let line = String::from_utf8_lossy(&output.stdout);
    let Some(idx) = parse_selection(&line) else {
        return Ok(None);
    };

    let mut actions: Vec<Option<Action>> =
        entries.into_iter().map(|e| Some(e.action)).collect();
    Ok(actions.get_mut(idx).and_then(|a| a.take()))
}

/// Build fzf's stdin: one line per entry, `<idx>\t<display>\n`.
/// The idx lets us recover the original Action after fzf returns the chosen line.
fn build_input(entries: &[Entry]) -> String {
    let mut out = String::new();
    for (i, e) in entries.iter().enumerate() {
        out.push_str(&format!("{}\t{}\n", i, e.display));
    }
    out
}

/// Extract the idx from fzf's output — the first tab-delimited field of the first line.
fn parse_selection(output: &str) -> Option<usize> {
    let line = output.lines().next()?.trim();
    if line.is_empty() {
        return None;
    }
    line.split('\t').next()?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(display: &str) -> Entry {
        Entry {
            display: display.to_string(),
            action: Action::Ssh("unused".into()),
        }
    }

    #[test]
    fn build_input_formats_tab_idx_display() {
        let input = build_input(&[entry("first"), entry("second")]);
        assert_eq!(input, "0\tfirst\n1\tsecond\n");
    }

    #[test]
    fn build_input_empty_yields_empty() {
        assert_eq!(build_input(&[]), "");
    }

    #[test]
    fn build_input_preserves_ansi_in_display() {
        let input = build_input(&[entry("\x1b[36mtag\x1b[0m x")]);
        assert_eq!(input, "0\t\x1b[36mtag\x1b[0m x\n");
    }

    #[test]
    fn parse_selection_single_line() {
        assert_eq!(parse_selection("3\t[runic] build"), Some(3));
    }

    #[test]
    fn parse_selection_empty() {
        assert_eq!(parse_selection(""), None);
        assert_eq!(parse_selection("\n"), None);
    }

    #[test]
    fn parse_selection_trailing_newline() {
        assert_eq!(parse_selection("7\thello\n"), Some(7));
    }

    #[test]
    fn parse_selection_rejects_non_numeric() {
        assert_eq!(parse_selection("abc\thello"), None);
    }
}
