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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_match() {
        assert!(matches_pattern("github.com", "github.com"));
        assert!(!matches_pattern("github.com", "gitlab.com"));
    }

    #[test]
    fn prefix_glob() {
        assert!(matches_pattern("git.*", "git.example.com"));
        assert!(matches_pattern("git.*", "git."));
        assert!(!matches_pattern("git.*", "github.com"));
    }

    #[test]
    fn suffix_glob() {
        assert!(matches_pattern("*.example.com", "foo.example.com"));
        assert!(!matches_pattern("*.example.com", "foo.other.com"));
    }

    #[test]
    fn middle_glob() {
        assert!(matches_pattern("prod-*-app", "prod-east-app"));
        assert!(!matches_pattern("prod-*-app", "dev-east-app"));
    }

    #[test]
    fn lone_star_matches_anything() {
        assert!(matches_pattern("*", "anything"));
        assert!(matches_pattern("*", ""));
    }

    #[test]
    fn glob_rejects_text_too_short() {
        // prefix+suffix length exceeds text length → no match
        assert!(!matches_pattern("ab*cd", "abc"));
        assert!(!matches_pattern("ab*cd", "cd"));
    }
}
