use anyhow::{Context, Result};
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};

/// Prompt on /dev/tty (bypassing stdin/stdout which may be captured
/// by the shell's `$(...)` substitution) and return the user's reply.
pub fn ask(label: &str) -> Result<String> {
    let tty_in = OpenOptions::new()
        .read(true)
        .open("/dev/tty")
        .context("open /dev/tty for reading")?;
    let mut tty_out = OpenOptions::new()
        .write(true)
        .open("/dev/tty")
        .context("open /dev/tty for writing")?;
    write!(tty_out, "{}: ", label)?;
    tty_out.flush()?;

    let mut reader = BufReader::new(tty_in);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    Ok(line.trim_end_matches(&['\r', '\n'][..]).to_string())
}
