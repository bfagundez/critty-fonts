# critty-fonts

A fuzzy-search font picker for Alacritty, inspired by kitty's `kitten choose-fonts`.

https://github.com/user-attachments/assets/3a69831e-722b-4fb7-a95a-2d085cfd7861

Browse installed monospace fonts and preview them live in your actual Alacritty
window as you move through the list, no restart needed.

## How it works

Alacritty supports `live_config_reload`, so this tool rewrites the `family`
fields in your Alacritty config as you navigate, and Alacritty re-renders
itself. Nothing else in your config is touched. Cancel and the original file
is restored exactly; commit and only the font fields change.

## Requirements

- Rust and Cargo
- Alacritty
- `fc-list` (fontconfig), used to enumerate installed fonts
- `live_config_reload = true` set in your Alacritty config for live preview
  (the tool still works without it, just without live preview)

## Usage

```
cargo run --release
```

- Type to fuzzy filter fonts
- Up/Down to move selection and preview live
- Enter to commit the selected font
- Esc or Ctrl-C to cancel and restore your original config

A timestamped backup (`alacritty.toml.bak-<timestamp>`) is written next to
your config on each run as a safety net.

## Development

Commit messages follow Conventional Commits and are enforced by commitlint
via a git hook. After cloning, enable it with:

```
git config core.hooksPath .githooks
```
