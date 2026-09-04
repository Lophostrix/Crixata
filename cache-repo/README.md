# Crixata Policy Cache (`crixata-cache`)

This repository serves as the public repository and CDN distribution source for Crixata pre-graded Terms of Service and Privacy Policy caches.

## Structure

```
cache-repo/
├── README.md
└── shards/
    ├── index.json          # Index of active shard filenames
    └── shard-000.json      # Pre-graded policy cache shard (version 1)
```

## CDN Distribution

Shards are distributed globally via the jsDelivr CDN endpoint:
`https://cdn.jsdelivr.net/gh/Lophostrix/crixata-cache@main/shards`

### Endpoints
- **Index:** `https://cdn.jsdelivr.net/gh/Lophostrix/crixata-cache@main/shards/index.json`
- **Shard:** `https://cdn.jsdelivr.net/gh/Lophostrix/crixata-cache@main/shards/shard-000.json`

## Schema Specification

Each shard file validates against the JSON Schema defined in `@crixata/cache-schema` (`shard.schema.json`):

```json
{
  "version": 1,
  "updated_at": "2026-09-04T12:00:00Z",
  "shards": [
    {
      "domain": "example.com",
      "policy_url": "https://example.com/privacy",
      "policy_hash": "sha256-or-text-hash",
      "grade": "A",
      "summary": { ... },
      "source": "cache",
      "graded_at": "2026-09-04T12:00:00Z"
    }
  ]
}
```
