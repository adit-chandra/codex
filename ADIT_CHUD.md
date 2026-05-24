# Adit CHUD Branch

This checkout keeps the HUD changes as a normal branch on a Codex fork instead
of rebuilding `openai/codex` from a patch file.

## Branch Model

- `main` tracks upstream `openai/codex/main` and should stay clean.
- `adit/chud` contains the HUD changes.
- `origin` points at `openai/codex`.
- `adit` points at `adit-chandra/codex`, which is a true GitHub fork.

Useful sync flow:

```sh
git fetch origin --prune
git switch main
git reset --hard origin/main
git switch adit/chud
git rebase main
```

Push the HUD branch to the fork:

```sh
git remote add adit git@github.com:adit-chandra/codex.git
git push -u adit adit/chud
```

## HUD Config

The branch adds the `custom-line` status item, which reads:

```json
{
  "display": {
    "customLine": "your text here"
  }
}
```

from `~/.codex-hud/config.json`.

For the Claude-style HUD layout, configure `/statusline` or `tui.status_line`
with a model item plus at least one meter item, for example:

```toml
tui_status_line = [
  "model-with-reasoning",
  "project-name",
  "git-branch",
  "used-tokens",
  "context-used",
  "five-hour-limit",
  "weekly-limit",
  "permissions",
  "approval-mode",
  "custom-line",
  "task-progress",
]
```

## Local Build

From the repository root:

```sh
just fmt
just fix -p codex-tui
just test -p codex-tui
cargo +1.93.0 build --manifest-path codex-rs/Cargo.toml --bin codex --release
```

The local binary lands at `codex-rs/target/release/codex`.
