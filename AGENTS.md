# Arcade Launcher

A fullscreen arcade cabinet frontend for student-created games. Targets Windows,
runs in a custom arcade cabinet at a school. Built with Electrobun (Bun + native
webview), SolidJS, and Vite.

---

## Structure

```
arcade-launcher/
├── electrobun.config.ts   — app name, identifier, Vite dist copy rules
├── vite.config.ts         — Vite root: src/mainview, out: dist/, solid plugin
├── tsconfig.json          — strict TS, jsxImportSource: solid-js
├── package.json
├── PLAN.md                — implementation roadmap and phase checklist
├── AGENTS.md              — this file
└── src/
    ├── shared/
    │   └── types.ts       — GameEntry, AppConfig, DownloadProgress, ArcadeRPCType
    ├── bun/
    │   └── index.ts       — main process: BrowserWindow, RPC handlers, Bun.spawn
    └── mainview/          — renderer (SolidJS app, built by Vite)
        ├── index.html
        ├── main.tsx       — SolidJS root mount, Electroview RPC init
        ├── style.css      — global styles, CRT aesthetic, CSS custom properties
        ├── components/    — SolidJS components (GameGrid, GameDetail, etc.)
        ├── stores/        — SolidJS stores and signals shared across components
        └── fonts/         — bundled Press Start 2P font files
```

---

## Commands

| Command                  | What it does                                              |
|--------------------------|-----------------------------------------------------------|
| `bun run start`          | Vite build then launch app via Electrobun (no HMR)        |
| `bun run dev:hmr`        | Vite dev server + Electrobun in parallel (HMR enabled)    |
| `bun run build:canary`   | Production build via Electrobun                           |

---

## Architecture

Electrobun runs two isolated processes:

**Bun process** (`src/bun/index.ts`) — the main process. Has full system access.
Creates the `BrowserWindow`, owns all RPC handlers, spawns game/MAME subprocesses
via `Bun.spawn()`, reads/writes files, and makes all network requests. Never
touches the DOM.

**Renderer** (`src/mainview/`) — a webview running the SolidJS app, built by Vite.
Has no filesystem or system access. All communication with the bun process goes
through typed RPC. The renderer calls bun for data and actions; bun pushes events
back as fire-and-forget messages.

**RPC bridge** — types are defined once in `src/shared/types.ts` as `ArcadeRPCType`.
The bun side wires handlers via `BrowserView.defineRPC<ArcadeRPCType>()` and passes
the result to `BrowserWindow`. The renderer initialises with `Electroview.defineRPC<ArcadeRPCType>()`
inside `main.tsx` and exports the `rpc` object for use in stores and components.

```
src/shared/types.ts  ←  imported by both sides, never contains runtime code
```

---

## Renderer Conventions

- **Framework**: SolidJS. Use signals and stores for state, not refs or classes.
- **JSX**: `.tsx` files. `jsxImportSource` is `solid-js` — no React imports.
- **Components**: one component per file in `src/mainview/components/`.
- **Global state**: SolidJS stores in `src/mainview/stores/`. The RPC instance and
  game list live here; components read from stores, not directly from RPC calls.
- **Styles**: global CSS in `style.css` using CSS custom properties for theming.
  Component-scoped styles co-located as `ComponentName.css` if needed.
- **No side effects in component bodies** outside `createEffect` / `onMount`.

---

## Key Constraints

- **Windows only.** All paths use `%APPDATA%` via `process.env.APPDATA` in the bun
  process. No macOS/Linux path assumptions.
- **Fullscreen, no titlebar.** The window is `titleBarStyle: "hidden"` and calls
  `win.setFullScreen(true)` on startup. The renderer fills 100vw × 100vh with no
  scrollbars.
- **All system calls go through the bun process.** The renderer must never attempt
  to read files, spawn processes, or make cross-origin requests. Use RPC.
- **Gamepad API polling, not events.** Input is read each frame in a
  `requestAnimationFrame` loop via `navigator.getGamepads()`. The Gamepad API does
  not reliably fire events in all webview environments.
- **Google Sheet as CSV, no API key.** The sheet is published via
  _File → Share → Publish to web → CSV_. The bun process fetches the raw CSV URL.
- **Google Drive downloads go through the bun process.** Drive direct-download URLs
  require handling a virus-scan redirect for large files; the renderer only receives
  progress messages.
- **Runtime data in `%APPDATA%\arcade-launcher\`.** Config, cache, and downloaded
  game files all live here. See `PLAN.md` for the full layout.
