# Crixata

Free, local-first, open-source AI Terms of Service / Privacy Policy grader.

Crixata automatically detects Terms of Service and Privacy Policy pages, extracts a structured summary using a local SLM, and grades the policy **A–D**. All inference happens on your machine. No browsing history or policy content is ever sent to a remote server.

---

## Monorepo Architecture & Packages

This repository is organized as an `npm` workspace:

- [`apps/desktop`](./apps/desktop/) — **Tauri v2 Desktop App**: Native Rust companion managing the local SQLite cache, `llama.cpp` sidecar, jsDelivr CDN sync, and localhost HTTP server for the browser extension.
- [`apps/extension`](./apps/extension/) — **Chrome Manifest V3 Extension**: Proactive legal policy detector, dynamic grade badges (A/B/C/D), page text extraction, and popup summary UI.
- [`packages/cache-schema`](./packages/cache-schema/) — **Cache Schema & Type Definitions**: JSON Schema (Draft-07) and TypeScript types for public pre-graded policy shards (`shard-v1.json`).
- [`docs`](./docs/) — Documentation:
  - [Product Requirements Document (PRD)](./docs/PRD.md)
  - [System Architecture](./docs/ARCHITECTURE.md)

---

## Core Principles

- **Zero telemetry** — no analytics, accounts, or crash reporting.
- **Only permitted network calls:**
  1. One-time on-demand download of the SLM model file from Hugging Face.
  2. Periodic cache sync via jsDelivr CDN (`Lophostrix/crixata-cache`).
  3. Browser extension fetching policy page text.
- **No Node.js runtime required for companion** — the companion app is a compiled Tauri v2 native executable.
- **Local SQLite cache** with optional sync from a public cache repo.
- **GBNF-constrained SLM output** to enforce a strict JSON schema.

---

## Development & Build Guide

This project standardizes on **`npm`** (npm v9+) for JavaScript/TypeScript package management across workspaces.

### Prerequisites

- **Node.js**: v18.0 or newer
- **npm**: v9.0 or newer
- **Rust & Cargo**: Rust 1.77+
- **Platform dependencies (Linux)**: `webkit2gtk-4.1` / `libsoup-3.0`, `gtk3`

### 1. Install Dependencies

Install all workspace dependencies from the monorepo root:

```bash
npm install
```

### 2. Build All Packages

Build all workspaces (desktop frontend, extension, and cache schema):

```bash
npm run build
```

Individual workspace build commands:
- Build cache schema: `npm run build:cache-schema`
- Build Chrome extension: `npm run build:extension`
- Build desktop app frontend: `npm run build:desktop`

### 3. Running the Desktop Companion App

Run the Tauri v2 desktop app in development mode:

```bash
npm run dev:desktop
# or from the app directory:
cd apps/desktop && cargo tauri dev
```

To build a standalone production desktop binary without installers:

```bash
cd apps/desktop && cargo tauri build --no-bundle
```

The compiled binary will be located in `apps/desktop/src-tauri/target/release/desktop`.

### 4. Loading the Chrome Extension

1. Build the extension:
   ```bash
   npm run build:extension
   ```
2. Open Google Chrome (or Chromium/Brave) and navigate to `chrome://extensions`.
3. Enable **Developer mode** in the top right corner.
4. Click **Load unpacked** and select the `apps/extension/dist` directory.

### 5. Validating Cache Shards

The JSON schema in `packages/cache-schema/schema.json` validates cache shards. Validate the example shard:

```bash
python3 -c "import json, jsonschema; jsonschema.validate(json.load(open('packages/cache-schema/examples/shard-v1.json')), json.load(open('packages/cache-schema/schema.json')))"
```

---

## License

Apache-2.0 / MIT. See LICENSE for details.
