use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::json;

use crate::grade::Summary;

pub const GBNF_GRAMMAR: &str = include_str!("../resources/policy_schema.gbnf");

/// Formats the prompt for the Llama 3.2 Instruct model.
pub fn create_analysis_prompt(policy_text: &str) -> String {
    // Truncate policy text to ~12000 characters to fit comfortably inside a 4096 context window
    let max_chars = 12000;
    let truncated = if policy_text.len() > max_chars {
        &policy_text[..max_chars]
    } else {
        policy_text
    };

    format!(
        "<|begin_of_text|><|start_header_id|>system<|end_header_id|>\n\n\
You are an expert privacy policy analyst. Analyze the policy text provided by the user and extract key terms strictly into the specified JSON format.\n\
Key fields required:\n\
- data_collected (list of strings)\n\
- data_used_for (list of strings)\n\
- shared_with_third_parties (boolean)\n\
- third_party_details (string)\n\
- retention_period (string)\n\
- user_rights: can_delete_data (boolean), can_export_data (boolean), can_opt_out_of_tracking (boolean)\n\
- tracking_and_ads (string)\n\
- arbitration_or_class_action_waiver (boolean)\n\
- policy_clarity_notes (string)\n\n\
Output ONLY valid JSON adhering to the schema.<|eot_id|>\n\
<|start_header_id|>user<|end_header_id|>\n\n\
Policy text:\n{}\n<|eot_id|>\n\
<|start_header_id|>assistant<|end_header_id|>\n\n",
        truncated
    )
}

/// Calls local llama-server /completion endpoint using GBNF grammar constrained generation
pub async fn extract_policy_summary(
    text: &str,
    client: &Client,
    port: u16,
) -> Result<Summary> {
    let health_url = format!("http://127.0.0.1:{}/health", port);
    let health_resp = client
        .get(&health_url)
        .timeout(std::time::Duration::from_secs(3))
        .send()
        .await;

    match health_resp {
        Ok(resp) if resp.status().is_success() => (),
        _ => {
            anyhow::bail!(
                "Local inference sidecar (llama-server) is not ready on port {}. Please verify the model and engine are running.",
                port
            );
        }
    }

    let prompt = create_analysis_prompt(text);
    let completion_url = format!("http://127.0.0.1:{}/completion", port);

    let payload = json!({
        "prompt": prompt,
        "grammar": GBNF_GRAMMAR,
        "temperature": 0.1,
        "n_predict": 1024,
        "stream": false,
        "stop": ["<|eot_id|>", "<|end_of_text|>"]
    });

    let resp = client
        .post(&completion_url)
        .json(&payload)
        .timeout(std::time::Duration::from_secs(120))
        .send()
        .await
        .with_context(|| format!("Failed to send request to llama-server at {}", completion_url))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let err_text = resp.text().await.unwrap_or_default();
        anyhow::bail!("llama-server returned error {}: {}", status, err_text);
    }

    let body: serde_json::Value = resp
        .json()
        .await
        .with_context(|| "Failed to parse JSON response from llama-server")?;

    let content = body
        .get("content")
        .and_then(|c| c.as_str())
        .or_else(|| {
            body.get("choices")
                .and_then(|ch| ch.get(0))
                .and_then(|c0| c0.get("text"))
                .and_then(|t| t.as_str())
        })
        .with_context(|| format!("Unexpected response structure from llama-server: {}", body))?;

    let summary: Summary = serde_json::from_str(content.trim())
        .with_context(|| format!("Failed to parse PolicySummary from completion output:\n{}", content))?;

    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grammar_is_non_empty() {
        assert!(!GBNF_GRAMMAR.is_empty());
        assert!(GBNF_GRAMMAR.contains("PolicySummary"));
        assert!(GBNF_GRAMMAR.contains("user-rights"));
    }

    #[test]
    fn test_prompt_formatting() {
        let prompt = create_analysis_prompt("Sample policy content.");
        assert!(prompt.contains("Sample policy content."));
        assert!(prompt.contains("<|start_header_id|>system<|end_header_id|>"));
    }
}
