# Crixata Product Requirements Document (v2)

## 1. Vision

Crixata is a free, local-first, open-source browser companion that reads website Terms of Service and Privacy Policies and grades them A–D, entirely on the user's machine.

## 2. Target Users

- Privacy-conscious web users.
- People who want a quick sanity check before signing up for a service.
- Users who do not want to send their browsing data to a cloud API.

## 3. User Stories

- As a user, when I visit a site with a ToS or Privacy Policy, I want the extension badge to show the grade immediately.
- As a user, when I click the extension, I want a clear summary of what the policy says about data collection, sharing, retention, and my rights.
- As a user, I want all processing to happen locally so my browsing history is never uploaded.
- As a user, I want the app to start automatically and stay out of the way in the system tray.
- As a user on a weak machine, I want the app to tell me honestly when it cannot grade a site.

## 4. Non-Functional Requirements

- **Privacy:** No telemetry, accounts, or crash reporting. Prominent disclaimer in installer and extension UI.
- **Network:** Only model download, jsDelivr cache sync, and extension page fetches.
- **Local-first:** Inference runs on-device via llama.cpp sidecar.
- **Cross-platform:** Tauri v2 produces Windows, macOS, and Linux installers.
- **Maintainable:** Clear separation between desktop app, extension, and cache schema.

## 5. MVP Scope

This first iteration focuses on the **full 3B SLM path**.

### Included

- Tauri v2 desktop app with system tray and autostart.
- llama.cpp `llama-server` sidecar bundled per target platform.
- On-demand model download (`Llama-3.2-3B-Instruct` GGUF from Hugging Face).
- Local SQLite cache of graded policies.
- Chrome MV3 extension with proactive detection, badge, and popup.
- Local HTTP API between extension and desktop app.
- GBNF-grammar-constrained extraction to the strict JSON schema.
- A–D grade computed from extracted structured data.
- Privacy disclaimers in installer and extension popup.

### Explicitly Excluded from MVP

- Hybrid fallback for low-RAM machines.
- Firefox / Safari extensions.
- Community contribution flow for the cache repo.
- Cloud analysis or opt-in remote processing.

## 6. Success Criteria

- Extension badge updates to A/B/C/D for supported policy pages.
- The extraction JSON validates against the schema in ≥90% of manual test cases on common sites.
- No unauthorized network traffic during normal use.
- App builds successfully for at least Linux and Windows.
