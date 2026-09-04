use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::cache::CacheManager;
use crate::grade::{Grade, PolicySummary};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardPolicyEntry {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardPayload {
    pub version: String,
    pub shard_id: String,
    pub updated_at: String,
    pub policies: Vec<ShardPolicyEntry>,
}

pub struct CacheSyncer {
    client: Client,
    cdn_base_url: String,
}

impl CacheSyncer {
    pub fn new(cdn_base_url: String) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .unwrap_or_default(),
            cdn_base_url,
        }
    }

    pub async fn fetch_and_apply_shard(
        &self,
        shard_id: &str,
        cache: &CacheManager,
    ) -> Result<usize> {
        let url = format!("{}/{}.json", self.cdn_base_url, shard_id);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .with_context(|| format!("Failed to fetch cache shard from {}", url))?;

        if !resp.status().is_success() {
            anyhow::bail!("CDN returned error status {}", resp.status());
        }

        let shard: ShardPayload = resp
            .json()
            .await
            .with_context(|| "Failed to parse cache shard JSON")?;

        let mut inserted = 0;
        for item in shard.policies {
            if cache
                .upsert_grade(
                    &item.domain,
                    &item.policy_url,
                    Some(&item.policy_version_hash),
                    &item.grade.to_string(),
                    &item.summary,
                    &item.source,
                )
                .is_ok()
            {
                inserted += 1;
            }
        }

        Ok(inserted)
    }
}
