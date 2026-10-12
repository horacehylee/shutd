# Shutd

Windows auto-shutdown utility. Rust + Tauri 2 backend (`src-tauri/`), Svelte 5
+ Vite + TypeScript frontend (`src/`). This is the Tauri rewrite of the old Go
app; the Go code is gone.

## Build & run

- Package manager is pnpm via corepack, pinned in `package.json`
  (`packageManager`). Prefix pnpm with corepack: `corepack pnpm install`.
  Do not bump past pnpm 11 without testing: pnpm 12 dropped corepack
  compatibility (its `bin/pnpm.cjs` is gone) and `corepack pnpm` fails with
  every pnpm 12 release.
- Dev with live reload: `corepack pnpm tauri dev` (vite serves 127.0.0.1:1420).
- Release binary: `corepack pnpm tauri build --no-bundle`, output at
  `src-tauri/target/release/shutd.exe`.
- Plain cargo release must enable the custom-protocol feature, or the exe
  loads the dev server instead of the bundled frontend:
  `cargo build --manifest-path src-tauri/Cargo.toml --release --features tauri/custom-protocol`
- When asked to "build it" (or equivalent), treat that as build **and**
  install: build the release exe, then have Horace copy it to its install
  location and confirm Panel > Settings > Launch at startup. A repo build
  alone leaves the startup binary stale.

## Release

- Bump the version in `package.json`, `src-tauri/tauri.conf.json`, and
  `src-tauri/Cargo.toml` before releasing (keep all three identical); the
  release script fails on a mismatch or an existing `v<version>` tag.
- Run `corepack pnpm release --changes "..."` (repeat `--changes` for each
  release-note bullet). It builds the portable exe, tags `v<version>`, and
  publishes the GitHub release.

## Tests

- Rust: `cargo test --manifest-path src-tauri/Cargo.toml`
- Frontend: `corepack pnpm test`
- Keep both green. Scheduler math (`scheduler.rs`), config parsing
  (`config.rs`), tray rendering (`tray.rs`), and the dismiss guard
  (`panel.rs`) have focused unit tests; frontend formatting helpers in
  `src/lib/countdown.ts` are tested with vitest. Add focused tests for real
  logic, not smoke slop.

## Layout

- `src-tauri/src/scheduler.rs` — daily schedule, snooze, toast windows (pure,
  unit-tested time math).
- `src-tauri/src/config.rs` — `~/.config/shutd/config.yaml`, hot reload,
  first-run migration from legacy `~/.shutd.yaml` (both camelCase and
  viper-lowercase keys are accepted).
- `src-tauri/src/tray.rs` — tray glyph rendered by tiny-skia at runtime;
  geometry constants at the top control padding and size.
- `src-tauri/src/panel.rs` — the single frameless window, Panel/Toast modes,
  tray anchoring, click-outside dismiss guard, WebView2 suspend on hide.
- `src-tauri/src/commands.rs` — the tauri commands the frontend invokes.
- `src/lib/backend.ts` — typed invoke wrappers; `src/lib/countdown.ts` — pure
  time formatting.
- `design/` — approved HTML mockups (`popup-mockups.html`,
  `tray-icon-concept.html`). Match these when touching visuals, and update
  them when a visual decision changes.

## Runtime data

- Config: `~/.config/shutd/config.yaml`, hot-reloaded on change.
- Log: `~/.config/shutd/shutd.log`.
- A running instance claims the tray via the single-instance plugin; a second
  launch just opens the panel.
