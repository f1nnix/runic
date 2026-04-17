use anyhow::{bail, Context, Result};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::cli::Command as CliCommand;
use crate::{config, makefile, picker, prompt, shell, ssh};

pub fn dispatch(cmd: CliCommand) -> Result<()> {
    match cmd {
        CliCommand::Init { shell } => {
            print!("{}", shell::init_script(shell));
            Ok(())
        }
        CliCommand::Pick => pick(),
        CliCommand::Run { name, args } => run(&name, args),
        CliCommand::Edit => edit(),
        CliCommand::List => list(),
    }
}

#[derive(Clone, Copy, Debug)]
enum SourceKind {
    RunicLocal,
    Makefile,
    RunicGlobal,
}

impl SourceKind {
    fn tag(self) -> &'static str {
        match self {
            SourceKind::RunicLocal => "runic",
            SourceKind::Makefile => "make",
            SourceKind::RunicGlobal => "global",
        }
    }
}

struct Source {
    kind: SourceKind,
    mk: makefile::Makefile,
}

fn cwd() -> Result<PathBuf> {
    std::env::current_dir().context("get current directory")
}

/// Sources in picker priority order (earlier wins on target-name collision).
fn sources() -> Result<Vec<Source>> {
    let mut out = Vec::new();
    let cwd = cwd()?;

    if let Some(p) = find_up(&cwd, "runic.mk") {
        out.push(Source {
            kind: SourceKind::RunicLocal,
            mk: makefile::parse(&p)?,
        });
    }

    let mk = cwd.join("Makefile");
    if mk.is_file() {
        out.push(Source {
            kind: SourceKind::Makefile,
            mk: makefile::parse(&mk)?,
        });
    }

    if let Some(home) = dirs::home_dir() {
        let g = home.join(".runic.mk");
        if g.is_file() {
            out.push(Source {
                kind: SourceKind::RunicGlobal,
                mk: makefile::parse(&g)?,
            });
        }
    }
    Ok(out)
}

fn find_up(start: &Path, name: &str) -> Option<PathBuf> {
    let mut cur = start.to_path_buf();
    loop {
        let p = cur.join(name);
        if p.is_file() {
            return Some(p);
        }
        if !cur.pop() {
            return None;
        }
    }
}

fn pick() -> Result<()> {
    let cfg = config::load()?;
    let sources = sources()?;

    let mut entries: Vec<picker::Entry> = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for src in &sources {
        for t in &src.mk.targets {
            if !seen.insert(t.name.clone()) {
                continue;
            }
            entries.push(picker::Entry {
                display: format_target(src.kind, &t.name, t.description.as_deref()),
                action: picker::Action::Target {
                    name: t.name.clone(),
                    source: src.mk.path.clone(),
                },
            });
        }
    }

    for host in ssh::hosts(&cfg.ssh) {
        entries.push(picker::Entry {
            display: format_ssh(&host),
            action: picker::Action::Ssh(host),
        });
    }

    match picker::pick(entries, &cfg.picker.height)? {
        Some(picker::Action::Target { name, source }) => {
            let mk = makefile::parse(&source)?;
            let target = mk
                .targets
                .iter()
                .find(|t| t.name == name)
                .expect("selected target must exist in its source");
            let assignments = collect_assignments(target, &mk, &[])?;
            print_make_invocation(&source, &name, &[], &assignments);
        }
        Some(picker::Action::Ssh(host)) => {
            println!("ssh {}", host);
        }
        None => {}
    }
    Ok(())
}

fn run(name: &str, args: Vec<String>) -> Result<()> {
    let sources = sources()?;
    for src in &sources {
        if let Some(t) = src.mk.targets.iter().find(|t| t.name == name) {
            let assignments = collect_assignments(t, &src.mk, &args)?;
            let status = Command::new("make")
                .arg("-f")
                .arg(&src.mk.path)
                .arg(name)
                .args(&args)
                .args(&assignments)
                .status()
                .context("spawn make")?;
            std::process::exit(status.code().unwrap_or(1));
        }
    }
    bail!(
        "no target named '{}' in runic.mk, Makefile, or ~/.runic.mk",
        name
    );
}

