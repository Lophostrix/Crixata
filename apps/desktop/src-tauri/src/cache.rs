use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::grade::PolicySummary;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradeRecord {
    pub id: i64,
    pub domain: String,
    pub policy_url: String,
    pub policy_hash: Option<String>,
    pub grade: String,
    pub summary_json: String,
    pub summary: PolicySummary,
    pub source: String,
    pub created_at: i64,
    pub updated_at: i64,
}

pub struct CacheManager {
    conn: Mutex<Connection>,
}

impl CacheManager {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory {:?}", parent))?;
        }

        let conn = Connection::open(path)
            .with_context(|| format!("Failed to open SQLite database at {:?}", path))?;

        let manager = Self {
            conn: Mutex::new(conn),
        };
        manager.init_schema()?;
        Ok(manager)
    }

    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let manager = Self {
            conn: Mutex::new(conn),
        };
        manager.init_schema()?;
        Ok(manager)
    }

    fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS grades (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                domain TEXT NOT NULL,
                policy_url TEXT NOT NULL,
                policy_hash TEXT,
                grade TEXT NOT NULL,
                summary_json TEXT NOT NULL,
                source TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                UNIQUE(domain, policy_url)
            );
            CREATE INDEX IF NOT EXISTS idx_domain ON grades(domain);
            CREATE TABLE IF NOT EXISTS sync_shards (
                shard_id TEXT PRIMARY KEY,
                version TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                synced_at TEXT NOT NULL
            );",
        )?;

        Ok(())
    }

    pub fn get_grade(&self, domain: &str, policy_url: &str) -> Result<Option<GradeRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, domain, policy_url, policy_hash, grade, summary_json, source, created_at, updated_at
             FROM grades WHERE domain = ?1 AND policy_url = ?2 LIMIT 1",
        )?;

        let mut rows = stmt.query(params![domain, policy_url])?;
        if let Some(row) = rows.next()? {
            let summary_json: String = row.get(5)?;
            let summary: PolicySummary = serde_json::from_str(&summary_json)
                .with_context(|| "Failed to deserialize cached PolicySummary JSON")?;

            Ok(Some(GradeRecord {
                id: row.get(0)?,
                domain: row.get(1)?,
                policy_url: row.get(2)?,
                policy_hash: row.get(3)?,
                grade: row.get(4)?,
                summary_json,
                summary,
                source: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn get_by_url(&self, policy_url: &str) -> Result<Option<GradeRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, domain, policy_url, policy_hash, grade, summary_json, source, created_at, updated_at
             FROM grades WHERE policy_url = ?1 LIMIT 1",
        )?;

        let mut rows = stmt.query(params![policy_url])?;
        if let Some(row) = rows.next()? {
            let summary_json: String = row.get(5)?;
            let summary: PolicySummary = serde_json::from_str(&summary_json)
                .with_context(|| "Failed to deserialize cached PolicySummary JSON")?;

            Ok(Some(GradeRecord {
                id: row.get(0)?,
                domain: row.get(1)?,
                policy_url: row.get(2)?,
                policy_hash: row.get(3)?,
                grade: row.get(4)?,
                summary_json,
                summary,
                source: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn upsert_grade(
        &self,
        domain: &str,
        policy_url: &str,
        policy_hash: Option<&str>,
        grade: &str,
        summary: &PolicySummary,
        source: &str,
    ) -> Result<GradeRecord> {
        let summary_json = serde_json::to_string(summary)?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO grades (domain, policy_url, policy_hash, grade, summary_json, source, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)
             ON CONFLICT(domain, policy_url) DO UPDATE SET
                policy_hash = excluded.policy_hash,
                grade = excluded.grade,
                summary_json = excluded.summary_json,
                source = excluded.source,
                updated_at = excluded.updated_at",
            params![
                domain,
                policy_url,
                policy_hash,
                grade,
                summary_json,
                source,
                now,
            ],
        )?;

        let id = conn.last_insert_rowid();
        drop(conn);

        // Fetch actual record
        self.get_grade(domain, policy_url)?
            .with_context(|| format!("Failed to retrieve upserted record for {} (id: {})", policy_url, id))
    }

    pub fn list_recent(&self, limit: usize) -> Result<Vec<GradeRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, domain, policy_url, policy_hash, grade, summary_json, source, created_at, updated_at
             FROM grades ORDER BY updated_at DESC LIMIT ?1",
        )?;

        let rows = stmt.query_map(params![limit as i64], |row| {
            let summary_json: String = row.get(5)?;
            let summary: PolicySummary = serde_json::from_str(&summary_json).unwrap_or_else(|_| PolicySummary {
                data_collected: vec![],
                data_used_for: vec![],
                shared_with_third_parties: false,
                third_party_details: "".into(),
                retention_period: "".into(),
                user_rights: Default::default(),
                tracking_and_ads: "".into(),
                arbitration_or_class_action_waiver: false,
                policy_clarity_notes: "".into(),
            });

            Ok(GradeRecord {
                id: row.get(0)?,
                domain: row.get(1)?,
                policy_url: row.get(2)?,
                policy_hash: row.get(3)?,
                grade: row.get(4)?,
                summary_json,
                summary,
                source: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })?;

        let mut list = Vec::new();
        for item in rows {
            list.push(item?);
        }
        Ok(list)
    }

    pub fn count_grades(&self) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM grades", [], |r| r.get(0))?;
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grade::UserRights;

    #[test]
    fn test_cache_crud() {
        let cache = CacheManager::in_memory().unwrap();

        let summary = PolicySummary {
            data_collected: vec!["email".into()],
            data_used_for: vec!["auth".into()],
            shared_with_third_parties: false,
            third_party_details: "None".into(),
            retention_period: "30 days".into(),
            user_rights: UserRights::default(),
            tracking_and_ads: "None".into(),
            arbitration_or_class_action_waiver: false,
            policy_clarity_notes: "Clear".into(),
        };

        let record = cache
            .upsert_grade(
                "example.com",
                "https://example.com/privacy",
                Some("hash123"),
                "A",
                &summary,
                "llm",
            )
            .unwrap();

        assert_eq!(record.domain, "example.com");
        assert_eq!(record.grade, "A");

        let fetched = cache
            .get_grade("example.com", "https://example.com/privacy")
            .unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().summary.retention_period, "30 days");

        let recent = cache.list_recent(10).unwrap();
        assert_eq!(recent.len(), 1);
    }
}
