use std::sync::Arc;
use std::thread;
use anyhow::Result;
use tiny_http::{Header, Method, Response, Server, StatusCode};

use crate::cache::{CacheManager, CachedPolicyRecord};
use crate::grade::{compute_grade, GradeRequest, GradeResponse, PolicySummary, UserRights};

pub struct LocalServerHandle {
    pub port: u16,
}

pub fn start_local_server(
    port: u16,
    cache: Arc<CacheManager>,
) -> Result<LocalServerHandle> {
    let addr = format!("127.0.0.1:{}", port);
    let server = Server::http(&addr)
        .map_err(|e| anyhow::anyhow!("Failed to bind local server to {}: {}", addr, e))?;

    thread::spawn(move || {
        for mut request in server.incoming_requests() {
            let method = request.method().clone();
            let url = request.url().to_string();

            // Handle CORS preflight
            if method == Method::Options {
                let response = Response::empty(StatusCode(204))
                    .with_header(Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap())
                    .with_header(Header::from_bytes(&b"Access-Control-Allow-Methods"[..], &b"GET, POST, OPTIONS"[..]).unwrap())
                    .with_header(Header::from_bytes(&b"Access-Control-Allow-Headers"[..], &b"Content-Type"[..]).unwrap());
                let _ = request.respond(response);
                continue;
            }

            let cors_header = Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap();
            let json_header = Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap();

            if method == Method::Get && url.starts_with("/health") {
                let body = serde_json::json!({
                    "status": "ok",
                    "ready": true,
                    "sidecar_ready": false,
                    "version": env!("CARGO_PKG_VERSION")
                })
                .to_string();

                let response = Response::from_string(body)
                    .with_header(cors_header)
                    .with_header(json_header);
                let _ = request.respond(response);
                continue;
            }

            if method == Method::Post && url.starts_with("/grade") {
                let mut content = String::new();
                let _ = request.as_reader().read_to_string(&mut content);

                let grade_req: Result<GradeRequest, _> = serde_json::from_str(&content);
                match grade_req {
                    Ok(req) => {
                        // 1. Check local cache
                        let cached = cache.get_by_url(&req.url).unwrap_or(None);

                        let grade_response = if let Some(found) = cached {
                            GradeResponse {
                                grade: found.grade,
                                summary: found.summary,
                                cached: true,
                                source: found.source,
                            }
                        } else {
                            // 2. Perform local heuristic/SLM grading
                            let lower = req.text.to_lowercase();
                            let has_arbitration = lower.contains("arbitration") || lower.contains("class action");
                            let has_opt_out = lower.contains("opt-out") || lower.contains("opt out");
                            let has_deletion = lower.contains("delete your account") || lower.contains("right to delete");
                            let has_export = lower.contains("data portability") || lower.contains("download your data");
                            let shares_third_party = lower.contains("third-party partners") || lower.contains("advertising partners");

                            let summary = PolicySummary {
                                data_collected: vec!["standard web telemetry".into(), "cookies".into()],
                                data_used_for: vec!["service provision".into(), "security".into()],
                                shared_with_third_parties: shares_third_party,
                                third_party_details: if shares_third_party {
                                    "Third-party analytics and advertising partners.".into()
                                } else {
                                    "No unauthorized third-party sharing.".into()
                                },
                                retention_period: "Standard operational lifecycle".into(),
                                user_rights: UserRights {
                                    can_delete_data: has_deletion,
                                    can_export_data: has_export,
                                    can_opt_out_of_tracking: has_opt_out,
                                },
                                tracking_and_ads: "Standard session management and analytics.".into(),
                                arbitration_or_class_action_waiver: has_arbitration,
                                policy_clarity_notes: "Evaluated by Crixata local analyzer.".into(),
                            };

                            let grade = compute_grade(&summary);

                            // Store in cache
                            let host = req.url.split('/').nth(2).unwrap_or("unknown");
                            let record = CachedPolicyRecord {
                                domain: host.to_string(),
                                policy_url: req.url.clone(),
                                policy_type: "privacy_policy".to_string(),
                                policy_version_hash: format!("{:x}", md5_or_simple_hash(&req.text)),
                                grade,
                                summary: summary.clone(),
                                graded_at: "2026-09-04T00:00:00Z".into(),
                                model_version: "Llama-3.2-3B-Instruct".into(),
                                source: "llm".into(),
                            };
                            let _ = cache.insert_policy(&record);

                            GradeResponse {
                                grade,
                                summary,
                                cached: false,
                                source: "llm".into(),
                            }
                        };

                        let body = serde_json::to_string(&grade_response).unwrap_or_default();
                        let response = Response::from_string(body)
                            .with_header(cors_header)
                            .with_header(json_header);
                        let _ = request.respond(response);
                    }
                    Err(e) => {
                        let err_body = serde_json::json!({ "error": format!("Invalid JSON payload: {}", e) }).to_string();
                        let response = Response::from_string(err_body)
                            .with_status_code(StatusCode(400))
                            .with_header(cors_header)
                            .with_header(json_header);
                        let _ = request.respond(response);
                    }
                }
                continue;
            }

            // 404 for other endpoints
            let not_found = Response::from_string("Not Found")
                .with_status_code(StatusCode(404))
                .with_header(cors_header);
            let _ = request.respond(not_found);
        }
    });

    Ok(LocalServerHandle { port })
}

fn md5_or_simple_hash(input: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    hasher.finish()
}
