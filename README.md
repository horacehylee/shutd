# Shutd

Auto shutdown utility for Windows: tray panel, snooze toast, dark mode.
Rust + Tauri 2 + Svelte 5.

## Build

```sh
corepack pnpm install
corepack pnpm tauri build -- --no-bundle
```

Binary: `src-tauri/target/release/shutd.exe` (about 9 MB, WebView2 is a system
component).

For development: `corepack pnpm tauri dev`.

## Use

1. Run `shutd.exe`. A power glyph appears in the tray.
2. Left click the tray icon to open the panel: next shutdown time, Snooze
   button, Settings. Settings edits save and hot-reload the scheduler.
3. A toast slides up above the taskbar 10 minutes (configurable) before
   shutdown. Snooze pushes it back, Dismiss closes the toast, timeout lets the
   shutdown proceed.
4. Click anywhere outside the panel to dismiss it.

## Config

`~/.config/shutd/config.yaml` — editable in the panel or by hand, hot-reloaded
either way. A legacy `~/.shutd.yaml` migrates on first run.

```yaml
startTime: "01:00"       # daily shutdown time (HH:MM)
snoozeInterval: 15       # minutes a snooze delays shutdown
notification:
  before: 10             # minutes before shutdown to show the toast
  duration: 10           # minutes before the toast auto-dismisses
```

Logs: `~/.config/shutd/shutd.log`.

## Autostart

Panel > Settings > Launch at startup. Replaces the old `shutd.vbs` workaround.

## Tests

```sh
cargo test --manifest-path src-tauri/Cargo.toml
corepack pnpm test
```

## License

MIT. See [LICENSE](./LICENSE).
