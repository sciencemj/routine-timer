# Routine Timer

A macOS-first desktop app for managing daily routines with a circular focus timer and a menubar countdown. Fill each routine's required time with a focus session — optionally in Pomodoro cycles — and watch your progress build up across the day.

Built with **Tauri v2** (Rust core) + **Svelte 5**.

---

## Features

- **오늘 / Dashboard** — daily goal ring, per-routine progress rows, and quick-start.
- **Circular focus timer** — a large ring countdown with play / pause / stop and a "next routine" suggestion.
- **Pomodoro** — 25/5 focus-break cycles (default on). A routine's Pomodoro block **suspends and resumes** across routine switches instead of restarting.
- **Menubar countdown + popover** — a live `mm:ss` in the macOS menu bar; click the tray icon for a focus/dashboard popover with rounded, translucent styling.
- **리포트 / Report** — a 13-week GitHub-style focus heatmap, weekly/average KPIs, and a 7-day bar chart.
- **Routines** — create / edit / delete, drag-to-reorder, 28 icon choices, and custom durations (chips + manual hours:minutes entry).
- **Configurable day boundary** — the "day" can start at a chosen hour (default 8 AM), so late-night focus counts toward the right day.
- **Themes** — system / light / dark.
- **Data reset** — a guarded "DB 리셋" action wipes routines + history (preferences kept).

## Tech stack

| Layer | Tech |
|---|---|
| Shell | Tauri v2 (macOS), tray icon, positioner, notifications |
| Core | Rust — timer state machine, SQLite (`rusqlite`), stats aggregation |
| UI | Svelte 5 (runes), Vite, `svelte-spa-router` |
| Tests | Vitest + Testing Library (frontend), `cargo test` (Rust core) |

## Getting started

**Prerequisites:** [Rust](https://rustup.rs/) (stable), [Bun](https://bun.sh/), and Xcode Command Line Tools on macOS.

```bash
bun install            # install frontend deps
bun run tauri dev      # run the app (hot-reload)
```

Tests / checks:

```bash
bun run check          # svelte-check (types)
bun run test           # vitest (frontend)
cd src-tauri && cargo test   # Rust core
```

## Build & local publishing

```bash
bun run tauri build
```

Produces a `.app` and a `.dmg` under:

```
src-tauri/target/release/bundle/macos/Routine Timer.app
src-tauri/target/release/bundle/dmg/Routine Timer_<version>_<arch>.dmg
```

- Builds for the host architecture by default. For a universal (Intel + Apple Silicon) binary:
  `rustup target add x86_64-apple-darwin aarch64-apple-darwin` then
  `bun run tauri build --target universal-apple-darwin`.
- **Unsigned builds** run on your own Mac, but other machines' Gatekeeper will block them unless the app is code-signed + notarized with an Apple Developer ID (set `bundle.macOS.signingIdentity` + the `APPLE_ID` / `APPLE_PASSWORD` / `APPLE_TEAM_ID` env vars).
- **Mac App Store is not supported** — the app uses `macOSPrivateApi` (for the translucent popover), so distribute via `.dmg` / direct download only.

## Project structure

```
src/                 Svelte 5 UI (routes, components, stores)
src-tauri/src/
  core/              timer engine, model, stats (pure, unit-tested)
  db/                SQLite access (routines, sessions, settings)
  commands.rs        Tauri command layer
  lib.rs             app setup: tray, popover window, tick loop
```

## Roadmap / Future updates

- **iOS app** — a companion iOS build sharing the Rust core (the timer engine, stats, and DB layer are platform-agnostic; the UI and menubar/tray integration are the macOS-specific parts).
- **Internationalization (i18n)** — the UI ships in Korean today; add a translation layer and locale switching (English first), including date/duration formatting per locale.

## License

Released under the [MIT License](LICENSE).

### Third-party fonts

Bundled fonts are used under the SIL Open Font License 1.1 (OFL-1.1):

- [Pretendard](https://github.com/orioncactus/pretendard) — © Kil Hyung-jin
- [JetBrains Mono](https://github.com/JetBrains/JetBrainsMono) — © JetBrains
- [Space Grotesk](https://github.com/floriankarsten/space-grotesk) — © Florian Karsten

Each font package ships its OFL license text. The app icon is currently a
placeholder and should be replaced with an original icon before public release.
