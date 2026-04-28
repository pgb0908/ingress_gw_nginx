use crate::runtime::GatewayRuntime;
use crate::state::load_state;
use anyhow::Result;
use serde::Deserialize;
use serde_json::json;
use std::path::PathBuf;
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

pub fn serve_admin(host: &str, port: u16) -> Result<()> {
    let server = Server::http(format!("{host}:{port}"))
        .map_err(|error| anyhow::anyhow!("failed to bind admin server: {error}"))?;
    let runtime = GatewayRuntime::new();

    for request in server.incoming_requests() {
        if let Err(error) = handle_request(request, &runtime) {
            eprintln!("admin server error: {error:#}");
        }
    }
    Ok(())
}

fn handle_request(mut request: Request, runtime: &GatewayRuntime) -> Result<()> {
    match (request.method(), request.url()) {
        (&Method::Get, "/status") => {
            let state = load_state()?;
            respond_json(request, 200, &state)?;
        }
        (&Method::Get, "/metrics") => {
            let state = load_state()?;
            let m = state.metrics;
            let body = format!(
                "# HELP gateway_reload_total Total nginx reload attempts\n\
# TYPE gateway_reload_total counter\n\
gateway_reload_total {}\n\
# HELP gateway_reload_failures_total Total nginx reload failures\n\
# TYPE gateway_reload_failures_total counter\n\
gateway_reload_failures_total {}\n\
# HELP gateway_requests_total Total number of requests processed\n\
# TYPE gateway_requests_total counter\n\
gateway_requests_total {}\n\
# HELP gateway_request_duration_ms Request duration in milliseconds\n\
# TYPE gateway_request_duration_ms counter\n\
gateway_request_duration_ms {}\n\
# HELP gateway_plugin_executions_total Total plugin executions\n\
# TYPE gateway_plugin_executions_total counter\n\
gateway_plugin_executions_total {}\n\
# HELP gateway_plugin_failures_total Total plugin failures\n\
# TYPE gateway_plugin_failures_total counter\n\
gateway_plugin_failures_total {}\n\
# HELP gateway_policy_denied_total Total requests denied by policy\n\
# TYPE gateway_policy_denied_total counter\n\
gateway_policy_denied_total {}\n\
# HELP gateway_rate_limit_denied_total Total requests denied by rate limit\n\
# TYPE gateway_rate_limit_denied_total counter\n\
gateway_rate_limit_denied_total {}\n",
                m.gateway_reload_total,
                m.gateway_reload_failures_total,
                m.gateway_requests_total,
                m.gateway_request_duration_ms,
                m.gateway_plugin_executions_total,
                m.gateway_plugin_failures_total,
                m.gateway_policy_denied_total,
                m.gateway_rate_limit_denied_total
            );
            let response = Response::from_string(body)
                .with_status_code(200)
                .with_header(content_type("text/plain; version=0.0.4"));
            request.respond(response)?;
        }
        (&Method::Post, "/admin/revisions/load") => {
            let body: RevisionPathBody = read_json_body(&mut request)?;
            let result = runtime.load_revision(&body.path)?;
            let code = if result.status == "loaded" { 200 } else { 400 };
            respond_json(request, code, &result)?;
        }
        (&Method::Post, "/admin/config") => {
            let body: RevisionPathBody = read_json_body(&mut request)?;
            let result = runtime.load_revision(&body.path)?;
            let code = if result.status == "loaded" { 200 } else { 400 };
            respond_json(request, code, &result)?;
        }
        _ => {
            respond_json(request, 404, &json!({ "error": "not found" }))?;
        }
    }
    Ok(())
}

fn read_json_body<T: for<'de> Deserialize<'de>>(request: &mut Request) -> Result<T> {
    let mut body = Vec::new();
    request.as_reader().read_to_end(&mut body)?;
    Ok(serde_json::from_slice(&body)?)
}

fn respond_json(request: Request, status: u16, payload: &impl serde::Serialize) -> Result<()> {
    let body = serde_json::to_vec(payload)?;
    let response = Response::from_data(body)
        .with_status_code(StatusCode(status))
        .with_header(content_type("application/json"));
    request.respond(response)?;
    Ok(())
}

fn content_type(value: &str) -> Header {
    Header::from_bytes("Content-Type", value).expect("valid content type")
}

#[derive(Debug, Deserialize)]
struct RevisionPathBody {
    path: PathBuf,
}
