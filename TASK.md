# Current Task: Implement Cache Sync from jsDelivr CDN

## Goal
Add a public pre-graded policy cache sync mechanism to the Crixata desktop app. The app should periodically fetch JSON shards from a jsDelivr-backed GitHub repo and merge them into the local SQLite cache.

## Context
- Project root: `/home/honeysh/projects/Crixata`
- Desktop app: `apps/desktop/`
- Cache schema package: `packages/cache-schema/`
- Base URL configured in `config.rs`: `https://cdn.jsdelivr.net/gh/Lophostrix/crixata-cache@main/shards`
- The public cache repo is `Lophostrix/crixata-cache`. It does not exist yet; we will create a sample shard file in this repo for testing.

## Requirements

### 1. Cache Shard Schema (`packages/cache-schema/`)
Update the schema and types to define a cache shard:

```json
{
  "version": 1,
  "updated_at": "2026-09-04T12:00:00Z",
  "shards": [
    {
      "domain": "example.com",
      "policy_url": "https://example.com/privacy",
      "policy_hash": "sha256-or-text-hash",
      "grade": "B",
      "summary": { /* same Summary object */ },
      "source": "cache",
      "graded_at": "2026-09-04T12:00:00Z"
    }
  ]
}
```

- Update `schema.json` to validate this structure.
- Update `types.ts` accordingly.
- Add a JSON Schema for individual shard files (e.g., `shard.schema.json`).
- Add sample `examples/shard-v1.json`.

### 2. Cache Sync in Rust (`apps/desktop/src-tauri/src/sync.rs`)
Implement:

- `CacheSync` struct with:
  - `base_url: String`
  - `client: reqwest::Client`
  - `cache: Arc<CacheManager>`
- `CacheSync::new(config, cache)`.
- `CacheSync::sync_all() -> Result<SyncReport>`:
  1. Fetch `index.json` from `${base_url}/index.json`.
  2. The index lists available shard filenames, e.g.:
     ```json
     { "shards": ["shard-000.json", "shard-001.json"] }
     ```
  3. For each shard file, fetch `${base_url}/${filename}`.
  4. Validate the JSON matches the schema (structurally via serde; full JSON Schema validation is optional).
  5. Merge each shard entry into SQLite using `CacheManager::upsert_grade`.
  6. Return a `SyncReport` with counts: `shards_fetched`, `entries_added`, `entries_updated`, `errors`.
- `CacheSync::sync_shard(url) -> Result<usize>` for fetching a single shard.
- Add rate limiting / politeness: do not hammer jsDelivr; sequential fetches with a small delay are fine.

### 3. SQLite Cache Updates (`apps/desktop/src-tauri/src/cache.rs`)
Ensure `upsert_grade` can receive cache entries. Add a method if needed:
- `upsert_cache_entry(domain, policy_url, policy_hash, grade, summary, source, graded_at)`

Also add:
- `get_last_sync_time() -> Option<DateTime<Utc>>` stored in a new `metadata` table.
- `set_last_sync_time()`.

Schema addition:
```sql
CREATE TABLE IF NOT EXISTS metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
```

### 4. Scheduler
In `lib.rs`, spawn a background tokio task that runs sync every 24 hours (configurable). Only run sync if the app is online and the base URL is reachable. Do not block startup.

### 5. Tauri Commands
Add commands:
- `trigger_cache_sync()` → starts a sync and returns a summary when complete.
- `get_last_sync_status()` → returns last sync time and counts.

### 6. Frontend Updates
Add to the desktop app UI:
- Button "Sync Policy Cache Now".
- Display last sync time and counts.
- Show sync errors if any.

### 7. Public Cache Repo Sample
Create `cache-repo/` in the project root as a stand-in for `Lophostrix/crixata-cache`:
- `cache-repo/README.md`
- `cache-repo/shards/index.json`
- `cache-repo/shards/shard-000.json` with 2-3 sample pre-graded policies (use realistic domains like `mozilla.org`, `wikipedia.org`, or `duckduckgo.com`).

This folder represents the source for the public cache repo and can later be pushed to `Lophostrix/crixata-cache`.

### 8. Verification
- `cargo test` passes.
- `cargo check` passes.
- Unit tests for sync parsing and cache metadata.

## Deliverables
- Updated `packages/cache-schema/` with shard schema/types and sample.
- Implemented `sync.rs` and scheduler in the desktop app.
- Local cache metadata support in `cache.rs`.
- Frontend sync UI.
- `cache-repo/` sample stand-in.
- Commit and push to `origin/main`.

## Notes
- Only jsDelivr/GitHub CDN network calls are allowed; no other remote endpoints.
- Keep shard files small (< 1 MB each) so they are easy to host and download.
- This task does not require the actual `Lophostrix/crixata-cache` repo to exist yet.
