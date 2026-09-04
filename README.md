# Crixata

Free, local-first, open-source AI Terms of Service / Privacy Policy grader.

Crixata automatically detects ToS and Privacy Policy pages, extracts a structured summary using a local small language model, and grades the policy **A–D**. All inference happens on your machine. No browsing history or policy content is ever sent to a remote server.

> **Privacy guarantee:** *No user data, browsing history, or policy content is ever tracked, collected, or sent to a remote server.*

## Components

- [`apps/desktop/`](apps/desktop/) — Tauri v2 desktop companion (Rust backend, lightweight web frontend)
- [`apps/extension/`](apps/extension/) — Chrome Manifest V3 extension (detection, badge, popup)
- [`packages/cache-schema/`](packages/cache-schema/) — JSON schema and TypeScript types for the public pre-graded policy cache
- [`cache-repo/`](cache-repo/) — Stand-in for the public cache repository (`Lophostrix/crixata-cache`)
- [`docs/`](docs/) — Architecture, PRD, and build instructions

## Core principles

- **Zero telemetry** — no analytics, accounts, or crash reporting.
- **Only three permitted network calls:**
  1. One-time on-demand download of the SLM model file from Hugging Face.
  2. Periodic cache sync via jsDelivr CDN.
  3. Browser extension fetching policy page text.
- **No Node.js server** — the companion app is a compiled Tauri v2 native executable.
- **Local SQLite cache** with optional sync from a public cache repo.
- **GBNF-constrained SLM output** to enforce a strict JSON schema.

## Quick start

### Desktop app

```bash
cd apps/desktop
npm install
cargo tauri dev
```

On first launch, click **Download Model & Engine** to fetch the local model and `llama-server` sidecar.

### Chrome extension

```bash
cd apps/extension
npm install
npm run build
```

Then load the `apps/extension/dist/` folder as an unpacked extension in Chrome:
1. Open `chrome://extensions/`
2. Enable **Developer mode**
3. Click **Load unpacked**
4. Select `apps/extension/dist/`

### Production build

```bash
cd apps/desktop
npm run build
cargo tauri build
```

Installers will be written to `apps/desktop/src-tauri/target/release/bundle/`.

## Hardware requirements

The MVP uses a **Llama-3.2-3B-Instruct** model (Q4_K_M quantization, ~2 GB):

- **Storage:** ~2.5 GB for the model + engine + app
- **RAM:** 4 GB or more recommended for comfortable inference
- **CPU:** modern dual-core or better
- **OS:** Linux, Windows, or macOS

## Network behavior

| Call | Destination | When |
|---|---|---|
| Model download | `huggingface.co` | On-demand, first setup |
| Sidecar download | `github.com/ggerganov/llama.cpp/releases` | On-demand, first setup |
| Cache sync | `cdn.jsdelivr.net/gh/Lophostrix/crixata-cache` | Periodically and on manual trigger |
| Page fetch | The website you are visiting | When extension extracts policy text |

No other network traffic is initiated.

## Architecture

See [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) for the full data flow and API contract.

## Status

MVP is feature-complete. See open tasks and known limitations in [`docs/BUILD.md`](docs/BUILD.md).
