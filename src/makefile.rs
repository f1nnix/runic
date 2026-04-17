use anyhow::{Context, Result};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Target {
    pub name: String,
    pub description: Option<String>,
    pub body_vars: BTreeSet<String>,
}

#[derive(Debug)]
pub struct Makefile {
    pub path: PathBuf,
    pub targets: Vec<Target>,
    pub defined_vars: BTreeSet<String>,
}

pub fn parse(path: &Path) -> Result<Makefile> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("read {}", path.display()))?;
    Ok(parse_str(&text, path.to_path_buf()))
}

fn parse_str(text: &str, path: PathBuf) -> Makefile {
    let mut targets: Vec<Target> = Vec::new();
    let mut defined_vars: BTreeSet<String> = BTreeSet::new();
    let mut pending_desc: Option<String> = None;
    let mut current: Option<Target> = None;
    let mut current_body = String::new();

    let flush = |targets: &mut Vec<Target>, cur: &mut Option<Target>, body: &mut String| {
        if let Some(mut t) = cur.take() {
            t.body_vars = extract_var_refs(body);
            targets.push(t);
            body.clear();
        }
    };

    for raw in text.lines() {
        if raw.starts_with('\t') {
            // Recipe line for the current target.
            if current.is_some() {
                current_body.push_str(raw);
                current_body.push('\n');
            }
            continue;
        }

        let trimmed = raw.trim_start();

        if trimmed.is_empty() {
            pending_desc = None;
            continue;
        }

        // Description convention: `## text` on its own line = description for next target.
        if let Some(rest) = trimmed.strip_prefix("##") {
            let d = rest.trim();
            if !d.is_empty() {
                pending_desc = Some(d.to_string());
            }
            continue;
        }

        // Regular comment — ignore.
        if trimmed.starts_with('#') {
            continue;
        }

        if let Some(var_name) = parse_var_def(trimmed) {
            defined_vars.insert(var_name);
            continue;
        }

        if let Some((name, on_line_desc, prereqs)) = parse_target(trimmed) {
            flush(&mut targets, &mut current, &mut current_body);
            let desc = on_line_desc.or_else(|| pending_desc.take());
            let t = Target {
                name,
                description: desc,
                body_vars: BTreeSet::new(),
            };
            // Vars referenced in prereqs count too.
            current_body.push_str(&prereqs);
            current_body.push('\n');
            current = Some(t);
            pending_desc = None;
            continue;
        }

        pending_desc = None;
    }

    flush(&mut targets, &mut current, &mut current_body);

    Makefile {
        path,
        targets,
        defined_vars,
    }
}

/// Parse a target line `name[ name...] : [prereqs] [## desc]`.
/// Returns (first_target_name, inline_description, prereqs_text).
fn parse_target(line: &str) -> Option<(String, Option<String>, String)> {
    // Skip variable-assignment false positives (handled elsewhere) by requiring `:` not followed by `=`.
    let colon = find_target_colon(line)?;
    let name_part = line[..colon].trim();
    let first = name_part.split_whitespace().next()?.to_string();
    if first.is_empty() {
        return None;
    }
    // Valid make target names: letters, digits, _, -, .
    if !first
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
    {
        return None;
    }
    // Skip special targets like .PHONY, .SUFFIXES, .DEFAULT, etc.
    if first.starts_with('.') {
        return None;
    }

    let rest = &line[colon + 1..];
    let (prereqs, description) = match rest.find("##") {
        Some(i) => (rest[..i].trim().to_string(), {
            let d = rest[i + 2..].trim();
            if d.is_empty() {
                None
            } else {
                Some(d.to_string())
            }
        }),
        None => (rest.trim().to_string(), None),
    };

    Some((first, description, prereqs))
}

/// Find a colon that marks a target, not a `:=` assignment operator or a double-colon rule.
fn find_target_colon(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b == b':' {
            // Skip `:=` (assignment)
            if bytes.get(i + 1) == Some(&b'=') {
                return None;
            }
            return Some(i);
        }
    }
    None
}

/// Recognize `NAME = ...`, `NAME := ...`, `NAME ?= ...`, `NAME += ...`,
/// optionally preceded by `export` or `override`.
fn parse_var_def(line: &str) -> Option<String> {
    let line = line
        .strip_prefix("export ")
        .or_else(|| line.strip_prefix("override "))
        .unwrap_or(line);

    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len()
        && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b'.')
    {
        i += 1;
    }
    if i == 0 {
        return None;
    }
    let name = line[..i].to_string();
    let rest = line[i..].trim_start();
    if rest.starts_with("=")
        || rest.starts_with(":=")
        || rest.starts_with("?=")
        || rest.starts_with("+=")
        || rest.starts_with("::=")
    {
        Some(name)
    } else {
        None
    }
}

