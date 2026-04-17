# runic

Spotlight-style launcher for two things:

- **Makefile targets** — executed locally via `make <target>`
- **SSH hosts** from `~/.ssh/config` — opens a plain `ssh <host>` interactive session

Press **Esc**, type to filter, hit Enter.

## Install

```sh
cargo install --path .
```

Add to your shell rc:

```zsh
# ~/.zshrc
eval "$(runic init zsh)"
```

```bash
# ~/.bashrc   (Esc is a readline prefix, so use double-Esc)
eval "$(runic init bash)"
```

Requires [`fzf`](https://github.com/junegunn/fzf) on `PATH` (`brew install fzf`).

## Using it

On an empty prompt, press **Esc**. A fuzzy picker opens, showing:

1. Targets from `runic.mk` in the current directory (or any parent)
2. Targets from the project's `Makefile`
3. Targets from `~/.runic.mk`
4. Hosts from `~/.ssh/config`

Pick one, it runs. The command lands in shell history so you can re-run with `↑`.

---

## Where commands live

Three files, merged in this priority (earlier wins on name collision):

### `runic.mk` — personal project commands

Drop alongside your project's `Makefile`. Descriptions come from `## ...` comments above each target.

```make
## Spin up the dev stack
up:
	docker compose up -d

## Tail the API logs
logs:
	docker compose logs -f api

## Deploy to $(ENV)
deploy:
	./scripts/deploy.sh $(ENV)

.PHONY: up logs deploy
```

### Project `Makefile` — coexistence with existing build

If your project already has a `Makefile`, runic reads it too. No duplication needed.

```make
## Build release binary
build:
	cargo build --release

## Run tests
test:
	cargo test
```

If a target name exists in both files, **`runic.mk` wins** — useful for personal overrides without touching the shared Makefile.

### `~/.runic.mk` — commands available everywhere

Shortcuts you want from any directory:

```make
## Open today's journal
journal:
	$(EDITOR) ~/notes/$(shell date +%Y-%m-%d).md

## Sync dotfiles
dotsync:
	cd ~/dotfiles && git pull && ./install.sh
```

---

## Interactive prompts

If a target references a variable that isn't defined anywhere (Makefile, environment, CLI args), runic asks for it on `/dev/tty` before running:

```make
deploy:
	ssh $(HOST) "cd /srv && ./deploy.sh"
```

Pressing Enter on `deploy` → runic asks `HOST: `, you type `prod.example.com`, it runs `make deploy HOST=prod.example.com`.

Skip the prompt by passing directly:

```sh
runic run deploy HOST=prod.example.com
```

Or set a default with `?=`:

```make
HOST ?= staging.example.com
```

---

## Config — `~/.config/runic/config.toml`

Optional. All fields have sensible defaults.

```toml
[ssh]
# Hostnames matching these patterns are hidden from the picker.
# `*` is the only glob metacharacter (one per pattern).
exclude = ["github.com", "git.*", "gitlab.*", "bitbucket.*"]

# If non-empty, ONLY hosts matching a pattern here appear.
include = []

[picker]
height = "50%"

[shell]
# How long the shell waits for a follow-up byte after Esc, in milliseconds.
# 10ms is near-instant and eliminates the ~400ms default Esc delay in zsh.
# Set to 0 to leave the shell's existing timeout untouched.
key_timeout_ms = 10
```

> **Tip:** If you're in tmux, add `set -sg escape-time 0` to `~/.tmux.conf` — tmux
> adds its own 500ms Esc delay on top of the shell's.

---

## Commands

| Command | What it does |
|---|---|
| `runic pick` | Open the picker (normally invoked via the Esc widget). |
| `runic run TARGET [VAR=VAL ...]` | Run a target directly, forwarding args to `make`. |
| `runic list` | List all discovered targets with their source file. |
| `runic edit` | Open the nearest `runic.mk` / `Makefile` / `~/.runic.mk` in `$EDITOR`. |
| `runic init zsh\|bash` | Print shell integration for `eval`. |

---

## Auto-run on `cd`?

Runic doesn't do directory-entry hooks — use [direnv](https://direnv.net/) for that. Runic is purely a launcher.

---

## License

MIT — see [LICENSE](LICENSE).
