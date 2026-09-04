# Current Task: Initialize Crixata MVP Scaffold

## Goal
Set up the foundational codebase for Crixata v2:
1. Initialize a Tauri v2 desktop app in `apps/desktop/`.
2. Scaffold a Chrome Manifest V3 extension in `apps/extension/`.
3. Create the cache-schema package in `packages/cache-schema/`.
4. Configure the monorepo build/workspace files.

## Context
Crixata is a free, local-first, open-source AI ToS/Privacy Policy grader. The MVP uses:
- Tauri v2 (Rust backend, lightweight web frontend)
- llama.cpp `llama-server` sidecar (to be bundled later)
- Chrome MV3 extension
- Local SQLite cache
- Public cache synced from jsDelivr CDN

Read `docs/PRD.md` and `docs/ARCHITECTURE.md` for the full plan.

## Requirements

### 1. Tauri v2 app (`apps/desktop/`)
- Use `npm create tauri-app@latest` or the cargo equivalent.
- Target a vanilla TS + Vite frontend (no heavy framework).
- Add required Tauri plugins:
  - `tauri-plugin-autostart`
  - `tauri-plugin-positioner` (optional but useful for tray)
  - `tauri-plugin-shell` or `tauri-plugin-process` for sidecar management
  - `tauri-plugin-http` for extension API (or use custom Rust commands + tiny_http/axum)
- Configure `tauri.conf.json`:
  - App name: "Crixata"
  - Identifier: `com.lophostrix.crixata`
  - Sidecar support enabled with a placeholder `llama-server` target triple.
  - System tray enabled.
- Add Rust crates:
  - `rusqlite` for SQLite
  - `serde`, `serde_json`, `tokio`, `reqwest`, `anyhow`, `thiserror`
  - `tauri-plugin-autostart`
  - `tauri-plugin-positioner`
  - `tauri-plugin-shell` (if used)
- Rust backend modules (create empty skeleton files):
  - `src/sidecar.rs` — manages llama-server lifecycle
  - `src/cache.rs` — SQLite cache operations
  - `src/sync.rs` — jsDelivr cache sync
  - `src/grade.rs` — grading logic
  - `src/model.rs` — model download manager
  - `src/server.rs` — local HTTP server for extension (if not using Tauri commands)
  - `src/config.rs` — app configuration
- Frontend pages:
  - `index.html` — main setup/status window
  - `src/main.ts` with a status view
  - Privacy disclaimer shown prominently.

### 2. Chrome MV3 extension (`apps/extension/`)
- `manifest.json` version 3 with:
  - `host_permissions`: `<all_urls>`
  - `permissions`: `activeTab`, `scripting`, `storage`, `tabs`
  - `action` popup
  - `background.service_worker`
  - `content_scripts` for all URLs
- Files:
  - `src/background.ts` — listens to tab updates, detects policy pages
  - `src/content.ts` — extracts page text
  - `src/popup.html` / `src/popup.ts` — shows grade, summary, disclaimer
  - `src/api.ts` — client for the Tauri local HTTP API
  - `src/detector.ts` — URL/title/link heuristics for policy pages
  - `src/badge.ts` — badge update helpers
- Build with Vite or plain `tsc`.

### 3. Cache schema package (`packages/cache-schema/`)
- `schema.json` — JSON Schema for a cache shard.
- `types.ts` — TypeScript types mirroring the schema.
- Example shard in `examples/shard-v1.json`.

### 4. Monorepo/workspace
- Add `package.json` at repo root with workspaces or just scripts to build both apps.
- Add a root `README.md` update linking to apps/packages.
- Keep committed files buildable: do not commit node_modules or target dirs.

## Deliverables
- `apps/desktop/` builds with `cargo tauri dev` without errors (even if UI is minimal).
- `apps/extension/` loads in Chrome as an unpacked extension.
- `packages/cache-schema/` has valid JSON Schema and example.
- All changes committed and pushed to `origin/main`.

## Notes
- Do not implement the full logic yet; skeletons and correct wiring are enough.
- Use `npm` or `pnpm`; pick one and document it.
- Do not bundle the actual GGUF model or llama-server binary yet; leave placeholders/config only.
