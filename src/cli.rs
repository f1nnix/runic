use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(
    name = "runic",
    version,
    about = "Spotlight-like launcher for Makefile targets and SSH hosts"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Print shell integration (add `eval "$(runic init zsh)"` to your rc file).
    Init { shell: Shell },
    /// Open the interactive picker; prints the chosen command to stdout for shell capture.
    Pick,
    /// Run a named target by invoking `make`. Extra args are forwarded (e.g. `runic run deploy ENV=prod`).
    Run {
        name: String,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Open the nearest runic.mk (or project Makefile, or ~/.runic.mk) in $EDITOR.
    Edit,
    /// List all known targets from all source files.
    List,
}

#[derive(ValueEnum, Clone, Copy)]
pub enum Shell {
    Zsh,
    Bash,
}
