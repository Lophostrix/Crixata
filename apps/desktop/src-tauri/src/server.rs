use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::thread;
use anyhow::Result;
use reqwest::Client;
use tiny_http::{Header, Method, Response, Server, StatusCode};
use tokio::sync::Mutex as AsyncMutex;

use crate::cache::CacheManager;
use crate::grade::{compute_grade, GradeRequest, GradeResponse};
use crate::llm::extract_policy_summary;
use crate::model::ModelManager;
use crate::sidecar::SidecarManager;

pub struct LocalServerHandle {
    pub port: u16,
}

pub fn start_local_server(
    port: u16,
    llama_server_port: u16,
    cache: Arc<CacheManager>,
    sidecar: Arc<AsyncMutex<SidecarManager>>,
    model_manager: Arc<ModelManager>,
) -> Result<LocalServerHandle> {
    let addr = format!("127.0.0.1:{}", port);
    let server = Server::http(&addr)
        .map_err(|e| anyhow::anyhow!("Failed to bind local server to {}: {}", addr, e))?;

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to create tokio runtime for local server: {}", e))?;

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .unwrap_or_default();

    thread::spawn(move || {
        for mut request in server.incoming_requests() {
            let method = request.method().clone();
            let url_path = request.url().to_string();

            let cors_headers = build_cors_headers(&request);

            // Handle CORS preflight
            if method == Method::Options {
                let mut resp = Response::empty(StatusCode(204));
                for h in cors_headers {
                    resp.add_header(h);
                }
                let _ = request.respond(resp);
                continue;
            }

            // GET /health
            if method == Method::Get && url_path.starts_with("/health") {
                let sidecar_ready = sidecar.try_lock().map(|sc| sc.is_ready()).unwrap_or(false);
                let model_downloaded = model_manager.is_model_present();

                let body = serde_json::json!({
                    "status": "ok",
                    "sidecar_ready": sidecar_ready,
                    "model_downloaded": model_downloaded,
                })
                .to_string();

                let mut resp = Response::from_string(body);
                resp.add_header(Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap());
                for h in cors_headers {
                    resp.add_header(h);
                }
                let _ = request.respond(resp);
                continue;
            }

            // POST /grade
            if method == Method::Post && url_path.starts_with("/grade") {
                let mut body_str = String::new();
                if let Err(e) = request.as_reader().read_to_string(&mut body_str) {
                    let err_body = serde_json::json!({ "error": format!("Failed to read request body: {}", e) }).to_string();
                    let mut resp = Response::from_string(err_body).with_status_code(StatusCode(400));
                    resp.add_header(Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap());
                    for h in cors_headers {
                        resp.add_header(h);
                    }
                    let _ = request.respond(resp);
                    continue;
                }

                let grade_req: Result<GradeRequest, _> = serde_json::from_str(&body_str);
                let req = match grade_req {
                    Ok(r) => r,
                    Err(e) => {
                        let err_body = serde_json::json!({ "error": format!("Invalid JSON request: {}", e) }).to_string();
                        let mut resp = Response::from_string(err_body).with_status_code(StatusCode(400));
                        resp.add_header(Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap());
                        for h in cors_headers {
                            resp.add_header(h);
                        }
                        let _ = request.respond(resp);
                        continue;
                    }
                };

                // 1. Validate url and text are non-empty
                if req.url.trim().is_empty() || req.text.trim().is_empty() {
                    let err_body = serde_json::json!({ "error": "Both 'url' and 'text' must be non-empty." }).to_string();
                    let mut resp = Response::from_string(err_body).with_status_code(StatusCode(400));
                    resp.add_header(Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap());
                    for h in cors_headers {
                        resp.add_header(h);
                    }
                    let _ = request.respond(resp);
                    continue;
                }

                // 2. Derive domain from URL
                let domain = extract_domain(&req.url);

                // 3. Check cache; return cached result if found
                let cached_record = cache.get_grade(&domain, &req.url).unwrap_or(None);
                if let Some(record) = cached_record {
                    let grade_char = record.grade.chars().next().unwrap_or('D');
                    let grade_resp = GradeResponse {
                        grade: grade_char,
                        summary: record.summary,
                        cached: true,
                        source: record.source,
                    };

                    let body = serde_json::to_string(&grade_resp).unwrap_or_default();
                    let mut resp = Response::from_string(body);
                    resp.add_header(Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap());
                    for h in cors_headers {
                        resp.add_header(h);
                    }
                    let _ = request.respond(resp);
                    continue;
                }

                // 4. If not cached and sidecar is ready, call the LLM extraction function
                let is_ready = sidecar.try_lock().map(|sc| sc.is_ready()).unwrap_or(false);
                if !is_ready {
                    let err_body = serde_json::json!({
                        "error": "Inference sidecar is not ready. Please download the model and verify the engine is running."
                    }).to_string();

                    let mut resp = Response::from_string(err_body).with_status_code(StatusCode(503));
                    resp.add_header(Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap());
                    for h in cors_headers {
                        resp.add_header(h);
                    }
                    let _ = request.respond(resp);
                    continue;
                }

                let extraction_res = rt.block_on(extract_policy_summary(
                    &req.text,
                    &client,
                    llama_server_port,
                ));

                match extraction_res {
                    Ok(summary) => {
                        // 5. Compute grade from extracted summary
                        let grade_char = compute_grade(&summary);

                        // 6. Store in cache and return
                        let policy_hash = format!("{:016x}", compute_hash(&req.text));
                        let _ = cache.upsert_grade(
                            &domain,
                            &req.url,
                            Some(&policy_hash),
                            &grade_char.to_string(),
                            &summary,
                            "llm",
                        );

                        let grade_resp = GradeResponse {
                            grade: grade_char,
                            summary,
                            cached: false,
                            source: "llm".into(),
                        };

                        let body = serde_json::to_string(&grade_resp).unwrap_or_default();
                        let mut resp = Response::from_string(body);
                        resp.add_header(Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap());
                        for h in cors_headers {
                            resp.add_header(h);
                        }
                        let _ = request.respond(resp);
                    }
                    Err(e) => {
                        let err_body = serde_json::json!({
                            "error": format!("LLM extraction failed: {}", e)
                        }).to_string();

                        let mut resp = Response::from_string(err_body).with_status_code(StatusCode(500));
                        resp.add_header(Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap());
                        for h in cors_headers {
                            resp.add_header(h);
                        }
                        let _ = request.respond(resp);
                    }
                }

                continue;
            }

            // 404 for any other route
            let mut not_found = Response::from_string("Not Found").with_status_code(StatusCode(404));
            for h in cors_headers {
                not_found.add_header(h);
            }
            let _ = request.respond(not_found);
        }
    });

    Ok(LocalServerHandle { port })
}

