use crate::cli::Shell;
use crate::config::ShellConfig;

const ZSH_INIT: &str = include_str!("../shell/init.zsh");
const BASH_INIT: &str = include_str!("../shell/init.bash");

pub fn init_script(shell: Shell, cfg: &ShellConfig) -> String {
    let mut out = String::new();
    match shell {
        Shell::Zsh => {
            if cfg.key_timeout_ms > 0 {
                // zsh's KEYTIMEOUT is in units of 10ms; clamp so we never emit 0
                // (which would break arrow keys and other escape sequences).
                let k = std::cmp::max(1, cfg.key_timeout_ms / 10);
                out.push_str(&format!("KEYTIMEOUT={}\n", k));
            }
            out.push_str(ZSH_INIT);
        }
        Shell::Bash => {
            if cfg.key_timeout_ms > 0 {
                out.push_str(&format!(
                    "bind 'set keyseq-timeout {}' 2>/dev/null\n",
                    cfg.key_timeout_ms
                ));
            }
            out.push_str(BASH_INIT);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zsh_emits_keytimeout_by_default() {
        let cfg = ShellConfig::default();
        let out = init_script(Shell::Zsh, &cfg);
        assert!(out.starts_with("KEYTIMEOUT=1\n"));
        assert!(out.contains("bindkey '\\e' _runic_widget"));
    }

    #[test]
    fn zsh_omits_keytimeout_when_zero() {
        let cfg = ShellConfig { key_timeout_ms: 0 };
        let out = init_script(Shell::Zsh, &cfg);
        assert!(!out.contains("KEYTIMEOUT"));
    }

    #[test]
    fn zsh_scales_ms_to_zsh_units() {
        // 100ms → KEYTIMEOUT=10 (10 × 10ms)
        let cfg = ShellConfig {
            key_timeout_ms: 100,
        };
        let out = init_script(Shell::Zsh, &cfg);
        assert!(out.starts_with("KEYTIMEOUT=10\n"));
    }

    #[test]
    fn zsh_never_emits_zero_for_small_values() {
        // 5ms would round to KEYTIMEOUT=0 which breaks escape sequences; clamp to 1.
        let cfg = ShellConfig { key_timeout_ms: 5 };
        let out = init_script(Shell::Zsh, &cfg);
        assert!(out.starts_with("KEYTIMEOUT=1\n"));
    }

    #[test]
    fn bash_emits_keyseq_timeout_by_default() {
        let cfg = ShellConfig::default();
        let out = init_script(Shell::Bash, &cfg);
        assert!(out.starts_with("bind 'set keyseq-timeout 10'"));
        assert!(out.contains("_runic_pick_widget"));
    }

    #[test]
    fn bash_omits_keyseq_timeout_when_zero() {
        let cfg = ShellConfig { key_timeout_ms: 0 };
        let out = init_script(Shell::Bash, &cfg);
        assert!(!out.contains("keyseq-timeout"));
    }
}
