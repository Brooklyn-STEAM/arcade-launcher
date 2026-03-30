# Arcade Launcher — Build Plan

A fullscreen arcade cabinet frontend for student-created games, built with Tauri v2
(Rust + Vite + native webview) targeting Windows.

---

## Current State

Initialized from the Tauri v2 + SolidJS starter template. The project builds and runs.
Nothing arcade-specific has been written yet.

```
src/
├── App.tsx               — boilerplate greet demo, delete everything
├── App.css               — boilerplate styles, replace entirely
└── index.tsx             — SolidJS root mount, fine as-is
src-tauri/src/
├── lib.rs                — boilerplate greet command, replace entirely
└── main.rs               — binary entry point, fine as-is
src-tauri/tauri.conf.json — identifier already correct; window config needs updating
vite.config.ts            — fine as-is (port 1420, ignores src-tauri/)
```

---

## How It Works

```
Instructor edits Google Sheet
        │
        ▼
lib.rs  ──fetch CSV──►  parse GameEntry[]  ──write──►  games-cache.json
        │                                                           │
        │  invoke()                                          read on offline
        ▼
src/stores/  ──renders──►  game grid  ──select──►  detail screen
        │
        │  invoke: launch_game / launch_mame
        ▼
lib.rs  ──std::process::Command──►  game.exe / mame64.exe
                                          │
                                  hide launcher window
                                  restore on process exit
```

---

## Google Sheet Schema

The instructor publishes the sheet as CSV via **File → Share → Publish to web → CSV**.
No API key required.

| Column                | Example value        | Notes                                    |
| --------------------- | -------------------- | ---------------------------------------- |
| `title`               | Dungeon Escape       |                                          |
| `author`              | Jane & Carlos        |                                          |
| `description`         | Navigate the maze... | Shown on detail screen                   |
| `thumbnail_drive_id`  | `1aBcDeFgHiJ...`     | Google Drive file ID                     |
| `executable_drive_id` | `1xYz012345...`      | Drive file ID — zip or exe               |
| `version`             | `1.2`                | Increment triggers automatic re-download |
| `enabled`             | `TRUE`               | `FALSE` hides without deleting row       |

Thumbnail URL pattern (loaded directly in renderer):

```
https://drive.google.com/uc?export=view&id=<thumbnail_drive_id>
```

Executable files must be shared as **"Anyone with the link — Viewer"**. Large files
trigger a Drive virus-scan interstitial; the download code must handle that redirect.

---

## Local Storage

All runtime data lives in `%APPDATA%\arcade-launcher\`.

```
%APPDATA%\arcade-launcher\
├── config.json          — sheetCsvUrl, mamePath, mameArgs, gamesDir override
├── games-cache.json     — last-fetched sheet rows + local manifest (version, path)
└── games\
    └── <game-id>\       — extracted game files live here
