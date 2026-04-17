use crate::cli::Shell;

const ZSH_INIT: &str = include_str!("../shell/init.zsh");
const BASH_INIT: &str = include_str!("../shell/init.bash");

pub fn init_script(shell: Shell) -> &'static str {
    match shell {
        Shell::Zsh => ZSH_INIT,
        Shell::Bash => BASH_INIT,
    }
}
