# Crixata

Free, local-first, open-source AI Terms of Service / Privacy Policy grader.

Crixata automatically detects ToS and Privacy Policy pages, extracts a structured summary using a local SLM, and grades the policy **A–D**. All inference happens on your machine. No browsing history or policy content is ever sent to a remote server.

## Components

- `apps/desktop/` — Tauri v2 desktop companion (Rust backend, lightweight frontend)
- `apps/extension/` — Chrome MV3 extension (detection, badge, popup)
- `packages/cache-schema/` — JSON schema for the public pre-graded policy cache
- `docs/` — Architecture, PRD, and development notes

## Core principles

- **Zero telemetry** — no analytics, accounts, or crash reporting.
- **Only permitted network calls:**
  1. One-time on-demand download of the SLM model file from Hugging Face.
  2. Periodic cache sync via jsDelivr CDN.
  3. Browser extension fetching policy page text.
- **No Node.js server** — the companion app is a compiled Tauri v2 native executable.
- **Local SQLite cache** with optional sync from a public cache repo.
- **GBNF-constrained SLM output** to enforce a strict JSON schema.

## Status

Active development. See `docs/PRD.md` and `docs/ARCHITECTURE.md` for the current plan.
