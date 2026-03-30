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
Instructor's browser (any machine on school LAN)
        │  http://<cabinet-ip>:8037  (PIN protected)
        ▼
axum web server (embedded in Rust process, always running)
  ├── GET  /                      → serves admin HTML/JS (embedded in binary)
  ├── GET  /api/games             → reads games.json, returns JSON array
  ├── POST /api/games             → add/update a game entry in games.json
  ├── DELETE /api/games/:id       → removes entry + deletes games/<id>/ folder
  └── POST /api/games/:id/upload  → streams ZIP → extracts to games/<id>/
        │
        ▼
games.json  ←→  %APPDATA%\arcade-launcher\games.json
        │
        ▼
launcher reads on startup → Tauri IPC → SolidJS store → game grid
        │
        │  invoke: launch_game / launch_mame
        ▼
lib.rs  ──std::process::Command──►  game.exe / mame64.exe
                                          │
                                  hide launcher window
                                  restore on process exit
```

---

## Game Entry Schema (`games.json`)

The `games.json` file is the authoritative registry. It is an array of
`GameEntry` objects written and read by the embedded axum server and the Tauri
commands.

| Field             | Example value                          | Notes                                    |
| ----------------- | -------------------------------------- | ---------------------------------------- |
| `id`              | `dungeon-escape`                       | URL-safe slug; used as the folder name   |
| `title`           | `Dungeon Escape`                       |                                          |
| `author`          | `Jane & Carlos`                        |                                          |
| `description`     | `Navigate the maze...`                 | Shown on detail screen                   |
| `thumbnailPath`   | `games/dungeon-escape/thumbnail.png`   | Relative to `%APPDATA%\arcade-launcher\` |
| `executablePath`  | `games/dungeon-escape/Game.exe`        | Relative to `%APPDATA%\arcade-launcher\` |
| `version`         | `1.2`                                  | Informational; updated on re-upload      |
| `enabled`         | `true`                                 | `false` hides without deleting           |

All paths are **local** — no URLs, no Drive IDs.

Example `games.json`:

```json
[
  {
    "id": "dungeon-escape",
    "title": "Dungeon Escape",
    "author": "Jane & Carlos",
    "description": "Navigate the maze...",
    "thumbnailPath": "games/dungeon-escape/thumbnail.png",
    "executablePath": "games/dungeon-escape/DungeonEscape.exe",
    "version": "1.2",
    "enabled": true
  }
]
```

---

## Local Storage

All runtime data lives in `%APPDATA%\arcade-launcher\`.

```
%APPDATA%\arcade-launcher\
├── config.json          — adminPin, mamePath, mameArgs, gamesDir override
├── games.json     — authoritative game registry (array of GameEntry)
└── games\
    └── <game-id>\       — extracted game files live here
        ├── thumbnail.png
        ├── Game.exe
        └── ...
