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

    let mut input = String::new();
    for (i, e) in entries.iter().enumerate() {
        input.push_str(&format!("{}\t{}\n", i, e.display));
    }

    let mut child = Command::new("fzf")
        .args([
            &format!("--height={}", height),
            "--reverse",
            "--no-info",
            "--ansi",
            "--prompt=▶ ",
            "--with-nth=2..",
            "--nth=2..",
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
    let line = line.trim();
    if line.is_empty() {
        return Ok(None);
    }

    let idx: usize = line
        .split('\t')
        .next()
        .and_then(|s| s.parse().ok())
        .context("parse fzf selection index")?;

    let mut actions: Vec<Option<Action>> =
        entries.into_iter().map(|e| Some(e.action)).collect();
    Ok(actions.get_mut(idx).and_then(|a| a.take()))
}
