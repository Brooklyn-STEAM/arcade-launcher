# Arcade Launcher

A fullscreen arcade cabinet frontend for student-created games. Targets Windows,
runs in a custom arcade cabinet at a school. Built with Tauri v2 (Rust backend +
native webview), SolidJS, and Vite.

---

## Structure

```
arcade-launcher/
├── src-tauri/             — Rust/Tauri backend
│   ├── tauri.conf.json    — app name, identifier, window config, build commands
│   ├── Cargo.toml         — Rust dependencies
│   ├── capabilities/
│   │   └── default.json   — Tauri capability grants for the main window
│   └── src/
│       ├── main.rs        — binary entry point, calls lib::run()
│       ├── lib.rs         — Tauri commands, event emitters, process spawning, axum server startup
│       └── admin.html     — management web UI (embedded in binary via include_str!)
├── src/                   — renderer (SolidJS app, built by Vite)
│   ├── index.tsx          — SolidJS root mount
│   ├── shared/
│   │   └── types.ts       — GameEntry, AppConfig (TS types)
│   ├── style.css          — global styles, CRT aesthetic, CSS custom properties
│   ├── components/        — SolidJS components (GameGrid, GameDetail, etc.)
│   ├── stores/            — SolidJS stores and signals shared across components
│   └── fonts/             — bundled Press Start 2P font files
├── index.html             — Vite HTML entry point
├── vite.config.ts         — solid plugin, port 1420, ignores src-tauri/
├── tsconfig.json          — strict TS, jsxImportSource: solid-js
├── package.json
├── PLAN.md                — implementation roadmap and phase checklist
└── AGENTS.md              — this file
```

---

## Commands

| Command            | What it does                                           |
| ------------------ | ------------------------------------------------------ |
| `vp run tauri dev`   | Vite dev server + Tauri app shell in parallel (HMR)    |
| `vp run tauri build` | Vite production build then Rust compile + installer    |
| `vp dev`           | Vite dev server only (renderer, no Tauri shell)        |
| `vp build`         | Vite production build only                             |
| `vp check`         | Format, lint, and TypeScript type checks               |

---

## Architecture

Tauri runs two isolated processes:

**Rust process** (`src-tauri/src/lib.rs`) — the backend. Has full system access.
Owns all Tauri command handlers, spawns game/MAME subprocesses via
`std::process::Command`, reads/writes `games-cache.json` and `config.json`, and
runs the embedded axum web server. Emits events to the renderer. Never touches the DOM.

**Renderer** (`src/`) — a webview running the SolidJS app, built by Vite. Has no
filesystem or system access. All communication with the Rust process goes through
Tauri's IPC. The renderer calls Rust commands via `invoke()` and listens for
fire-and-forget events via `listen()`.

**IPC bridge** — Rust commands are registered with `tauri::generate_handler![]` and
decorated with `#[tauri::command]`. The renderer uses `invoke('commandName', args)`
from `@tauri-apps/api/core` to call them. Rust emits events with
`app_handle.emit('eventName', payload)`; the renderer subscribes with `listen()`
from `@tauri-apps/api/event`.

**Admin web server** — an axum HTTP server spawned in a background tokio task on
startup. Listens on `0.0.0.0:8037`. Serves `admin.html` at `GET /` and a REST API
for managing the game registry. Accessible from any browser on the school LAN.
All mutating routes require the `X-Admin-Pin` header to match `config.adminPin`.






---

## Renderer Conventions

- **Framework**: SolidJS. Use signals and stores for state, not refs or classes.
- **JSX**: `.tsx` files. `jsxImportSource` is `solid-js` — no React imports.
- **Components**: one component per file in `src/components/`.
- **Global state**: SolidJS stores in `src/stores/`. Invoke results and game list
  live here; components read from stores, not directly from `invoke()` calls.
- **Styles**: global CSS in `src/style.css` using CSS custom properties for theming.
  Component-scoped styles co-located as `ComponentName.css` if needed.
- **No side effects in component bodies** outside `createEffect` / `onMount`.

---

## Key Constraints

- **Windows only.** The Rust process resolves `%APPDATA%` via Tauri's
  `app.path().app_data_dir()`. No macOS/Linux path assumptions.
- **Fullscreen, no titlebar.** `tauri.conf.json` sets `fullscreen: true` and
  `decorations: false` on the main window. The renderer fills 100vw × 100vh with
  no scrollbars.
- **All system calls go through the Rust process.** The renderer must never attempt
  to read files, spawn processes, or make cross-origin requests. Use `invoke()`.