fn build_cors_headers(request: &tiny_http::Request) -> Vec<Header> {
    let mut origin_val = "*".to_string();
    for header in request.headers() {
        if header.field.equiv("Origin") {
            let val = header.value.as_str();
            if val.starts_with("chrome-extension://")
                || val.starts_with("http://localhost")
                || val.starts_with("http://127.0.0.1")
            {
                origin_val = val.to_string();
                break;
            }
        }
    }

    vec![
        Header::from_bytes(&b"Access-Control-Allow-Origin"[..], origin_val.as_bytes()).unwrap(),
        Header::from_bytes(&b"Access-Control-Allow-Methods"[..], &b"GET, POST, OPTIONS"[..]).unwrap(),
        Header::from_bytes(&b"Access-Control-Allow-Headers"[..], &b"Content-Type, Authorization"[..]).unwrap(),
    ]
}

pub fn extract_domain(url_str: &str) -> String {
    if let Ok(parsed) = url::Url::parse(url_str) {
        if let Some(host) = parsed.host_str() {
            return host.to_string();
        }
    }

    let stripped = url_str
        .trim_start_matches("https://")
        .trim_start_matches("http://");

    stripped
        .split('/')
        .next()
        .unwrap_or("unknown")
        .split(':')
        .next()
        .unwrap_or("unknown")
        .to_string()
}

fn compute_hash(input: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_domain() {
        assert_eq!(extract_domain("https://example.com/privacy"), "example.com");
        assert_eq!(extract_domain("http://github.com/tos"), "github.com");
        assert_eq!(extract_domain("sub.domain.org/terms"), "sub.domain.org");
    }
}
