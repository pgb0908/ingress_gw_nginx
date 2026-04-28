use proxy_wasm::traits::{Context, HttpContext, RootContext};
use proxy_wasm::types::{Action, ContextType, LogLevel};

proxy_wasm::main! {{
    proxy_wasm::set_log_level(LogLevel::Info);
    proxy_wasm::set_root_context(|_| -> Box<dyn RootContext> {
        Box::new(TenantFilterRoot)
    });
}}

struct TenantFilterRoot;

impl Context for TenantFilterRoot {}

impl RootContext for TenantFilterRoot {
    fn get_type(&self) -> Option<ContextType> {
        Some(ContextType::HttpContext)
    }

    fn create_http_context(&self, _context_id: u32) -> Option<Box<dyn HttpContext>> {
        Some(Box::new(TenantFilter))
    }
}

struct TenantFilter;

impl Context for TenantFilter {}

impl HttpContext for TenantFilter {
    fn on_http_request_headers(&mut self, _num_headers: usize, _end_of_stream: bool) -> Action {
        let tenant_id = self
            .get_http_request_header("x-tenant-id")
            .unwrap_or_default();

        if tenant_id.trim().is_empty() {
            self.send_http_response(
                401,
                vec![("content-type", "application/json")],
                Some(
                    br#"{"error":{"code":"missing_tenant","message":"tenant header is required"}}"#,
                ),
            );
            return Action::Pause;
        }

        self.set_http_request_header("x-gateway-decision", Some("pending"));
        Action::Continue
    }
}