- **Gamepad API polling, not events.** Input is read each frame in a
  `requestAnimationFrame` loop via `navigator.getGamepads()`. The Gamepad API does
  not reliably fire events in all webview environments.
- **Game registry is local.** `games-cache.json` in `%APPDATA%\arcade-launcher\` is
  the authoritative source of truth. No external server or Google Sheets dependency.
- **Games are uploaded, not downloaded.** The instructor uploads ZIP files via the
  admin web UI at `http://<cabinet-ip>:8037`. The Rust process extracts them to
  `games/<id>/`. The renderer never fetches game files from the network.
- **Admin server runs inside the launcher.** An axum HTTP server on port 8037 is
  spawned at startup and shut down on exit. It serves the admin UI and a REST API
  for managing `games-cache.json`. PIN-protected via `X-Admin-Pin` header.
- **Runtime data in `%APPDATA%\arcade-launcher\`.** Config, registry, and game files
  all live here. See `PLAN.md` for the full layout.



<!--VITE PLUS START-->

# Using Vite+, the Unified Toolchain for the Web

This project is using Vite+, a unified toolchain built on top of Vite, Rolldown, Vitest, tsdown, Oxlint, Oxfmt, and Vite Task. Vite+ wraps runtime management, package management, and frontend tooling in a single global CLI called `vp`. Vite+ is distinct from Vite, but it invokes Vite through `vp dev` and `vp build`.

## Vite+ Workflow

`vp` is a global binary that handles the full development lifecycle. Run `vp help` to print a list of commands and `vp <command> --help` for information about a specific command.

### Start

- create - Create a new project from a template
- migrate - Migrate an existing project to Vite+
- config - Configure hooks and agent integration
- staged - Run linters on staged files
- install (`i`) - Install dependencies
- env - Manage Node.js versions

### Develop

- dev - Run the development server
- check - Run format, lint, and TypeScript type checks
- lint - Lint code
- fmt - Format code
- test - Run tests

### Execute

- run - Run monorepo tasks
- exec - Execute a command from local `node_modules/.bin`
- dlx - Execute a package binary without installing it as a dependency
- cache - Manage the task cache

### Build

- build - Build for production
- pack - Build libraries
- preview - Preview production build

### Manage Dependencies

Vite+ automatically detects and wraps the underlying package manager such as pnpm, npm, or Yarn through the `packageManager` field in `package.json` or package manager-specific lockfiles.

- add - Add packages to dependencies
- remove (`rm`, `un`, `uninstall`) - Remove packages from dependencies
- update (`up`) - Update packages to latest versions
- dedupe - Deduplicate dependencies
- outdated - Check for outdated packages
- list (`ls`) - List installed packages
- why (`explain`) - Show why a package is installed
- info (`view`, `show`) - View package information from the registry
- link (`ln`) / unlink - Manage local package links
- pm - Forward a command to the package manager

### Maintain

- upgrade - Update `vp` itself to the latest version

These commands map to their corresponding tools. For example, `vp dev --port 3000` runs Vite's dev server and works the same as Vite. `vp test` runs JavaScript tests through the bundled Vitest. The version of all tools can be checked using `vp --version`. This is useful when researching documentation, features, and bugs.

## Common Pitfalls

- **Using the package manager directly:** Do not use pnpm, npm, or Yarn directly. Vite+ can handle all package manager operations.
- **Always use Vite commands to run tools:** Don't attempt to run `vp vitest` or `vp oxlint`. They do not exist. Use `vp test` and `vp lint` instead.
- **Running scripts:** Vite+ commands take precedence over `package.json` scripts. If there is a `test` script defined in `scripts` that conflicts with the built-in `vp test` command, run it using `vp run test`.
- **Do not install Vitest, Oxlint, Oxfmt, or tsdown directly:** Vite+ wraps these tools. They must not be installed directly. You cannot upgrade these tools by installing their latest versions. Always use Vite+ commands.
- **Use Vite+ wrappers for one-off binaries:** Use `vp dlx` instead of package-manager-specific `dlx`/`npx` commands.
- **Import JavaScript modules from `vite-plus`:** Instead of importing from `vite` or `vitest`, all modules should be imported from the project's `vite-plus` dependency. For example, `import { defineConfig } from 'vite-plus';` or `import { expect, test, vi } from 'vite-plus/test';`. You must not install `vitest` to import test utilities.
- **Type-Aware Linting:** There is no need to install `oxlint-tsgolint`, `vp lint --type-aware` works out of the box.

## Review Checklist for Agents

- [ ] Run `vp install` after pulling remote changes and before getting started.
- [ ] Run `vp check` and `vp test` to validate changes.
<!--VITE PLUS END-->
