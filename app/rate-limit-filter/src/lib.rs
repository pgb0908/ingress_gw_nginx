use proxy_wasm::traits::{Context, HttpContext, RootContext};
use proxy_wasm::types::{Action, ContextType, LogLevel};

proxy_wasm::main! {{
    proxy_wasm::set_log_level(LogLevel::Info);
    proxy_wasm::set_root_context(|_| -> Box<dyn RootContext> {
        Box::new(RateLimitRoot { rules: Vec::new(), default_requests: 60, default_window_secs: 60 })
    });
}}

struct RateLimitRule {
    key: String,
    requests: u64,
    window_secs: u64,
}

struct RateLimitRoot {
    rules: Vec<RateLimitRule>,
    default_requests: u64,
    default_window_secs: u64,
}

impl Context for RateLimitRoot {}

impl RootContext for RateLimitRoot {
    fn on_configure(&mut self, _config_size: usize) -> bool {
        if let Some(bytes) = self.get_plugin_configuration() {
            if let Ok(s) = core::str::from_utf8(&bytes) {
                parse_config(s, &mut self.rules, &mut self.default_requests, &mut self.default_window_secs);
            }
        }
        true
    }

    fn get_type(&self) -> Option<ContextType> {
        Some(ContextType::HttpContext)
    }

    fn create_http_context(&self, _context_id: u32) -> Option<Box<dyn HttpContext>> {
        Some(Box::new(RateLimitFilter {
            rules: self.rules.iter().map(|r| RateLimitRule {
                key: r.key.clone(),
                requests: r.requests,
                window_secs: r.window_secs,
            }).collect(),
            default_requests: self.default_requests,
            default_window_secs: self.default_window_secs,
        }))
    }
}

struct RateLimitFilter {
    rules: Vec<RateLimitRule>,
    default_requests: u64,
    default_window_secs: u64,
}

impl Context for RateLimitFilter {}

impl HttpContext for RateLimitFilter {
    fn on_http_request_headers(&mut self, _num_headers: usize, _end_of_stream: bool) -> Action {
        let tenant = self.get_http_request_header("x-tenant-id").unwrap_or_default();
        let service = self.get_http_request_header("x-service-id").unwrap_or_default();
        let route = self.get_http_request_header("x-route-id").unwrap_or_default();

        if tenant.is_empty() {
            // tenant-filter should have caught this; fail-open
            return Action::Continue;
        }

        let (limit, window_secs) = self.find_rule(&tenant, &service, &route);
        let bucket_key = format!("rl:{}:{}:{}", tenant, service, route);
        let now_secs = {
            use std::time::UNIX_EPOCH;
            self.get_current_time()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        };

        match self.check_and_increment(&bucket_key, limit, window_secs, now_secs) {
            Ok((allowed, remaining)) => {
                if allowed {
                    self.set_http_request_header(
                        "x-gateway-limit-remaining",
                        Some(&remaining.to_string()),
                    );
                    Action::Continue
                } else {
                    self.set_http_request_header("x-gateway-decision", Some("deny"));
                    self.set_http_request_header("x-gateway-decision-reason", Some("rate_limited"));
                    self.send_http_response(
                        429,
                        vec![
                            ("content-type", "application/json"),
                            ("x-gateway-decision", "deny"),
                            ("x-gateway-decision-reason", "rate_limited"),
                        ],
                        Some(
                            br#"{"error":{"code":"rate_limited","message":"rate limit exceeded"}}"#,
                        ),
                    );
                    Action::Pause
                }
            }
            // Shared data error → fail-open (configurable; here choosing open)
            Err(_) => Action::Continue,
        }
    }
}

impl RateLimitFilter {
    fn find_rule(&self, tenant: &str, service: &str, route: &str) -> (u64, u64) {
        let full_key = format!("{}:{}:{}", tenant, service, route);
        let svc_key = format!("{}:{}", tenant, service);

        for rule in &self.rules {
            if rule.key == full_key || rule.key == svc_key || rule.key == tenant {
                return (rule.requests, rule.window_secs);
            }
        }
        (self.default_requests, self.default_window_secs)
    }

