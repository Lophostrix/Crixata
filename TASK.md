# Current Task: Implement Core Rust Backend

## Goal
Make the Crixata Tauri desktop app functional as a local server that can receive policy text from the extension, grade it (using a local SLM), and cache results in SQLite. This task focuses on the Rust backend and a minimal working frontend.

## Context
- Project root: `/home/honeysh/projects/Crixata`
- Desktop app: `apps/desktop/`
- Docs: `docs/PRD.md`, `docs/ARCHITECTURE.md`
- Extension will be implemented in a later task.

## Requirements

### 1. Configuration (`apps/desktop/src-tauri/src/config.rs`)
- Define `AppConfig` with methods returning:
  - `cache_db_path()` — SQLite path in app data dir.
  - `model_dir()` — directory for the downloaded GGUF model.
  - `model_path()` — path to `Llama-3.2-3B-Instruct.Q4_K_M.gguf`.
  - `llama_server_bin_path()` — path to downloaded `llama-server` executable.
  - `server_port` and `llama_server_port` defaults (e.g., 4343 and 4344).
- Use `dirs::data_dir()` / `dirs::data_local_dir()` for cross-platform paths.
- Create directories on first use.

### 2. Model Download Manager (`apps/desktop/src-tauri/src/model.rs`)
- Implement `ModelManager` that can:
  - Check whether the model file exists locally.
  - Download the GGUF model from Hugging Face with progress reporting:
    - URL: `https://huggingface.co/bartowski/Llama-3.2-3B-Instruct-GGUF/resolve/main/Llama-3.2-3B-Instruct-Q4_K_M.gguf`
    - (Accept a model URL config option; the above is the default.)
  - Report download progress as percentage.
  - Resume interrupted downloads if possible (range requests optional; nice-to-have).
  - Download the appropriate pre-compiled `llama-server` binary from the official llama.cpp releases on GitHub into the app data dir, based on target triple:
    - Linux x86_64: `llama-server-x86_64-unknown-linux-gnu`
    - Linux aarch64: `llama-server-aarch64-unknown-linux-gnu`
    - macOS x86_64: `llama-server-x86_64-apple-darwin`
    - macOS aarch64: `llama-server-aarch64-apple-darwin`
    - Windows x86_64: `llama-server-x86_64-pc-windows-msvc.exe`
    - Map target triple to a release asset URL.
    - Mark the binary executable on Unix.

### 3. Sidecar Manager (`apps/desktop/src-tauri/src/sidecar.rs`)
- Rewrite `SidecarManager` to:
  - Spawn `llama-server` using `std::process::Command` (not Tauri sidecar feature for now).
  - Args: `-m <model_path> --port <port> -c 4096 --host 127.0.0.1`
  - Wait for `/health` endpoint to respond before marking `Ready`.
  - Stop/kill the process on app exit.
  - Restart on failure up to a max retry count.
- Keep health-check logic.

### 4. SQLite Cache (`apps/desktop/src-tauri/src/cache.rs`)
- Schema:
  ```sql
  CREATE TABLE IF NOT EXISTS grades (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      domain TEXT NOT NULL,
      policy_url TEXT NOT NULL,
      policy_hash TEXT,
      grade TEXT NOT NULL,
      summary_json TEXT NOT NULL,
      source TEXT NOT NULL, -- 'llm' or 'cache'
      created_at INTEGER NOT NULL,
      updated_at INTEGER NOT NULL,
      UNIQUE(domain, policy_url)
  );
  CREATE INDEX IF NOT EXISTS idx_domain ON grades(domain);
  ```
- Implement:
  - `CacheManager::open(path)` / `CacheManager::in_memory()`.
  - `get_grade(domain, policy_url)` → optional grade record.
  - `upsert_grade(domain, policy_url, policy_hash, grade, summary, source)`.
  - `list_recent(limit)`.

### 5. Local HTTP Server (`apps/desktop/src-tauri/src/server.rs`)
- Use `tiny_http`.
- Endpoints:
  - `GET /health` → JSON `{ "status": "ok", "sidecar_ready": bool, "model_downloaded": bool }`.
  - `POST /grade` → accepts `{ "url", "title", "text" }`, returns grade response.
- `/grade` flow:
  1. Validate `url` and `text` are non-empty.
  2. Derive domain from URL.
  3. Check cache; return cached result if found.
  4. If not cached and sidecar is ready, call the LLM extraction function.
  5. Compute grade from extracted summary.
  6. Store in cache and return.
- CORS: allow `chrome-extension://*` origins.

### 6. LLM Extraction Stub (`apps/desktop/src-tauri/src/llm.rs` — new file)
- Define the strict extraction schema types.
- Implement `extract_policy_summary(text: &str, client: &reqwest::Client, port: u16) -> Result<Summary>`.
- For now, implement the prompt and call `/completion` on the local llama-server.
- Use a GBNF grammar string passed in the request body (`grammar` field) to constrain output.
- If the sidecar is not ready or model is missing, return a clear error.
- The actual GBNF grammar file can live at `apps/desktop/src-tauri/resources/policy_schema.gbnf`.

### 7. Grading Logic (`apps/desktop/src-tauri/src/grade.rs`)
- Define `Summary` struct matching the JSON schema in `docs/ARCHITECTURE.md`.
- Implement `compute_grade(summary: &Summary) -> char` with the rubric from the architecture doc.
- Grade is deterministic based on extracted fields.

### 8. Wiring (`apps/desktop/src-tauri/src/lib.rs`)
- Construct config, cache, sidecar manager, and start the local server.
- Add Tauri commands:
  - `get_app_status()`
  - `download_model()` with progress events (emit `model-download-progress` events).
  - `get_download_status()`
- Ensure the sidecar is started only after the model exists (or at least model file is present).
- On app shutdown, stop the sidecar.

### 9. Minimal Frontend (`apps/desktop/src/`)
- Show the privacy disclaimer.
- Show app status (sidecar, model, cache).
- Button to trigger model download with progress bar.
- Display the local API port.

### 10. Build Verification
- `cd apps/desktop && npm run tauri dev` must compile and start without errors.
- If `llama-server` or model is missing, the app should gracefully show download UI instead of crashing.
- `cargo check` should pass.

## Deliverables
- All Rust modules implemented and compiling.
- Frontend status/download page functional.
- Local HTTP server responds to `/health`.
- Commit and push to `origin/main`.

## Notes
- Do **not** commit the downloaded `llama-server` binary or GGUF model.
- Do **not** implement the browser extension yet; only the desktop backend.
- Use the existing skeleton files where they exist; rewrite if needed.
- Keep error handling explicit with `anyhow`/`thiserror`.
