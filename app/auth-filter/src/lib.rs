// Pure parsing logic — compiled on all targets including native test builds
pub fn parse_api_keys(s: &str) -> Vec<(String, String)> {
    let mut result = Vec::new();
    let s = s.trim();

    let inner = match s.find("\"api_keys\"") {
        Some(pos) => &s[pos + 10..],
        None => return result,
    };
    let inner = match inner.find('{') {
        Some(pos) => &inner[pos + 1..],
        None => return result,
    };
    let inner = match inner.rfind('}') {
        Some(pos) => &inner[..pos],
        None => inner,
    };

    let mut rest = inner;
    while let Some(k_start) = rest.find('"') {
        rest = &rest[k_start + 1..];
        let k_end = match rest.find('"') {
            Some(p) => p,
            None => break,
        };
        let key = rest[..k_end].to_string();
        rest = &rest[k_end + 1..];

        let colon = match rest.find(':') {
            Some(p) => p,
            None => break,
        };
        rest = rest[colon + 1..].trim_start();
        let v_start = match rest.find('"') {
            Some(p) => p,
            None => break,
        };
        rest = &rest[v_start + 1..];
        let v_end = match rest.find('"') {
            Some(p) => p,
            None => break,
        };
        let val = rest[..v_end].to_string();
        rest = &rest[v_end + 1..];
        result.push((key, val));
    }
    result
}

// Wasm filter implementation — only compiled for wasm32 target
#[cfg(target_arch = "wasm32")]
mod wasm_filter {
    use super::parse_api_keys;
    use proxy_wasm::traits::{Context, HttpContext, RootContext};
    use proxy_wasm::types::{Action, ContextType, LogLevel};

    proxy_wasm::main! {{
        proxy_wasm::set_log_level(LogLevel::Info);
        proxy_wasm::set_root_context(|_| -> Box<dyn RootContext> {
            Box::new(AuthFilterRoot { api_keys: Vec::new() })
        });
    }}

    struct AuthFilterRoot {
        api_keys: Vec<(String, String)>,
    }

    impl Context for AuthFilterRoot {}

    impl RootContext for AuthFilterRoot {
        fn on_configure(&mut self, _config_size: usize) -> bool {
            if let Some(bytes) = self.get_plugin_configuration() {
                if let Ok(s) = core::str::from_utf8(&bytes) {
                    self.api_keys = parse_api_keys(s);
                }
            }
            true
        }

        fn get_type(&self) -> Option<ContextType> {
            Some(ContextType::HttpContext)
        }

        fn create_http_context(&self, _context_id: u32) -> Option<Box<dyn HttpContext>> {
            Some(Box::new(AuthFilter { api_keys: self.api_keys.clone() }))
        }
    }

    struct AuthFilter {
        api_keys: Vec<(String, String)>,
    }

    impl Context for AuthFilter {}

    impl HttpContext for AuthFilter {
        fn on_http_request_headers(&mut self, _num_headers: usize, _end_of_stream: bool) -> Action {
            let tenant_id = self.get_http_request_header("x-tenant-id").unwrap_or_default();
            let api_key = self.get_http_request_header("x-api-key").unwrap_or_default();

            let expected = self
                .api_keys
                .iter()
                .find(|(k, _)| k == &tenant_id)
                .map(|(_, v)| v.as_str());

            if let Some(expected_key) = expected {
                if api_key != expected_key {
                    self.send_http_response(
                        401,
                        vec![("content-type", "application/json")],
                        Some(br#"{"error":{"code":"unauthorized","message":"missing or invalid api key"}}"#),
                    );
                    return Action::Pause;
                }
            }

            self.set_http_request_header("x-auth-subject", Some(&tenant_id));
            self.set_http_request_header("x-auth-method", Some("x-api-key"));
            Action::Continue
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty_string_returns_empty() {
        assert!(parse_api_keys("").is_empty());
    }

    #[test]
    fn parse_no_api_keys_section_returns_empty() {
        assert!(parse_api_keys(r#"{"other":"value"}"#).is_empty());
    }

    #[test]
    fn parse_single_key_value() {
        let keys = parse_api_keys(r#"{"api_keys":{"tenant-a":"secret-a"}}"#);
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0], ("tenant-a".to_string(), "secret-a".to_string()));
    }

    #[test]
    fn parse_multiple_key_values() {
        let keys = parse_api_keys(
            r#"{"api_keys":{"tenant-a":"key-a","tenant-b":"key-b","tenant-c":"key-c"}}"#,
        );
        assert_eq!(keys.len(), 3);
        assert!(keys.contains(&("tenant-a".to_string(), "key-a".to_string())));
        assert!(keys.contains(&("tenant-b".to_string(), "key-b".to_string())));
        assert!(keys.contains(&("tenant-c".to_string(), "key-c".to_string())));
    }

    #[test]
    fn parse_empty_api_keys_object_returns_empty() {
        assert!(parse_api_keys(r#"{"api_keys":{}}"#).is_empty());
    }

    #[test]
    fn parse_with_whitespace_around_values() {
        let keys = parse_api_keys(r#"{ "api_keys" : { "t1" : "k1" } }"#);
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].0, "t1");
        assert_eq!(keys[0].1, "k1");
    }

    #[test]
    fn parse_key_lookup_logic() {
        let keys = parse_api_keys(r#"{"api_keys":{"tenant-a":"correct-key"}}"#);
        let found = keys.iter().find(|(k, _)| k == "tenant-a").map(|(_, v)| v.as_str());
        assert_eq!(found, Some("correct-key"));

        let not_found = keys.iter().find(|(k, _)| k == "unknown-tenant");
        assert!(not_found.is_none());
    }
}
