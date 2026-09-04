use std::sync::Arc;
use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::cache::CacheManager;
use crate::config::AppConfig;
use crate::grade::{Grade, PolicySummary};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardIndex {
    pub shards: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardEntry {
    pub domain: String,
    pub policy_url: String,
    #[serde(alias = "policy_version_hash", default)]
    pub policy_hash: Option<String>,
    pub grade: Grade,
    pub summary: PolicySummary,
    #[serde(default = "default_cache_source")]
    pub source: String,
    #[serde(default)]
    pub graded_at: Option<String>,
}

fn default_cache_source() -> String {
    "cache".to_string()
}

fn default_version() -> u32 {
    1
}

fn deserialize_version<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    struct VersionVisitor;
    impl<'de> serde::de::Visitor<'de> for VersionVisitor {
        type Value = u32;
        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("an integer or version string")
        }
        fn visit_u64<E>(self, v: u64) -> Result<u32, E> {
            Ok(v as u32)
        }
        fn visit_i64<E>(self, v: i64) -> Result<u32, E> {
            Ok(v as u32)
        }
        fn visit_str<E: Error>(self, v: &str) -> Result<u32, E> {
            let major = v.split('.').next().unwrap_or("1");
            major.parse::<u32>().map_err(E::custom)
        }
    }
    deserializer.deserialize_any(VersionVisitor)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardPayload {
    #[serde(default = "default_version", deserialize_with = "deserialize_version")]
    pub version: u32,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(alias = "policies", alias = "entries")]
    pub shards: Vec<ShardEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct SyncReport {
    pub shards_fetched: usize,
    pub entries_added: usize,
    pub entries_updated: usize,
    pub errors: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_sync_time: Option<String>,
}

/// Backwards compatibility aliases
pub type ShardPolicyEntry = ShardEntry;
pub type CacheSyncer = CacheSync;

pub struct CacheSync {
    pub base_url: String,
    pub client: Client,
    pub cache: Arc<CacheManager>,
}

impl CacheSync {
    pub fn new(config: &AppConfig, cache: Arc<CacheManager>) -> Self {
        Self {
            base_url: config.cdn_shard_base_url.trim_end_matches('/').to_string(),
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .unwrap_or_default(),
            cache,
        }
    }