```

`config.json` defaults (created on first run if missing):

```json
{
  "adminPin": "1234",
  "mamePath": "C:\\mame\\mame64.exe",
  "mameArgs": [],
  "gamesDir": ""
}
```

`gamesDir` is optional; defaults to `%APPDATA%\arcade-launcher\games\` when empty.

---

## Admin Web UI (port 8037)

A plain HTML + vanilla JS single-page app embedded in the Rust binary via
`include_str!`. The axum server serves it at `GET /`. No build step required.

The PIN (`adminPin` from `config.json`) is sent as an `X-Admin-Pin` header on all
mutating requests. All non-GET routes return `401` if the PIN is missing or wrong.

**Features:**
- List all games (title, author, version, enabled status)
- Add game: form with title, author, description, version, enabled toggle + ZIP upload
- Edit game: same form pre-populated; re-upload replaces files
- Delete game: removes entry from `games.json` + deletes `games/<id>/` folder
- Thumbnail preview: served via `GET /games/<id>/<filename>` static route
- PIN entry: shown on page load; stored in `sessionStorage`

**ZIP upload contract:**
- The ZIP must contain the game executable (`.exe`) at its root or one level deep
- The ZIP may optionally contain a `thumbnail.png` at its root
- On upload the server extracts all files to `games/<id>/`, auto-detects the first
  `.exe` as `executablePath`, looks for `thumbnail.png` as `thumbnailPath`

---

## IPC Contract

Defined in `src/shared/types.ts` (renderer-only TS types). Rust command signatures
live in `src-tauri/src/lib.rs` and are registered via `tauri::generate_handler![]`.

**Renderer → Rust (via `invoke()`):**

| Command        | Params         | Response                          |
| -------------- | -------------- | --------------------------------- |
| `load_games`   | —              | `{ games: GameEntry[] }`          |
| `launch_game`  | `{ gameId }`   | `{ success, error? }`             |
| `launch_mame`  | —              | `{ success, error? }`             |
| `get_config`   | —              | `AppConfig`                       |

**Rust → Renderer (via `app_handle.emit()` / `listen()`):**

| Event        | Payload        |
| ------------ | -------------- |
| `gameExited` | `{ gameId }`   |
| `mameExited` | `{}`           |

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
- [ ] Delete boilerplate greet command from `lib.rs` and remove `App.tsx` / `App.css` demo
- [ ] Add `src/shared/types.ts` with `GameEntry`, `AppConfig` TS types

### Phase 2 — Rust backend

- [ ] On startup: read `config.json` from `app_data_dir()`; create with defaults if missing
- [ ] On startup: read `games.json`; create empty array if missing
- [ ] `load_games`: read and return parsed `games.json`
- [ ] `launch_game`: resolve local exe path from registry; `std::process::Command` spawn; call `window.hide()`; watch for exit → restore window + emit `gameExited`
- [ ] `launch_mame`: `std::process::Command` with `mamePath`/`mameArgs`; same hide/restore + emit `mameExited`
- [ ] `get_config`: return parsed `AppConfig`
- [ ] Register all commands with `tauri::generate_handler![]`
- [ ] Spawn axum server on `0.0.0.0:8037` in a background tokio task on startup
- [ ] axum route `GET /api/games`: read and return `games.json` as JSON
- [ ] axum route `POST /api/games`: add or update a `GameEntry` in `games.json`
- [ ] axum route `DELETE /api/games/:id`: remove entry + delete `games/<id>/` folder
- [ ] axum route `POST /api/games/:id/upload`: receive ZIP, extract to `games/<id>/`, auto-detect exe + thumbnail, update entry in `games.json`
- [ ] axum route `GET /games/:id/:file`: serve static files from `games/<id>/` (for thumbnail previews in admin UI)
- [ ] axum route `GET /`: serve embedded admin HTML
- [ ] PIN middleware: all non-GET routes require `X-Admin-Pin` header matching `config.adminPin`; return 401 otherwise
- [ ] Add `axum`, `tower-http`, `zip` crates to `Cargo.toml`

### Phase 3 — Admin web UI

- [ ] `src-tauri/src/admin.html`: plain HTML + vanilla JS single-page admin UI
- [ ] Game list view: fetch `GET /api/games`, render table with title/author/version/enabled
- [ ] Add/edit form: title, author, description, version, enabled checkbox, ZIP file input
- [ ] Upload flow: POST to `/api/games/:id/upload` with multipart form; show progress
- [ ] Delete: DELETE `/api/games/:id` with confirmation prompt
- [ ] PIN gate: prompt for PIN on load, store in `sessionStorage`, send as `X-Admin-Pin`
- [ ] Thumbnail preview using `GET /games/:id/thumbnail.png`

### Phase 4 — Renderer UI

- [ ] Replace `App.tsx` with arcade shell; import `invoke` from `@tauri-apps/api/core` and `listen` from `@tauri-apps/api/event` in stores
- [ ] On load: call `load_games` → render grid
- [ ] **Grid screen**: scrollable tile grid — thumbnail, title, author. Max visible tiles determined by viewport; rest scroll.
- [ ] **Detail screen**: thumbnail, title, author, description, "PRESS START" prompt; slide in over the grid
- [ ] **Offline/error state**: if `load_games` returns empty array, show full-screen "NO GAMES LOADED" message with admin UI URL

### Phase 5 — Gamepad navigation

- [ ] `requestAnimationFrame` polling loop; read `navigator.getGamepads()`
- [ ] Deadzone helper for analog axes
- [ ] Button debounce: track which buttons were pressed last frame
- [ ] Focus model: single `focusedIndex` integer; arrow input mutates it, wraps at edges
- [ ] Apply CSS focus class to the focused tile (neon glow ring)
- [ ] Keyboard fallback handler (same actions as gamepad)

### Phase 6 — Konami code + MAME

- [ ] Input sequence buffer; append gamepad and keyboard inputs to the same buffer; reset on 10s timeout
- [ ] Detect `↑↑↓↓←→←→BA` sequence
- [ ] Trigger flash animation, call `invoke('launch_mame')`
- [ ] Handle `mameExited` event: restore launcher, play "welcome back" animation

### Phase 7 — Retro visual design

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

- **Attract mode**: screensaver cycling game art after N minutes idle
- **Sound effects**: coin insert, blip on navigation, launch whoosh (Web Audio API)
- **Multiple thumbnails per game**: cycle on detail screen
- **Auto-launch on boot**: Windows Task Scheduler setup guide
- **Admin PIN change UI**: change PIN from within the admin web UI without editing `config.json`
