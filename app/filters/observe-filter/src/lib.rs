use proxy_wasm::traits::{Context, HttpContext, RootContext};
use proxy_wasm::types::{Action, ContextType, LogLevel};

proxy_wasm::main! {{
    proxy_wasm::set_log_level(LogLevel::Info);
    proxy_wasm::set_root_context(|_| -> Box<dyn RootContext> {
        Box::new(ObserveFilterRoot)
    });
}}

struct ObserveFilterRoot;

impl Context for ObserveFilterRoot {}

impl RootContext for ObserveFilterRoot {
    fn get_type(&self) -> Option<ContextType> {
        Some(ContextType::HttpContext)
    }

    fn create_http_context(&self, context_id: u32) -> Option<Box<dyn HttpContext>> {
        Some(Box::new(ObserveFilter { context_id }))
    }
}

struct ObserveFilter {
    context_id: u32,
}

impl Context for ObserveFilter {}

impl HttpContext for ObserveFilter {
    fn on_http_request_headers(&mut self, _num_headers: usize, _end_of_stream: bool) -> Action {
        let start_ns = {
            use std::time::UNIX_EPOCH;
            self.get_current_time()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64
        };
        let key = format!("obs:start:{}", self.context_id);
        let encoded = start_ns.to_le_bytes();
        let _ = self.set_shared_data(&key, Some(&encoded), None);
        Action::Continue
    }

    fn on_http_response_headers(&mut self, _num_headers: usize, _end_of_stream: bool) -> Action {
        if let Some(decision) = self.get_http_request_header("x-gateway-decision") {
            self.set_http_response_header("x-gateway-decision", Some(&decision));
        }
        if let Some(reason) = self.get_http_request_header("x-gateway-decision-reason") {
            self.set_http_response_header("x-gateway-decision-reason", Some(&reason));
        }
        if let Some(revision) = self.get_http_request_header("x-gateway-revision") {
            self.set_http_response_header("x-gateway-revision", Some(&revision));
        }
        Action::Continue
    }

    fn on_log(&mut self) {
        let key = format!("obs:start:{}", self.context_id);
        let elapsed_ms = match self.get_shared_data(&key).0 {
            Some(b) if b.len() >= 8 => {
                let start_ns = u64::from_le_bytes(b[0..8].try_into().unwrap_or([0u8; 8]));
                let now_ns = {
                    use std::time::UNIX_EPOCH;
                    self.get_current_time()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_nanos() as u64
                };
                now_ns.saturating_sub(start_ns) / 1_000_000
            }
            _ => 0,
        };

        let decision = self
            .get_http_request_header("x-gateway-decision")
            .unwrap_or_else(|| "unknown".to_string());
        let reason = self
            .get_http_request_header("x-gateway-decision-reason")
            .unwrap_or_default();
        let tenant = self
            .get_http_request_header("x-tenant-id")
            .unwrap_or_default();
        let route = self
            .get_http_request_header("x-route-id")
            .unwrap_or_default();
        let revision = self
            .get_http_request_header("x-gateway-revision")
            .unwrap_or_default();

        let msg = format!(
            "observe: decision={} reason={} tenant={} route={} revision={} latency_ms={}",
            decision, reason, tenant, route, revision, elapsed_ms
        );
        let _ = proxy_wasm::hostcalls::log(LogLevel::Info, &msg);

        let _ = self.set_shared_data(&key, None, None);
    }
}
