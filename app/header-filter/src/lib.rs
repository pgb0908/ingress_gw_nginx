// Pure helpers — compiled on all targets
pub fn make_id(prefix: &str, context_id: u32) -> String {
    format!("{}-{:08x}", prefix, context_id)
}

pub fn extract_string_field(s: &str, field: &str) -> String {
    let needle = format!("\"{}\"", field);
    let pos = match s.find(&needle) {
        Some(p) => p,
        None => return String::new(),
    };
    let after = &s[pos + needle.len()..];
    let colon = match after.find(':') {
        Some(p) => p,
        None => return String::new(),
    };
    let after = after[colon + 1..].trim_start();
    if after.starts_with('"') {
        let inner = &after[1..];
        inner.find('"').map(|end| inner[..end].to_string()).unwrap_or_default()
    } else {
        String::new()
    }
}

// Wasm filter implementation — only compiled for wasm32 target
#[cfg(target_arch = "wasm32")]
mod wasm_filter {
    use super::{extract_string_field, make_id};
    use proxy_wasm::traits::{Context, HttpContext, RootContext};
    use proxy_wasm::types::{Action, ContextType, LogLevel};

    proxy_wasm::main! {{
        proxy_wasm::set_log_level(LogLevel::Info);
        proxy_wasm::set_root_context(|_| -> Box<dyn RootContext> {
            Box::new(HeaderFilterRoot { revision: String::new(), plugin_chain: String::new() })
        });
    }}

    struct HeaderFilterRoot {
        revision: String,
        plugin_chain: String,
    }

    impl Context for HeaderFilterRoot {}

    impl RootContext for HeaderFilterRoot {
        fn on_configure(&mut self, _config_size: usize) -> bool {
            if let Some(bytes) = self.get_plugin_configuration() {
                if let Ok(s) = core::str::from_utf8(&bytes) {
                    self.revision = extract_string_field(s, "revision");
                    self.plugin_chain = extract_string_field(s, "plugin_chain");
                }
            }
            true
        }

        fn get_type(&self) -> Option<ContextType> {
            Some(ContextType::HttpContext)
        }

        fn create_http_context(&self, context_id: u32) -> Option<Box<dyn HttpContext>> {
            Some(Box::new(HeaderFilter {
                context_id,
                revision: self.revision.clone(),
                plugin_chain: self.plugin_chain.clone(),
            }))
        }
    }

    struct HeaderFilter {
        context_id: u32,
        revision: String,
        plugin_chain: String,
    }

    impl Context for HeaderFilter {}

    impl HttpContext for HeaderFilter {
        fn on_http_request_headers(&mut self, _num_headers: usize, _end_of_stream: bool) -> Action {
            if self.get_http_request_header("x-request-id").unwrap_or_default().is_empty() {
                let id = make_id("req", self.context_id);
                self.set_http_request_header("x-request-id", Some(&id));
            }
            if self.get_http_request_header("x-trace-id").unwrap_or_default().is_empty() {
                let id = make_id("trc", self.context_id);
                self.set_http_request_header("x-trace-id", Some(&id));
            }
            if !self.revision.is_empty() {
                self.set_http_request_header("x-gateway-revision", Some(&self.revision));
            }
            if !self.plugin_chain.is_empty() {
                self.set_http_request_header("x-gateway-plugin-chain", Some(&self.plugin_chain));
            }
            Action::Continue
        }

        fn on_http_response_headers(&mut self, _num_headers: usize, _end_of_stream: bool) -> Action {
            if self.get_http_response_header("x-gateway-decision").unwrap_or_default().is_empty() {
                self.set_http_response_header("x-gateway-decision", Some("allow"));
            }
            Action::Continue
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_field_basic_string() {
        assert_eq!(extract_string_field(r#"{"revision":"rev-001"}"#, "revision"), "rev-001");
    }

    #[test]
    fn extract_field_multiple_fields() {
        let cfg = r#"{"revision":"rev-123","plugin_chain":"tenant,auth"}"#;
        assert_eq!(extract_string_field(cfg, "revision"), "rev-123");
        assert_eq!(extract_string_field(cfg, "plugin_chain"), "tenant,auth");
    }

    #[test]
    fn extract_field_missing_key_returns_empty() {
        assert_eq!(extract_string_field(r#"{"other":"val"}"#, "revision"), "");
    }

    #[test]
    fn extract_field_empty_string_returns_empty() {
        assert_eq!(extract_string_field("", "revision"), "");
    }

    #[test]
    fn extract_field_with_whitespace() {
        assert_eq!(extract_string_field(r#"{ "revision" : "spaced-rev" }"#, "revision"), "spaced-rev");
    }

    #[test]
    fn make_id_format_is_prefix_hex() {
        let id = make_id("req", 1);
        assert!(id.starts_with("req-"), "id should start with prefix-");
        assert_eq!(id, "req-00000001");
    }

    #[test]
    fn make_id_zero_context() {
        assert_eq!(make_id("trc", 0), "trc-00000000");
    }

    #[test]
    fn make_id_large_context_id() {
        let id = make_id("req", 0xdeadbeef);
        assert_eq!(id, "req-deadbeef");
    }

    #[test]
    fn make_id_different_prefixes_differ() {
        let req_id = make_id("req", 42);
        let trc_id = make_id("trc", 42);
        assert_ne!(req_id, trc_id);
        assert_eq!(req_id, "req-0000002a");
        assert_eq!(trc_id, "trc-0000002a");
    }
}
