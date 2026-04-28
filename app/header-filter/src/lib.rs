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
        // Inject x-request-id if missing
        if self
            .get_http_request_header("x-request-id")
            .unwrap_or_default()
            .is_empty()
        {
            let id = make_id("req", self.context_id);
            self.set_http_request_header("x-request-id", Some(&id));
        }

        // Inject x-trace-id if missing
        if self
            .get_http_request_header("x-trace-id")
            .unwrap_or_default()
            .is_empty()
        {
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
        if self
            .get_http_response_header("x-gateway-decision")
            .unwrap_or_default()
            .is_empty()
        {
            self.set_http_response_header("x-gateway-decision", Some("allow"));
        }
        Action::Continue
    }
}

fn make_id(prefix: &str, context_id: u32) -> String {
    // Simple deterministic ID using context_id; sufficient for dev/tracing
    format!("{}-{:08x}", prefix, context_id)
}

// Minimal field extractor for {"key":"value"} JSON
fn extract_string_field(s: &str, field: &str) -> String {
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
        inner
            .find('"')
            .map(|end| inner[..end].to_string())
            .unwrap_or_default()
    } else {
        String::new()
    }
}
