# Crixata Architecture (v2)

## High-level data flow

```
┌─────────────────┐      fetch page text       ┌──────────────────┐
│ Chrome MV3      │ ─────────────────────────▶ │ Tauri v2 Desktop │
│ Extension       │                            │ App (Rust)       │
│                 │ ◀───────────────────────── │                  │
└─────────────────┘     grade + summary        │  • SQLite cache  │
                                               │  • sidecar mgr   │
                                               └────────┬─────────┘
                                                        │ local HTTP
                                               ┌────────▼─────────┐
                                               │ llama-server     │
                                               │ sidecar          │
                                               └────────┬─────────┘
                                                        │ GGUF model
                                               ┌────────▼─────────┐
                                               │ Llama-3.2-3B-Instruct
                                               │ (downloaded once)
                                               └──────────────────┘

Cache sync:
Tauri app  ──(https)──▶  cdn.jsdelivr.net/gh/Lophostrix/crixata-cache@main/...
```

## Components

### 1. Desktop app (`apps/desktop/`)

- **Rust backend:** manages SQLite, sidecar lifecycle, model download, local HTTP server for the extension, cache sync.
- **Frontend:** lightweight Tauri webview for setup, model download progress, status, and privacy disclaimer.
- **Autostart / tray:** `tauri-plugin-autostart`, system tray icon with grade status.

### 2. Sidecar (`apps/desktop/src-tauri/sidecars/`)

- Pre-compiled `llama-server` binaries for Windows, macOS, Linux.
- Tauri sidecar feature bundles the binary and spawns it as a child process.
- Rust backend communicates via localhost HTTP to `/completion`.

### 3. Extension (`apps/extension/`)

- **Service worker (MV3):** listens to tab updates, inspects URL/title/DOM for policy pages.
- **Content script:** extracts clean policy text from the page.
- **Popup:** displays grade, summary breakdown, and privacy disclaimer.
- **Badge:** shows A/B/C/D or "?" for unknown/ungraded pages.

### 4. Cache sync

- Public cache repo: `Lophostrix/crixata-cache`.
- Shards fetched via jsDelivr CDN.
- Local SQLite merges shards by domain + policy URL + version hash.

## Local API contract (extension ↔ desktop)

### `POST /grade`

Request:

```json
{
  "url": "https://example.com/privacy",
  "title": "Privacy Policy - Example",
  "text": "<plain text of policy>"
}
```

Response:

```json
{
  "grade": "B",
  "summary": {
    "data_collected": ["email", "IP address", "usage data"],
    "data_used_for": ["service provision", "personalization", "analytics"],
    "shared_with_third_parties": true,
    "third_party_details": "Analytics and advertising partners.",
    "retention_period": "Account lifetime plus 30 days.",
    "user_rights": {
      "can_delete_data": true,
      "can_export_data": false,
      "can_opt_out_of_tracking": true
    },
    "tracking_and_ads": "Uses cookies and third-party ad trackers.",
    "arbitration_or_class_action_waiver": false,
    "policy_clarity_notes": "Readable but vague on retention."
  },
  "cached": false,
  "source": "llm"
}
```

### `GET /health`

Returns whether the app and sidecar are ready.

## SLM prompt and GBNF grammar

The prompt instructs the model to act as a privacy-policy analyst and return only the JSON object. The `llama-server` API is called with `grammar` set to a GBNF definition matching the schema exactly, preventing runaway generation.

## Grading rubric (MVP)

| Grade | Meaning | Criteria |
|---|---|---|
| A | Strong privacy | Minimal collection, no third-party sharing, clear rights, no arbitration waiver |
| B | Generally fair | Some tracking or sharing, but rights exist and clarity is reasonable |
| C | Concerning | Broad data collection, significant third-party sharing, weak rights, or forced arbitration |
| D | Poor | Excessive collection, unclear retention, no deletion/export, arbitration/class-action waiver |

The grade is computed deterministically from the extracted JSON fields.
