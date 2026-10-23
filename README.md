# Pom

A minimalist Pomodoro timer for the desktop.

## Install

Grab the installer for your platform from the [Releases](https://github.com/Aqualy/Pom/releases) page:
`*-setup.exe` (Windows), `.deb` or `.AppImage` (Linux), `.dmg` (macOS, Apple Silicon).
The builds aren't notarized, so Windows SmartScreen and macOS Gatekeeper will warn on first launch.
On macOS, open the app once, then go to **System Settings → Privacy & Security** and click **Open Anyway**.
If macOS says the app is damaged (v0.1.1 and earlier), run `xattr -dr com.apple.quarantine /Applications/Pom.app`.

## Use

- **Click the tomato** to start or pause. **Double-click** to reset the current session.
- The red drains clockwise as time passes. Orange is a short break, green a long one.
- The dots count focus sessions toward the long break.
- Hover the window for controls: ○ keep on top, × quit, » skip phase, ••• settings.
- Drag anywhere outside the tomato to move the window.
- In settings, type a value or hold − / + to change it quickly. The sound row has a volume slider and a ▶ preview.

Settings live in the platform config dir (`%APPDATA%\Pom\config\config.toml` on Windows,
`~/Library/Application Support/Pom/config.toml` on macOS, `~/.config/pom/config.toml` on Linux).

## Build

```sh
cargo run
cargo test
cargo packager --release --out-dir dist   # installers, needs cargo-packager
```

Linux needs `libfontconfig1-dev libxkbcommon-dev libasound2-dev libdbus-1-dev`.
CI tests on Windows, Linux and macOS and builds the installers; pushing a `v*` tag publishes a release.

## License

MIT, see [LICENSE](LICENSE).

Inter typeface by Rasmus Andersson (SIL OFL, `ui/fonts/LICENSE.txt`). Made with [Slint](https://slint.dev) (Royalty-free license).
