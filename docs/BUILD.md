# Build Guide

## Prerequisites

- [Node.js](https://nodejs.org/) 22+
- [Rust](https://rustup.rs/) stable
- [Tauri CLI](https://tauri.app/start/create-project/): `cargo install tauri-cli --locked`

### Linux only

```bash
sudo apt-get update
sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf
```

## Repository structure

```text
.
├── apps/
│   ├── desktop/          # Tauri v2 desktop app
│   └── extension/        # Chrome MV3 extension
├── packages/
│   └── cache-schema/     # Shared schema/types
├── cache-repo/           # Stand-in public cache repo
└── docs/                 # PRD, architecture, build guide
```

## Desktop app

```bash
cd apps/desktop
npm install

# Development mode with hot reload
cargo tauri dev

# Production build
cargo tauri build
```

Installers are written to `apps/desktop/src-tauri/target/release/bundle/`.

### First launch

The app starts a local HTTP server on `127.0.0.1:4343`. Click **Download Model & Engine** in the UI to download:

- `Llama-3.2-3B-Instruct-Q4_K_M.gguf` (~2 GB) from Hugging Face
- `llama-server` for your platform from the official llama.cpp releases

Both are saved to your local app data directory and never uploaded anywhere.

## Extension

```bash
cd apps/extension
npm install
npm run build
```

The unpacked extension is output to `apps/extension/dist/`.

Load it in Chrome:
1. Open `chrome://extensions/`
2. Enable **Developer mode**
3. Click **Load unpacked**
4. Select `apps/extension/dist/`

## Running tests

```bash
# Rust backend tests
cd apps/desktop/src-tauri
cargo test

# Type checks
cd apps/desktop && npm run build
cd apps/extension && npm run build
```

## Cache sync

The desktop app syncs pre-graded policies from `https://cdn.jsdelivr.net/gh/Lophostrix/crixata-cache@main/shards`.

A stand-in cache repo is in `cache-repo/`. To publish it, push it to `Lophostrix/crixata-cache` or update `cdn_shard_base_url` in `apps/desktop/src-tauri/src/config.rs`.

## Known limitations

- The first run requires downloading ~2 GB of model data.
- Inference speed depends on CPU; GPU acceleration is not configured in this MVP.
- Windows and macOS installers are configured in CI but have not been manually tested on this Linux machine.
- The cache repo currently contains only sample data.