```

`config.json` defaults (created on first run if missing):

```json
{
  "sheetCsvUrl": "",
  "mamePath": "C:\\mame\\mame64.exe",
  "mameArgs": [],
  "gamesDir": ""
}
```

`gamesDir` is optional; defaults to `%APPDATA%\arcade-launcher\games\` when empty.

---

## IPC Contract

Defined in `src/shared/types.ts` (renderer-only TS types). Rust command signatures
live in `src-tauri/src/lib.rs` and are registered via `tauri::generate_handler![]`.

**Renderer → Rust (via `invoke()`):**

| Command          | Params                             | Response                                       |
| ---------------- | ---------------------------------- | ---------------------------------------------- |
| `fetch_games`    | —                                  | `{ games: GameEntry[], fromCache: boolean }`   |
| `download_game`  | `{ gameId, driveFileId, version }` | `{ success, localPath?, error? }`              |
| `check_updates`  | —                                  | `{ needsUpdate: string[] }` (array of gameIds) |
| `launch_game`    | `{ gameId }`                       | `{ success, error? }`                          |
| `launch_mame`    | —                                  | `{ success, error? }`                          |
| `get_config`     | —                                  | `AppConfig`                                    |

**Rust → Renderer (via `app_handle.emit()` / `listen()`):**

| Event              | Payload                                                        |
| ------------------ | -------------------------------------------------------------- |
| `downloadProgress` | `{ gameId, bytesReceived, totalBytes, percent, done, error? }` |
| `gameExited`       | `{ gameId }`                                                   |
| `mameExited`       | `{}`                                                           |

---

## Gamepad Input Map

The arcade stick appears as an XInput gamepad. Navigation is polled each frame via
the Gamepad API (`requestAnimationFrame` loop). Keyboard arrow keys + Enter/Escape
are a fallback.

| XInput button index   | Physical button | Action               |
| --------------------- | --------------- | -------------------- |
| Axis 0/1 (left stick) | Joystick        | Move focus           |
| 12 / 13 / 14 / 15     | D-pad           | Move focus           |
| 0                     | A               | Confirm / launch     |
| 9                     | Start           | Confirm / launch     |
| 1                     | B               | Back (detail → grid) |
| 4 / 5                 | LB / RB         | Page left / right    |

Analog stick requires deadzone filtering (threshold ≈ 0.25). Buttons require
debounce — fire once per press, not continuously while held.

---

## Konami Code

Sequence tracked in the renderer against both keyboard and gamepad input:

```
↑ ↑ ↓ ↓ ← → ← → B A
```

Buffer resets after 10 seconds of inactivity. On match:

1. Play screen-flash + glitch animation in the renderer.
2. Send `launch_mame` invoke.
3. Rust hides the launcher window and spawns MAME.
4. On MAME exit, Rust emits `mameExited` event and restores the window.

---

## Phases

### Phase 1 — Project cleanup

- [x] Set app identifier to `nyc.steamcenter.arcade-launcher` in `tauri.conf.json`
- [ ] Configure window in `tauri.conf.json`: `fullscreen: true`, `decorations: false`, `width: 1920`, `height: 1080`
- [ ] Delete boilerplate greet command from `lib.rs` and remove `App.tsx` / `App.css` demo
- [ ] Add `src/shared/types.ts` with `GameEntry`, `AppConfig`, `DownloadProgress` TS types

### Phase 2 — Rust backend (Tauri commands)

- [ ] On startup: read `config.json` from `app_data_dir()`; create with defaults if missing
- [ ] `fetch_games`: fetch CSV from `sheetCsvUrl` via `reqwest`, parse rows into `GameEntry[]`, write to `games-cache.json`; return cached data if fetch fails
- [ ] `check_updates`: diff sheet versions against `games-cache.json` manifest, return stale game IDs
- [ ] `download_game`: stream Drive file to disk via `reqwest`; handle virus-scan redirect; unzip if `.zip`; emit `downloadProgress` events; update manifest
- [ ] `launch_game`: resolve local exe path from manifest; `std::process::Command` spawn; call `window.hide()`; watch for exit → restore window + emit `gameExited`
- [ ] `launch_mame`: `std::process::Command` with `mamePath`/`mameArgs`; same hide/restore + emit `mameExited`
- [ ] Register all commands with `tauri::generate_handler![]`
- [ ] Add `reqwest` (with `blocking` or async) and `zip` crates to `Cargo.toml`

### Phase 3 — Renderer UI

- [ ] Replace `App.tsx` with arcade shell; import `invoke` from `@tauri-apps/api/core` and `listen` from `@tauri-apps/api/event` in stores
- [ ] On load: call `fetch_games` → render grid; call `check_updates` → silently queue downloads in background
- [ ] **Grid screen**: scrollable tile grid — thumbnail, title, author. Max visible tiles determined by viewport; rest scroll.
- [ ] **Detail screen**: full cover art (Drive thumbnail), title, author, description, "PRESS START" prompt; slide in over the grid
- [ ] **Download progress overlay**: progress bar anchored to tile; listens for `downloadProgress` events; auto-dismisses on `done: true`
- [ ] **Offline/error state**: if `fetch_games` returns `fromCache: true`, show a small "OFFLINE" badge; if no cache exists at all, show a full-screen error

### Phase 4 — Gamepad navigation

- [ ] `requestAnimationFrame` polling loop; read `navigator.getGamepads()`
- [ ] Deadzone helper for analog axes
- [ ] Button debounce: track which buttons were pressed last frame
- [ ] Focus model: single `focusedIndex` integer; arrow input mutates it, wraps at edges
- [ ] Apply CSS focus class to the focused tile (neon glow ring)
- [ ] Keyboard fallback handler (same actions as gamepad)

### Phase 5 — Konami code + MAME

- [ ] Input sequence buffer; append gamepad and keyboard inputs to the same buffer; reset on 10s timeout
- [ ] Detect `↑↑↓↓←→←→BA` sequence
- [ ] Trigger flash animation, call `invoke('launch_mame')`
- [ ] Handle `mameExited` event: restore launcher, play "welcome back" animation

### Phase 6 — Retro visual design

- [ ] Replace system font with `Press Start 2P` (bundle locally under `src/fonts/`)
- [ ] Color palette: `#0a0a0a` background, `#ff2d78` primary accent, `#00e5ff` secondary, `#f5ff00` highlight
- [ ] CRT scanline overlay: full-viewport `::after` pseudo-element, `pointer-events: none`, horizontal repeating gradient, ~3% opacity
- [ ] Scanline flicker: `@keyframes` brightness oscillation, 8s loop, subtle
- [ ] Tile focus ring: `box-shadow: 0 0 0 3px <accent>, 0 0 24px <accent>` — animated pulse
- [ ] Boot sequence: power-on animation on app start (brief static → scanline sweep → logo fade in)
- [ ] Launch transition: screen flash + brief "INSERT COIN" overlay before game spawns
- [ ] Marquee text: CSS overflow + keyframe translate for titles longer than tile width

---

## Deferred / Future

- **Admin overlay**: hold Start + Select for 3s → settings screen (edit Sheet URL, MAME path) without recompiling
- **Attract mode**: screensaver cycling game art after N minutes idle
- **Sound effects**: coin insert, blip on navigation, launch whoosh (Web Audio API)
- **Multiple thumbnails per game**: cycle on detail screen
- **Auto-launch on boot**: Windows Task Scheduler setup guide
