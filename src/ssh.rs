use crate::config::SshConfig;
use std::fs;

pub fn hosts(cfg: &SshConfig) -> Vec<String> {
    let Some(home) = dirs::home_dir() else {
        return vec![];
    };
    let path = home.join(".ssh").join("config");
    let Ok(text) = fs::read_to_string(&path) else {
        return vec![];
    };

    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        let rest = match line
            .strip_prefix("Host ")
            .or_else(|| line.strip_prefix("host "))
        {
            Some(r) => r,
            None => continue,
        };
        for host in rest.split_whitespace() {
            if host.contains('*') || host.contains('?') {
                continue;
            }
            if !cfg.include.is_empty()
                && !cfg.include.iter().any(|p| matches_pattern(p, host))
            {
                continue;
            }
            if cfg.exclude.iter().any(|p| matches_pattern(p, host)) {
                continue;
            }
            out.push(host.to_string());
        }
    }
    out
}

/// Tiny glob: `*` is the only metachar, one per pattern. Literal match otherwise.
fn matches_pattern(pattern: &str, text: &str) -> bool {
    if !pattern.contains('*') {
        return pattern == text;
    }
    let mut parts = pattern.splitn(2, '*');
    let prefix = parts.next().unwrap_or("");
    let suffix = parts.next().unwrap_or("");
    text.starts_with(prefix)
        && text.ends_with(suffix)
        && text.len() >= prefix.len() + suffix.len()
}
