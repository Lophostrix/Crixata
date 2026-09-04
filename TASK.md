# Current Task: End-to-End Integration, Build Verification, and Documentation

## Goal
Verify that the Crixata desktop app, extension, and cache sync work together. Fix any integration issues. Build installers for Linux and Windows. Update documentation. Push continuously.

## Context
- Desktop app: `apps/desktop/`
- Extension: `apps/extension/`
- Cache stand-in: `cache-repo/`
- Local API: `http://127.0.0.1:4343`
- llama-server port: `http://127.0.0.1:4344`

## Requirements

### 1. Integration Fixes
- Make sure the desktop app starts the local HTTP server correctly.
- Ensure the extension can reach `127.0.0.1:4343` (CORS, permissions).
- Fix any manifest/Vite/asset issues discovered during testing.
- Ensure `cache.rs` and `server.rs` handle errors gracefully when model/sidecar are missing.

### 2. Build Verification
- Run `cargo check` and `cargo test` for the desktop app.
- Run `npm run build` for the extension.
- Run `npm run build` for the desktop frontend.
- If possible, run `cargo tauri build` for Linux. Document any failures honestly.

### 3. Windows / Cross-Platform Build Prep
- Ensure `tauri.conf.json` targets are reasonable.
- Add CI workflow skeleton (GitHub Actions) in `.github/workflows/build.yml` that:
  - Builds the desktop app on `ubuntu-latest`, `windows-latest`, and `macos-latest`.
  - Builds the extension.
  - Runs `cargo test`.
- This is a skeleton; full CI may need secrets/signing later.

### 4. Documentation
- Update root `README.md` with:
  - What Crixata is.
  - How to build the desktop app.
  - How to load the extension.
  - How cache sync works.
  - Privacy statement and network calls.
  - Hardware requirements (3B model ~2 GB).
- Update `docs/ARCHITECTURE.md` if anything changed.
- Add `docs/BUILD.md` with step-by-step build instructions.

### 5. Final Cleanup
- Remove any committed `target/`, `dist/`, `node_modules/`, or `.gguf` files if they slipped in.
- Verify `.gitignore` covers all build artifacts.
- Remove `TASK.md` from tracking or update it to a roadmap.

### 6. Push
- Commit all changes with clear messages.
- Push to `origin/main`.

## Deliverables
- All tests passing.
- Extension builds successfully.
- Desktop app builds successfully for Linux (report Windows/macOS CI status).
- Documentation is complete.
- Repo is clean of build artifacts.

## Notes
- Do not spend time on code signing or store publishing yet.
- If `cargo tauri build` fails due to missing system dependencies, document the exact error and next steps.
- Be honest about what works and what is still TODO.