fn extract_var_refs(text: &str) -> BTreeSet<String> {
    let mut refs = BTreeSet::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'$' && i + 1 < bytes.len() {
            let next = bytes[i + 1];
            // $$ → literal $
            if next == b'$' {
                i += 2;
                continue;
            }
            if next == b'(' || next == b'{' {
                let close = if next == b'(' { b')' } else { b'}' };
                let start = i + 2;
                if let Some(offset) = find_matching(&bytes[start..], close) {
                    let inner = std::str::from_utf8(&bytes[start..start + offset]).unwrap_or("");
                    let inner = inner.trim();
                    // Skip make functions: `$(shell ...)`, `$(wildcard ...)`, etc. —
                    // detected by whitespace inside.
                    if !inner.is_empty()
                        && !inner.contains(char::is_whitespace)
                        && is_identifier(inner)
                        && !is_builtin(inner)
                    {
                        refs.insert(inner.to_string());
                    }
                    i = start + offset + 1;
                    continue;
                }
            }
            // Single-char automatic vars ($@, $<, $^, $*, $?, $+, $|) — skip.
            i += 2;
            continue;
        }
        i += 1;
    }
    refs
}

/// Find the matching close byte, handling nested `()` / `{}`.
fn find_matching(bytes: &[u8], close: u8) -> Option<usize> {
    let open = if close == b')' { b'(' } else { b'{' };
    let mut depth = 1usize;
    for (i, &b) in bytes.iter().enumerate() {
        if b == open {
            depth += 1;
        } else if b == close {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
    }
    None
}

fn is_identifier(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
        && !s.starts_with(|c: char| c.is_ascii_digit())
}

fn is_builtin(name: &str) -> bool {
    matches!(
        name,
        "MAKE"
            | "MAKEFLAGS"
            | "MAKEFILE_LIST"
            | "MAKECMDGOALS"
            | "MAKELEVEL"
            | "MAKEVARS"
            | "MAKEFILES"
            | "MAKESHELL"
            | "SHELL"
            | "CURDIR"
            | "SUFFIXES"
            | ".VARIABLES"
            | ".FEATURES"
            | ".DEFAULT_GOAL"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_mk(text: &str) -> Makefile {
        parse_str(text, PathBuf::from("test.mk"))
    }

    // -- parse_target --------------------------------------------------------

    #[test]
    fn target_simple() {
        let (name, desc, prereqs) = parse_target("build:").unwrap();
        assert_eq!(name, "build");
        assert!(desc.is_none());
        assert_eq!(prereqs, "");
    }

    #[test]
    fn target_with_prereqs() {
        let (name, desc, prereqs) = parse_target("install: build test").unwrap();
        assert_eq!(name, "install");
        assert!(desc.is_none());
        assert_eq!(prereqs, "build test");
    }

    #[test]
    fn target_with_trailing_description() {
        let (name, desc, _) = parse_target("build: ## Build the project").unwrap();
        assert_eq!(name, "build");
        assert_eq!(desc.as_deref(), Some("Build the project"));
    }

    #[test]
    fn target_with_prereqs_and_description() {
        let (name, desc, prereqs) = parse_target("deploy: build ## Deploy").unwrap();
        assert_eq!(name, "deploy");
        assert_eq!(desc.as_deref(), Some("Deploy"));
        assert_eq!(prereqs, "build");
    }

    #[test]
    fn target_skips_special() {
        assert!(parse_target(".PHONY: build").is_none());
        assert!(parse_target(".SUFFIXES:").is_none());
    }

    #[test]
    fn target_skips_assignment() {
        assert!(parse_target("VAR := value").is_none());
        assert!(parse_target("VAR ?= default").is_none());
    }

    // -- parse_var_def -------------------------------------------------------

    #[test]
    fn var_def_all_operators() {
        assert_eq!(parse_var_def("HOST = x").as_deref(), Some("HOST"));
        assert_eq!(parse_var_def("HOST := x").as_deref(), Some("HOST"));
        assert_eq!(parse_var_def("HOST ?= x").as_deref(), Some("HOST"));
        assert_eq!(parse_var_def("CFLAGS += -O2").as_deref(), Some("CFLAGS"));
    }

    #[test]
    fn var_def_with_export() {
        assert_eq!(parse_var_def("export PATH = /usr/bin").as_deref(), Some("PATH"));
    }

    #[test]
    fn var_def_with_override() {
        assert_eq!(parse_var_def("override CC = clang").as_deref(), Some("CC"));
    }

    #[test]
    fn var_def_rejects_target() {
        assert!(parse_var_def("build: test").is_none());
    }

    // -- extract_var_refs ----------------------------------------------------

    #[test]
    fn refs_simple_paren() {
        let refs = extract_var_refs("ssh $(HOST)");
        assert!(refs.contains("HOST"));
        assert_eq!(refs.len(), 1);
    }

    #[test]
    fn refs_simple_curly() {
        let refs = extract_var_refs("ssh ${HOST}");
        assert!(refs.contains("HOST"));
    }

    #[test]
    fn refs_multiple() {
        let refs = extract_var_refs("ssh $(USER)@$(HOST):$(PORT)");
        assert!(refs.contains("USER"));
        assert!(refs.contains("HOST"));
        assert!(refs.contains("PORT"));
        assert_eq!(refs.len(), 3);
    }

    #[test]
    fn refs_skip_builtins() {
        let refs = extract_var_refs("$(MAKE) -C sub; $(SHELL) -c foo");
        assert!(!refs.contains("MAKE"));
        assert!(!refs.contains("SHELL"));
    }

    #[test]
    fn refs_skip_shell_functions() {
        // Any whitespace inside $(...) means it's a function call, not a var.
        let refs = extract_var_refs("echo $(shell date +%Y)");
        assert!(refs.is_empty());
    }

    #[test]
    fn refs_skip_automatic_vars() {
        // $@, $<, $^, $*, $? — two-char refs, not variables to prompt for.
        let refs = extract_var_refs("cp $< $@; tar $^");
        assert!(refs.is_empty());
    }

    #[test]
    fn refs_skip_dollar_escape() {
        let refs = extract_var_refs("echo $$HOME");
        assert!(refs.is_empty());
    }

    // -- find_matching (nested parens) ---------------------------------------

    #[test]
    fn matching_handles_flat() {
        // find_matching assumes depth 1 on entry (caller already consumed the open).
        assert_eq!(find_matching(b"foo)", b')'), Some(3));
    }

    #[test]
    fn matching_skips_nested_pairs() {
        // Inner `()` pair at idx 0..2 bumps depth up and back; outer close at idx 2.
        assert_eq!(find_matching(b"())", b')'), Some(2));
        // One nested pair plus trailing text: outer close at idx 7.
        assert_eq!(find_matching(b"(a()b)c)", b')'), Some(7));
    }

    // -- full parse ----------------------------------------------------------

    #[test]
    fn parse_collects_targets_and_descriptions() {
        let mk = parse_mk(
            "## Build the project\n\
             build:\n\
             \tcargo build\n\
             \n\
             ## Run tests\n\
             test: build\n\
             \tcargo test\n",
        );
        assert_eq!(mk.targets.len(), 2);
        assert_eq!(mk.targets[0].name, "build");
        assert_eq!(mk.targets[0].description.as_deref(), Some("Build the project"));
        assert_eq!(mk.targets[1].name, "test");
        assert_eq!(mk.targets[1].description.as_deref(), Some("Run tests"));
    }

    #[test]
    fn parse_trailing_description_also_works() {
        let mk = parse_mk("build: ## Build it\n\tcargo build\n");
        assert_eq!(mk.targets[0].description.as_deref(), Some("Build it"));
    }

    #[test]
    fn parse_blank_line_discards_pending_description() {
        let mk = parse_mk(
            "## Orphan description\n\
             \n\
             build:\n\
             \tcargo build\n",
        );
        assert!(mk.targets[0].description.is_none());
    }

    #[test]
    fn parse_collects_defined_vars() {
        let mk = parse_mk(
            "HOST ?= localhost\n\
             PORT := 22\n\
             deploy:\n\
             \tssh $(HOST):$(PORT)\n",
        );
        assert!(mk.defined_vars.contains("HOST"));
        assert!(mk.defined_vars.contains("PORT"));
    }

    #[test]
    fn parse_collects_body_vars_including_prereqs() {
        let mk = parse_mk(
            "deploy: $(ARTIFACT)\n\
             \tscp $(ARTIFACT) $(HOST):/srv\n",
        );
        let target = &mk.targets[0];
        assert!(target.body_vars.contains("ARTIFACT"));
        assert!(target.body_vars.contains("HOST"));
    }

    #[test]
    fn parse_ignores_phony() {
        let mk = parse_mk(
            "build:\n\
             \tcargo build\n\
             .PHONY: build\n",
        );
        assert_eq!(mk.targets.len(), 1);
        assert_eq!(mk.targets[0].name, "build");
    }

    #[test]
    fn parse_handles_var_def_with_colon_equals() {
        // The `:=` should NOT be misread as a target colon.
        let mk = parse_mk("CC := clang\nbuild:\n\t$(CC) -o x\n");
        assert_eq!(mk.targets.len(), 1);
        assert_eq!(mk.targets[0].name, "build");
        assert!(mk.defined_vars.contains("CC"));
    }
}