    pub fn with_client(base_url: String, client: Client, cache: Arc<CacheManager>) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client,
            cache,
        }
    }

    /// Checks if the CDN base URL is reachable before attempting sync.
    pub async fn is_reachable(&self) -> bool {
        let index_url = format!("{}/index.json", self.base_url);
        match self
            .client
            .head(&index_url)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
        {
            Ok(resp) => resp.status().is_success() || resp.status().is_redirection(),
            Err(_) => {
                match self
                    .client
                    .get(&index_url)
                    .timeout(std::time::Duration::from_secs(5))
                    .send()
                    .await
                {
                    Ok(resp) => resp.status().is_success() || resp.status().is_redirection(),
                    Err(_) => false,
                }
            }
        }
    }

    /// Fetches and applies a single shard from either a full URL or relative filename.
    pub async fn sync_shard(&self, url: &str) -> Result<usize> {
        let full_url = if url.starts_with("http://") || url.starts_with("https://") {
            url.to_string()
        } else {
            format!("{}/{}", self.base_url, url.trim_start_matches('/'))
        };

        let resp = self
            .client
            .get(&full_url)
            .send()
            .await
            .with_context(|| format!("Failed to fetch cache shard from {}", full_url))?;

        if !resp.status().is_success() {
            anyhow::bail!("CDN returned error status {} for shard at {}", resp.status(), full_url);
        }

        let shard: ShardPayload = resp
            .json()
            .await
            .with_context(|| format!("Failed to parse cache shard JSON from {}", full_url))?;

        let mut inserted = 0;
        for item in shard.shards {
            if self
                .cache
                .upsert_cache_entry(
                    &item.domain,
                    &item.policy_url,
                    item.policy_hash.as_deref(),
                    &item.grade.to_string(),
                    &item.summary,
                    &item.source,
                    item.graded_at.as_deref(),
                )
                .is_ok()
            {
                inserted += 1;
            }
        }

        Ok(inserted)
    }

    /// Syncs all shards listed in `${base_url}/index.json`.
    pub async fn sync_all(&self) -> Result<SyncReport> {
        let index_url = format!("{}/index.json", self.base_url);
        let resp = self
            .client
            .get(&index_url)
            .send()
            .await
            .with_context(|| format!("Failed to fetch cache index from {}", index_url))?;

        if !resp.status().is_success() {
            anyhow::bail!("CDN returned error status {} for index.json", resp.status());
        }

        let index: ShardIndex = resp
            .json()
            .await
            .with_context(|| "Failed to parse cache index.json")?;

        let mut report = SyncReport::default();

        for (idx, filename) in index.shards.iter().enumerate() {
            // Politeness: brief delay between sequential fetches to avoid hammering CDN
            if idx > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            }

            let shard_url = if filename.starts_with("http://") || filename.starts_with("https://") {
                filename.clone()
            } else {
                format!("{}/{}", self.base_url, filename.trim_start_matches('/'))
            };

            let shard_resp = match self.client.get(&shard_url).send().await {
                Ok(r) => r,
                Err(e) => {
                    report.errors.push(format!("Network error fetching {}: {}", filename, e));
                    continue;
                }
            };

            if !shard_resp.status().is_success() {
                report.errors.push(format!(
                    "HTTP status {} fetching {}",
                    shard_resp.status(),
                    filename
                ));
                continue;
            }

            let payload: ShardPayload = match shard_resp.json().await {
                Ok(p) => p,
                Err(e) => {
                    report
                        .errors
                        .push(format!("JSON parsing error for {}: {}", filename, e));
                    continue;
                }
            };

            report.shards_fetched += 1;

            for item in payload.shards {
                let is_update = match self.cache.get_grade(&item.domain, &item.policy_url) {
                    Ok(Some(_)) => true,
                    _ => false,
                };

                match self.cache.upsert_cache_entry(
                    &item.domain,
                    &item.policy_url,
                    item.policy_hash.as_deref(),
                    &item.grade.to_string(),
                    &item.summary,
                    &item.source,
                    item.graded_at.as_deref(),
                ) {
                    Ok(_) => {
                        if is_update {
                            report.entries_updated += 1;
                        } else {
                            report.entries_added += 1;
                        }
                    }
                    Err(e) => {
                        report
                            .errors
                            .push(format!("Error saving policy for {}: {}", item.domain, e));
                    }
                }
            }
        }

        let now = chrono::Utc::now();
        let _ = self.cache.set_last_sync_time(now);
        report.last_sync_time = Some(now.to_rfc3339());
        if let Ok(report_json) = serde_json::to_string(&report) {
            let _ = self.cache.set_metadata("last_sync_report", &report_json);
        }

        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grade::UserRights;

    #[test]
    fn test_parse_shard_index() {
        let json_str = r#"{ "shards": ["shard-000.json", "shard-001.json"] }"#;
        let index: ShardIndex = serde_json::from_str(json_str).unwrap();
        assert_eq!(index.shards.len(), 2);
        assert_eq!(index.shards[0], "shard-000.json");
        assert_eq!(index.shards[1], "shard-001.json");
    }

    #[test]
    fn test_parse_shard_payload_v1() {
        let json_str = r#"{
            "version": 1,
            "updated_at": "2026-09-04T12:00:00Z",
            "shards": [
                {
                    "domain": "example.com",
                    "policy_url": "https://example.com/privacy",
                    "policy_hash": "6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b",
                    "grade": "A",
                    "summary": {
                        "data_collected": ["email"],
                        "data_used_for": ["login"],
                        "shared_with_third_parties": false,
                        "third_party_details": "None",
                        "retention_period": "30 days",
                        "user_rights": {
                            "can_delete_data": true,
                            "can_export_data": true,
                            "can_opt_out_of_tracking": true
                        },
                        "tracking_and_ads": "None",
                        "arbitration_or_class_action_waiver": false,
                        "policy_clarity_notes": "Clear"
                    },
                    "source": "cache",
                    "graded_at": "2026-09-04T12:00:00Z"
                }
            ]
        }"#;

        let payload: ShardPayload = serde_json::from_str(json_str).unwrap();
        assert_eq!(payload.version, 1);
        assert_eq!(payload.shards.len(), 1);
        assert_eq!(payload.shards[0].domain, "example.com");
        assert_eq!(payload.shards[0].grade, Grade::A);
        assert_eq!(payload.shards[0].source, "cache");
    }

    #[test]
    fn test_parse_shard_payload_with_policies_alias() {
        let json_str = r#"{
            "version": "1.0.0",
            "updated_at": "2026-09-04T12:00:00Z",
            "policies": [
                {
                    "domain": "duckduckgo.com",
                    "policy_url": "https://duckduckgo.com/privacy",
                    "policy_version_hash": "hash123",
                    "grade": "A",
                    "summary": {
                        "data_collected": [],
                        "data_used_for": [],
                        "shared_with_third_parties": false,
                        "third_party_details": "None",
                        "retention_period": "None",
                        "user_rights": {
                            "can_delete_data": true,
                            "can_export_data": true,
                            "can_opt_out_of_tracking": true
                        },
                        "tracking_and_ads": "None",
                        "arbitration_or_class_action_waiver": false,
                        "policy_clarity_notes": "Exemplary"
                    },
                    "source": "curated",
                    "graded_at": "2026-09-04T12:00:00Z"
                }
            ]
        }"#;

        let payload: ShardPayload = serde_json::from_str(json_str).unwrap();
        assert_eq!(payload.version, 1);
        assert_eq!(payload.shards.len(), 1);
        assert_eq!(payload.shards[0].domain, "duckduckgo.com");
        assert_eq!(payload.shards[0].policy_hash, Some("hash123".into()));
    }

    #[test]
    fn test_apply_shard_payload_to_cache() {
        let cache = Arc::new(CacheManager::in_memory().unwrap());

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

        let entry = ShardEntry {
            domain: "duckduckgo.com".into(),
            policy_url: "https://duckduckgo.com/privacy".into(),
            policy_hash: Some("ddg_hash".into()),
            grade: Grade::A,
            summary,
            source: "cache".into(),
            graded_at: Some("2026-09-04T12:00:00Z".into()),
        };

        let is_update = cache.get_grade(&entry.domain, &entry.policy_url).unwrap().is_some();
        assert!(!is_update);

        cache
            .upsert_cache_entry(
                &entry.domain,
                &entry.policy_url,
                entry.policy_hash.as_deref(),
                &entry.grade.to_string(),
                &entry.summary,
                &entry.source,
                entry.graded_at.as_deref(),
            )
            .unwrap();

        let fetched = cache.get_grade("duckduckgo.com", "https://duckduckgo.com/privacy").unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().grade, "A");

        // Verify that second insertion is identified as an update
        let is_update2 = cache.get_grade(&entry.domain, &entry.policy_url).unwrap().is_some();
        assert!(is_update2);
    }

    #[tokio::test]
    async fn test_sync_all_e2e() {
        use std::io::Cursor;
        use std::sync::atomic::{AtomicBool, Ordering};

        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let port = server.server_addr().to_ip().unwrap().port();
        let base_url = format!("http://127.0.0.1:{}", port);

        let running = Arc::new(AtomicBool::new(true));
        let running_clone = Arc::clone(&running);

        let server_thread = std::thread::spawn(move || {
            let index_json = r#"{"shards":["shard-000.json"]}"#;
            let shard_json = r#"{
                "version": 1,
                "updated_at": "2026-09-04T12:00:00Z",
                "shards": [
                    {
                        "domain": "duckduckgo.com",
                        "policy_url": "https://duckduckgo.com/privacy",
                        "policy_hash": "hash-ddg",
                        "grade": "A",
                        "summary": {
                            "data_collected": [],
                            "data_used_for": [],
                            "shared_with_third_parties": false,
                            "third_party_details": "None",
                            "retention_period": "None",
                            "user_rights": {
                                "can_delete_data": true,
                                "can_export_data": true,
                                "can_opt_out_of_tracking": true
                            },
                            "tracking_and_ads": "None",
                            "arbitration_or_class_action_waiver": false,
                            "policy_clarity_notes": "Great"
                        },
                        "source": "cache",
                        "graded_at": "2026-09-04T12:00:00Z"
                    }
                ]
            }"#;

            while running_clone.load(Ordering::Relaxed) {
                if let Ok(Some(rq)) = server.recv_timeout(std::time::Duration::from_millis(50)) {
                    let url = rq.url().to_string();
                    if url.contains("index.json") {
                        let resp = tiny_http::Response::new(
                            tiny_http::StatusCode(200),
                            vec![tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap()],
                            Cursor::new(index_json.as_bytes().to_vec()),
                            Some(index_json.len()),
                            None,
                        );
                        let _ = rq.respond(resp);
                    } else if url.contains("shard-000.json") {
                        let resp = tiny_http::Response::new(
                            tiny_http::StatusCode(200),
                            vec![tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap()],
                            Cursor::new(shard_json.as_bytes().to_vec()),
                            Some(shard_json.len()),
                            None,
                        );
                        let _ = rq.respond(resp);
                    }
                }
            }
        });

        let cache = Arc::new(CacheManager::in_memory().unwrap());
        let syncer = CacheSync::with_client(base_url, reqwest::Client::new(), Arc::clone(&cache));

        assert!(syncer.is_reachable().await);

        let report = syncer.sync_all().await.unwrap();
        assert_eq!(report.shards_fetched, 1);
        assert_eq!(report.entries_added, 1);
        assert_eq!(report.entries_updated, 0);
        assert!(report.errors.is_empty());

        let record = cache.get_grade("duckduckgo.com", "https://duckduckgo.com/privacy").unwrap();
        assert!(record.is_some());
        assert_eq!(record.unwrap().source, "cache");

        assert!(cache.get_last_sync_time().unwrap().is_some());
        assert!(cache.get_metadata("last_sync_report").unwrap().is_some());

        running.store(false, Ordering::Relaxed);
        server_thread.join().unwrap();
    }
}
