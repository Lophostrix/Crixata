use std::path::Path;
use std::sync::Mutex;
use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::grade::{Grade, PolicySummary};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedPolicyRecord {
    pub domain: String,
    pub policy_url: String,
    pub policy_type: String,
    pub policy_version_hash: String,
    pub grade: Grade,
    pub summary: PolicySummary,
    pub graded_at: String,
    pub model_version: String,
    pub source: String,
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
        conn.execute(
            "CREATE TABLE IF NOT EXISTS policies (
                domain TEXT NOT NULL,
                policy_url TEXT NOT NULL,
                policy_type TEXT NOT NULL,
                policy_version_hash TEXT PRIMARY KEY,
                grade TEXT NOT NULL,
                summary_json TEXT NOT NULL,
                graded_at TEXT NOT NULL,
                model_version TEXT NOT NULL,
                source TEXT NOT NULL
            );",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_policies_domain_url 
             ON policies(domain, policy_url);",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS sync_shards (
                shard_id TEXT PRIMARY KEY,
                version TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                synced_at TEXT NOT NULL
            );",
            [],
        )?;

        Ok(())
    }

    pub fn get_by_url(&self, url: &str) -> Result<Option<CachedPolicyRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT domain, policy_url, policy_type, policy_version_hash, grade, summary_json, graded_at, model_version, source
             FROM policies WHERE policy_url = ?1 LIMIT 1",
        )?;

        let mut rows = stmt.query(params![url])?;
        if let Some(row) = rows.next()? {
            let grade_str: String = row.get(4)?;
            let grade = match grade_str.as_str() {
                "A" => Grade::A,
                "B" => Grade::B,
                "C" => Grade::C,
                _ => Grade::D,
            };
            let summary_json: String = row.get(5)?;
            let summary: PolicySummary = serde_json::from_str(&summary_json)?;

            Ok(Some(CachedPolicyRecord {
                domain: row.get(0)?,
                policy_url: row.get(1)?,
                policy_type: row.get(2)?,
                policy_version_hash: row.get(3)?,
                grade,
                summary,
                graded_at: row.get(6)?,
                model_version: row.get(7)?,
                source: row.get(8)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn insert_policy(&self, record: &CachedPolicyRecord) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let summary_json = serde_json::to_string(&record.summary)?;

        conn.execute(
            "INSERT OR REPLACE INTO policies 
             (domain, policy_url, policy_type, policy_version_hash, grade, summary_json, graded_at, model_version, source)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                record.domain,
                record.policy_url,
                record.policy_type,
                record.policy_version_hash,
                record.grade.to_string(),
                summary_json,
                record.graded_at,
                record.model_version,
                record.source,
            ],
        )?;

        Ok(())
    }

    pub fn count_policies(&self) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM policies", [], |r| r.get(0))?;
        Ok(count)
    }
}
