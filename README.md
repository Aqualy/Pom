# Pomodoro

A minimalist tomato timer for the desktop, after Enzo Mari's flat fruit prints.
Pure Rust + [Slint](https://slint.dev), one small native binary.

## Install

Grab the installer for your platform from the [Releases](https://github.com/Aqualy/Pom/releases) page:
`*-setup.exe` (Windows), `.deb` or `.AppImage` (Linux), `.dmg` (macOS, Apple Silicon).
The builds are unsigned, so Windows SmartScreen and macOS Gatekeeper will warn on first launch
(on macOS, right-click the app and choose **Open**).

## Use

- **Click the tomato** to start or pause. **Double-click** to reset the current session.
- The red drains clockwise as time passes. Orange is a short break, green a long one.
- The dots count focus sessions toward the long break.
- Hover the window for controls: ○ keep on top, × quit, » skip phase, ••• settings.
- Drag anywhere outside the tomato to move the window.
- In settings, type a value or hold − / + to change it quickly. The sound row has a volume slider and a ▶ preview.

Settings live in the platform config dir (`%APPDATA%\Pomodoro\config\config.toml` on Windows,
`~/Library/Application Support/Pomodoro/config.toml` on macOS, `~/.config/pomodoro/config.toml` on Linux).

## Artwork

The tomato comes from `tomato_crown_lowered.svg`, embedded at compile time. It needs two `<path>`s,
body first, then crown (a `translate` on the crown is honored), using absolute `M`/`L`/`C`/`Z` commands.
The draining wedge is traced from the body outline, so swapping the SVG just works.

## Build

```sh
cargo run            # debug
cargo test
cargo build --release
```

Linux needs the usual Slint/ALSA/D-Bus dev packages (`libfontconfig1-dev libxkbcommon-dev libasound2-dev libdbus-1-dev`).

Installers are built with [cargo-packager](https://github.com/crabnebula-dev/cargo-packager)
(`cargo packager --release --out-dir dist`). CI tests on Windows, Linux and macOS for every push,
then builds the installers; pushing a `v*` tag also publishes them as a GitHub release.
The icons in `icons/` are rendered from the SVG by `python scripts/make_icons.py` (needs Pillow).

## Credits

Inter typeface by Rasmus Andersson (SIL OFL, `ui/fonts/LICENSE.txt`). Made with Slint (Royalty-free license).
