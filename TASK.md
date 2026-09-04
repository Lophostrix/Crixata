# Current Task: Implement Chrome MV3 Extension

## Goal
Build a complete Chrome Manifest V3 extension that detects ToS/Privacy Policy pages, extracts policy text, calls the local Crixata desktop app API, and displays the grade in the toolbar badge and popup.

## Context
- Project root: `/home/honeysh/projects/Crixata`
- Extension location: `apps/extension/`
- Desktop app local API: `http://127.0.0.1:4343`
- API endpoints: `GET /health`, `POST /grade`
- Docs: `docs/PRD.md`, `docs/ARCHITECTURE.md`

## Requirements

### 1. Fix Manifest and Build Layout
- Update `apps/extension/manifest.json` to reference built files at the root of `dist/` (not `src/`):
  - `"default_popup": "popup.html"`
  - `"service_worker": "background.js"`
  - content script: `"js": ["content.js"]`
- Update `apps/extension/vite.config.ts` so the build outputs:
  - `dist/popup.html`
  - `dist/background.js`
  - `dist/content.js`
  - `dist/icons/` copied from `apps/extension/icons/`
  - `dist/manifest.json` copied from `apps/extension/manifest.json`
  - CSS/JS assets under `dist/assets/` or similar.
- Add a `copy-manifest-and-icons` npm script or use a Vite plugin to copy statics.

### 2. Policy Detection (`src/detector.ts`)
Implement heuristics to decide whether a page is a Terms of Service or Privacy Policy page.

Check in order:
- URL path contains one of: `privacy`, `privacypolicy`, `privacy-policy`, `tos`, `terms`, `terms-of-service`, `terms-of-use`, `legal`, `eula`, `user-agreement`, `conditions`.
- Page `<title>` contains: `Privacy Policy`, `Terms of Service`, `Terms of Use`, `Terms and Conditions`, `Legal`, `Cookie Policy`.
- DOM: look for links on the page whose text or href matches the above keywords (use a content script message to get these).

Return a confidence object:
```ts
interface DetectionResult {
  isPolicyPage: boolean;
  policyType: 'privacy' | 'terms' | 'cookie' | 'unknown';
  confidence: 'high' | 'medium' | 'low';
}
```

### 3. Content Script (`src/content.ts`)
- Extract the main readable text from the page body.
- Remove script/style/nav/footer/header/aside elements where appropriate.
- Trim and limit text to ~15,000 characters (send the first N chars if longer).
- Listen for messages from the service worker:
  - `action: "extract-text"` → returns `{ text: string, title: string, url: string }`.
  - `action: "scan-links"` → returns candidate policy links found in the page.

### 4. Service Worker (`src/background.ts`)
- On tab update (`chrome.tabs.onUpdated`), when status is `complete`:
  - Use `chrome.scripting.executeScript` to call the content script and detect/scan.
  - If a policy page is detected, extract text.
  - Call `GET http://127.0.0.1:4343/health`.
  - If desktop app is healthy, call `POST http://127.0.0.1:4343/grade` with `{ url, title, text }`.
  - Update the tab's badge with the grade (A/B/C/D) and a badge color:
    - A → green (`#22c55e`)
    - B → blue (`#3b82f6`)
    - C → orange (`#f97316`)
    - D → red (`#ef4444`)
    - Unknown / error / no app → gray (`#6b7280`) or "?"
- Store the last grade result per tab in `chrome.storage.session` (or `local` if session unavailable) so the popup can read it instantly.
- Throttle: do not re-grade the same URL more than once per 5 minutes unless the user clicks the popup.

### 5. Popup (`src/popup.html`, `src/popup.ts`, `src/popup.css`)
- Query the active tab's stored grade result.
- Display:
  - Large grade badge (A/B/C/D/?).
  - Domain / policy URL.
  - Summary breakdown: data collected, data used for, third-party sharing, retention, user rights, tracking/ads, arbitration/class-action waiver, clarity notes.
  - Source label: "Analyzed locally" or "From cache".
  - A prominent privacy disclaimer: *"No user data, browsing history, or policy content is ever tracked, collected, or sent to a remote server."*
- Show a "Desktop app not running" state if `/health` fails, with instructions to launch Crixata.
- Add a "Re-analyze this page" button.

### 6. API Client (`src/api.ts`)
- Strongly typed functions:
  - `checkHealth(): Promise<HealthResponse>`
  - `gradePolicy(req: GradeRequest): Promise<GradeResponse>`
- Handle network errors gracefully (app not running).

### 7. Badge Helper (`src/badge.ts`)
- `setBadge(tabId, grade)` updates badge text and color.
- `clearBadge(tabId)` resets to "?" or empty.

### 8. Type Safety
- Use types from `packages/cache-schema/types.ts` for `Summary`.
- Add local `src/types.ts` for extension-specific types.

### 9. Build and Load Verification
- `npm run build` in `apps/extension/` must produce a valid `dist/` folder.
- The extension must be loadable in Chrome as an unpacked extension from `dist/`.
- No TypeScript errors.

### 10. Privacy Disclaimer
- Must be visible in the popup.
- Must not initiate any network requests except to `127.0.0.1:4343` and to fetch the current page's text.

## Deliverables
- Complete extension source under `apps/extension/src/`.
- Valid `apps/extension/manifest.json`.
- Working Vite build that produces a loadable `dist/`.
- Commit and push to `origin/main`.

## Notes
- Do not implement the cache-sync backend in this task; only the extension.
- Keep the popup lightweight; it should read from storage, not re-call the API on every open.
- Handle MV3 service worker lifecycle: global state will be lost, so rely on `chrome.storage`.