    // Returns (allowed, remaining) using shared data CAS for atomic update
    fn check_and_increment(
        &self,
        key: &str,
        limit: u64,
        window_secs: u64,
        now_secs: u64,
    ) -> Result<(bool, u64), ()> {
        for _ in 0..5 {
            let (raw, cas) = self.get_shared_data(key);
            let (window_start, count) = decode_bucket(raw.as_deref());

            let (new_start, new_count) = if now_secs.saturating_sub(window_start) >= window_secs {
                (now_secs, 0u64)
            } else {
                (window_start, count)
            };

            let allowed = new_count < limit;
            let updated_count = if allowed { new_count + 1 } else { new_count };
            let remaining = limit.saturating_sub(updated_count);
            let encoded = encode_bucket(new_start, updated_count);

            match self.set_shared_data(key, Some(&encoded), cas) {
                Ok(()) => return Ok((allowed, remaining)),
                Err(_) => continue, // CAS conflict; retry
            }
        }
        Err(())
    }
}

// Bucket encoding: 8 bytes window_start (u64 LE) + 8 bytes count (u64 LE)
fn encode_bucket(window_start: u64, count: u64) -> Vec<u8> {
    let mut v = Vec::with_capacity(16);
    v.extend_from_slice(&window_start.to_le_bytes());
    v.extend_from_slice(&count.to_le_bytes());
    v
}

fn decode_bucket(raw: Option<&[u8]>) -> (u64, u64) {
    match raw {
        Some(b) if b.len() >= 16 => {
            let ws = u64::from_le_bytes(b[0..8].try_into().unwrap_or([0u8; 8]));
            let cnt = u64::from_le_bytes(b[8..16].try_into().unwrap_or([0u8; 8]));
            (ws, cnt)
        }
        _ => (0, 0),
    }
}

// Minimal config parser for:
// {"default":{"requests":N,"window_seconds":N},"limits":{"key":{"requests":N,"window_seconds":N},...}}
fn parse_config(
    s: &str,
    rules: &mut Vec<RateLimitRule>,
    default_requests: &mut u64,
    default_window_secs: &mut u64,
) {
    if let Some(dr) = extract_u64(s, "\"default\"", "requests") {
        *default_requests = dr;
    }
    if let Some(dw) = extract_u64(s, "\"default\"", "window_seconds") {
        *default_window_secs = dw;
    }

    // Parse "limits" object entries
    let limits_start = match s.find("\"limits\"") {
        Some(p) => p,
        None => return,
    };
    let after = &s[limits_start + 8..];
    let obj_start = match after.find('{') {
        Some(p) => p,
        None => return,
    };
    let obj = &after[obj_start + 1..];

    let mut rest = obj;
    while let Some(k_pos) = rest.find('"') {
        rest = &rest[k_pos + 1..];
        let k_end = match rest.find('"') {
            Some(p) => p,
            None => break,
        };
        let rule_key = rest[..k_end].to_string();
        rest = &rest[k_end + 1..];

        if rule_key == "}" { break; }

        let colon = match rest.find(':') {
            Some(p) => p,
            None => break,
        };
        let val_part = &rest[colon + 1..].trim_start();

        let reqs = extract_u64_inline(val_part, "requests").unwrap_or(*default_requests);
        let win = extract_u64_inline(val_part, "window_seconds").unwrap_or(*default_window_secs);
        rules.push(RateLimitRule { key: rule_key, requests: reqs, window_secs: win });

        // Advance past the value object
        if let Some(close) = val_part.find('}') {
            rest = &val_part[close + 1..];
        } else {
            break;
        }
    }
}

fn extract_u64(s: &str, section: &str, field: &str) -> Option<u64> {
    let pos = s.find(section)?;
    let section_str = &s[pos..];
    extract_u64_inline(section_str, field)
}

fn extract_u64_inline(s: &str, field: &str) -> Option<u64> {
    let needle = format!("\"{}\"", field);
    let pos = s.find(&needle)?;
    let after = &s[pos + needle.len()..];
    let colon = after.find(':')?;
    let val = after[colon + 1..].trim_start();
    let end = val.find(|c: char| !c.is_ascii_digit()).unwrap_or(val.len());
    val[..end].parse().ok()
}
