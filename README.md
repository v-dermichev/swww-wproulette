# wproulette

Wallpaper roulette CLI for [swww](https://github.com/LGFae/swww) — browse, star, trash, and rotate wallpapers from your terminal or waybar.

## Features

- **Random wallpaper** from any directory tree, excluding trashed
- **Star/unstar** favorites — starred wallpapers are protected from trashing
- **Starred random** — pick only from your favorites
- **Trash with recovery** — preserves subdirectory structure, adds content hash for dedup, full undo
- **Waybar integration** — JSON status output for star/trash button modules
- **Configurable** — wallpaper directory, transition type/duration, badge glyph

## Install

```sh
cargo install --path .
```

Or build manually:

```sh
cargo build --release
cp target/release/wproulette ~/.local/bin/
```

### Arch/Artix

```sh
# Install dependencies
pacman -S swww

# Build and install
git clone https://github.com/v-dermichev/swww-wproulette
cd swww-wproulette
cargo build --release
sudo cp target/release/wproulette /usr/local/bin/
```

### Dependencies

- [swww](https://github.com/LGFae/swww) — wallpaper daemon (must be running)
- Rust toolchain (for building from source)

## Usage

```sh
# Browse wallpapers
wproulette random              # pick a random wallpaper
wproulette starred             # pick from starred only

# Manage
wproulette star                # toggle star on current wallpaper
wproulette trash               # trash current and pick a new one
wproulette restore             # undo last trash (restores to original path)

# Info
wproulette current             # show current wallpaper path
wproulette list-starred        # list all starred wallpapers
wproulette list-trashed        # show last 5 trashed (with original paths)
wproulette list-trashed -n 20  # show last 20

# Waybar integration
wproulette status star         # JSON: star button state
wproulette status trash        # JSON: trash button state

# Config
wproulette config              # show current config
```

## Waybar Integration

Add these modules to your waybar config:

```jsonc
"custom/wallpaper-random": {
    "format": "󰒟󰋩",
    "tooltip-format": "Random wallpaper",
    "on-click": "wproulette random; pkill -RTMIN+11 waybar"
},
"custom/wallpaper-starred": {
    "format": "󰒟󰓎",
    "tooltip-format": "Random from starred",
    "on-click": "wproulette starred; pkill -RTMIN+11 waybar"
},
"custom/wallpaper-star": {
    "exec": "wproulette status star",
    "return-type": "json",
    "interval": "once",
    "signal": 11,
    "on-click": "wproulette star; pkill -RTMIN+11 waybar"
},
"custom/wallpaper-trash": {
    "exec": "wproulette status trash",
    "return-type": "json",
    "interval": "once",
    "signal": 11,
    "on-click": "wproulette trash; pkill -RTMIN+11 waybar"
}
```

Recommended CSS for `style.css`:

```css
/* Wallpaper roulette controls */
#custom-wallpaper-random,
#custom-wallpaper-starred,
#custom-wallpaper-star,
#custom-wallpaper-trash {
    padding: 0 10px;
    box-shadow: inset 0 -3px transparent;
    transition-property: background, box-shadow;
    transition-duration: 300ms;
}

#custom-wallpaper-random:hover,
#custom-wallpaper-starred:hover,
#custom-wallpaper-star:hover,
#custom-wallpaper-trash:hover {
    background: rgba(0, 0, 0, 0.2);
    box-shadow: inset 0 -3px #ffffff;
}

/* Starred random button — golden tint */
#custom-wallpaper-random,
#custom-wallpaper-starred { letter-spacing: 4px; }
#custom-wallpaper-starred { color: #d4a84b; }

/* Star states */
#custom-wallpaper-star.unstarred { color: rgba(255, 255, 255, 0.3); }
#custom-wallpaper-star.starred { color: #d4a84b; }

/* Trash disabled when starred */
#custom-wallpaper-trash.disabled { color: rgba(255, 255, 255, 0.25); }
```

## Config

Config file at `~/.config/wproulette/config.toml`:

```toml
wallpaper_dir = "~/Pictures/Wallpapers"
transition_type = "fade"
transition_duration = 1.0

[icons]
star_active = "󰓎"
star_inactive = "󰓎"
trash_active = "󰩹"
trash_inactive = "󰩹"
random = "󰒟󰋩"
starred = "󰒟󰓎"
```

All fields are optional — defaults are shown above. Tilde (`~`) is expanded automatically.

Icons are used in waybar JSON status output and can be any string (nerd font glyphs, emoji, text).

## How Trash Works

When you trash a wallpaper:

1. The file is moved to `.trash/` inside your wallpaper directory
2. Subdirectory structure is preserved (e.g. `minimal/abstract/file.png` → `.trash/minimal/abstract/file.<sha256>.png`)
3. A full SHA256 content hash is appended to the filename to handle duplicates
4. The original path is recorded in a tab-delimited manifest for recovery
5. `wproulette restore` moves it back to the exact original location

The operation is crash-safe: the file is moved first, then the manifest is updated. If a crash occurs between steps, the file is in `.trash/` but untracked — recoverable by inspection.

Starred wallpapers cannot be trashed — unstar first. Concurrent access is protected by file locks.

## State

All state is stored alongside the config in `~/.config/wproulette/`:

- `config.toml` — configuration
- `current` — current wallpaper path
- `starred` — starred wallpaper list
- `trash_manifest` — trash recovery manifest
- `<wallpaper_dir>/.trash/` — trashed files with preserved structure