fn edit() -> Result<()> {
    let cwd = cwd()?;
    let path = if let Some(p) = find_up(&cwd, "runic.mk") {
        p
    } else if cwd.join("Makefile").is_file() {
        cwd.join("Makefile")
    } else if let Some(home) = dirs::home_dir() {
        let g = home.join(".runic.mk");
        if g.is_file() {
            g
        } else {
            cwd.join("runic.mk")
        }
    } else {
        cwd.join("runic.mk")
    };
    let editor = std::env::var("EDITOR")
        .or_else(|_| std::env::var("VISUAL"))
        .unwrap_or_else(|_| "vi".to_string());
    let status = Command::new(&editor).arg(&path).status()?;
    if !status.success() {
        bail!("{} exited with status {}", editor, status);
    }
    Ok(())
}

fn list() -> Result<()> {
    let sources = sources()?;
    if sources.is_empty() {
        eprintln!("runic: no runic.mk, Makefile, or ~/.runic.mk found");
        return Ok(());
    }
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for src in &sources {
        println!("# {} ({})", src.mk.path.display(), src.kind.tag());
        for t in &src.mk.targets {
            let marker = if seen.insert(t.name.clone()) {
                " "
            } else {
                "~"
            };
            let desc = t.description.as_deref().unwrap_or("");
            println!("  {} {:24}  {}", marker, t.name, desc);
        }
    }
    Ok(())
}

fn collect_assignments(
    target: &makefile::Target,
    mk: &makefile::Makefile,
    user_args: &[String],
) -> Result<Vec<String>> {
    let mut out = Vec::new();
    for v in &target.body_vars {
        if mk.defined_vars.contains(v) {
            continue;
        }
        if std::env::var(v).is_ok() {
            continue;
        }
        if user_args
            .iter()
            .any(|a| a.starts_with(&format!("{}=", v)))
        {
            continue;
        }
        let value = prompt::ask(v)?;
        out.push(format!("{}={}", v, value));
    }
    Ok(out)
}

fn print_make_invocation(source: &Path, target: &str, args: &[String], assignments: &[String]) {
    let mut parts: Vec<String> = vec![
        "make".into(),
        "-f".into(),
        shell_quote(&source.to_string_lossy()),
        target.into(),
    ];
    for a in args {
        parts.push(shell_quote(a));
    }
    for a in assignments {
        parts.push(shell_quote(a));
    }
    println!("{}", parts.join(" "));
}

fn format_target(kind: SourceKind, name: &str, desc: Option<&str>) -> String {
    let tag_color = match kind {
        SourceKind::RunicLocal => "\x1b[36m",
        SourceKind::Makefile => "\x1b[35m",
        SourceKind::RunicGlobal => "\x1b[33m",
    };
    let tag = format!("{}{}\x1b[0m", tag_color, kind.tag());
    match desc {
        Some(d) if !d.is_empty() => format!("[{}] {}  \x1b[2m— {}\x1b[0m", tag, name, d),
        _ => format!("[{}] {}", tag, name),
    }
}

fn format_ssh(host: &str) -> String {
    format!("[\x1b[32mssh\x1b[0m] {}", host)
}

fn shell_quote(s: &str) -> String {
    if !s.is_empty()
        && s.chars().all(|c| {
            c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '/' | '=' | ':' | ',')
        })
    {
        return s.to_string();
    }
    format!("'{}'", s.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quote_preserves_safe_chars() {
        assert_eq!(shell_quote("simple"), "simple");
        assert_eq!(shell_quote("KEY=VAL"), "KEY=VAL");
        assert_eq!(shell_quote("path/to/file.rs"), "path/to/file.rs");
        assert_eq!(shell_quote("host-1"), "host-1");
    }

    #[test]
    fn quote_wraps_empty_string() {
        assert_eq!(shell_quote(""), "''");
    }

    #[test]
    fn quote_wraps_spaces() {
        assert_eq!(shell_quote("hello world"), "'hello world'");
    }

    #[test]
    fn quote_escapes_single_quote() {
        assert_eq!(shell_quote("it's"), "'it'\\''s'");
    }

    #[test]
    fn quote_wraps_shell_metacharacters() {
        assert_eq!(shell_quote("$FOO"), "'$FOO'");
        assert_eq!(shell_quote("a;b"), "'a;b'");
        assert_eq!(shell_quote("`echo`"), "'`echo`'");
    }
}
