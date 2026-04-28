use crate::paths;
use crate::providers::{consume_rate_limit, load_secret};
use crate::revision::load_revision_bundle;
use crate::runtime::GatewayRuntime;
use crate::state::{load_state, save_state};
use anyhow::Result;
use serde::Deserialize;
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};
use uuid::Uuid;

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
            let metrics = state.metrics;
            let body = format!(
                "gateway_reload_total {}\n\
gateway_reload_failures_total {}\n\
gateway_requests_total {}\n\
gateway_request_duration_ms {}\n\
gateway_plugin_executions_total {}\n\
gateway_plugin_failures_total {}\n\
gateway_policy_denied_total {}\n\
gateway_rate_limit_denied_total {}\n",
                metrics.gateway_reload_total,
                metrics.gateway_reload_failures_total,
                metrics.gateway_requests_total,
                metrics.gateway_request_duration_ms,
                metrics.gateway_plugin_executions_total,
                metrics.gateway_plugin_failures_total,
                metrics.gateway_policy_denied_total,
                metrics.gateway_rate_limit_denied_total
            );
            let response = Response::from_string(body)
                .with_status_code(200)
                .with_header(content_type("text/plain; version=0.0.4"));
            request.respond(response)?;
        }
        (&Method::Get, "/plugin-check") | (&Method::Post, "/plugin-check") => {
            handle_plugin_check(request)?;
        }
        (&Method::Post, "/admin/revisions/validate") => {
            let body: RevisionBody = read_json_body(&mut request)?;
            let result = runtime.validate_revision(&body.revision_path)?;
            let code = if result.valid { 200 } else { 400 };
            respond_json(request, code, &result)?;
        }
        (&Method::Post, "/admin/revisions/activate") => {
            let body: RevisionBody = read_json_body(&mut request)?;
            let result = runtime.activate_revision(&body.revision_path)?;
            let code = if result.status == "activated" { 200 } else { 400 };
            respond_json(request, code, &result)?;
        }
        (&Method::Post, "/admin/revisions/rollback") => {
            let result = runtime.rollback()?;
            let code = if result.status == "activated" || result.status == "rolled_back" {
                200
            } else {
                400
            };
            respond_json(request, code, &result)?;
        }
        _ => {
            respond_json(request, 404, &json!({ "error": "not found" }))?;
        }
    }
    Ok(())
}

fn handle_plugin_check(request: Request) -> Result<()> {
    let started = Instant::now();
    let current = resolve_current_revision()?;
    let bundle = load_revision_bundle(&current)?;
    let headers = request.headers();

    let request_id = header_value(headers, "X-Request-Id").unwrap_or_else(|| Uuid::new_v4().to_string());
    let trace_id = header_value(headers, "X-Trace-Id").unwrap_or_else(|| Uuid::new_v4().to_string());
    let tenant_id = header_value(headers, "X-Tenant-Id").unwrap_or_default();
    let route_id = header_value(headers, "X-Route-Id").unwrap_or_else(|| "unknown-route".to_string());
    let service_id = header_value(headers, "X-Service-Id")
        .or_else(|| service_for_route(&bundle, &route_id))
        .unwrap_or_else(|| "unknown-service".to_string());
    let api_key = header_value(headers, "X-Api-Key").unwrap_or_default();
    let plugin_versions = bundle
        .manifest
        .plugins
        .iter()
        .map(|plugin| format!("{}:{}", plugin.name, plugin.version))
        .collect::<Vec<_>>()
        .join(",");
    let policy_files = bundle
        .policies
        .iter()
        .filter(|policy| {
            policy.document.spec.target_ref.name == service_id || policy.document.spec.target_ref.name.starts_with("orders-route")
        })
        .map(|policy| {
            policy
                .source_file
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .to_string()
        })
        .collect::<Vec<_>>()
        .join(",");

    let mut state = load_state()?;
    state.metrics.gateway_requests_total += 1;
    state.metrics.gateway_plugin_executions_total += bundle.plugin_chain.len() as u64;

    let base_headers: Vec<(String, String)> = vec![
        ("X-Request-Id".to_string(), request_id.clone()),
        ("X-Trace-Id".to_string(), trace_id.clone()),
        ("X-Tenant-Id".to_string(), tenant_id.clone()),
        ("X-Route-Id".to_string(), route_id.clone()),
        ("X-Service-Id".to_string(), service_id.clone()),
        ("X-Gateway-Revision".to_string(), bundle.manifest.revision.clone()),
        ("X-Gateway-Plugin-Chain".to_string(), bundle.plugin_chain.join(",")),
        ("X-Gateway-Policy-Profile".to_string(), "default".to_string()),
        ("X-Gateway-Plugin-Version".to_string(), plugin_versions),
        ("X-Gateway-Policy-Files".to_string(), policy_files),
    ];

    if tenant_id.is_empty() {
        state.metrics.gateway_policy_denied_total += 1;
        save_state(&state)?;
        return respond_error(
            request,
            401,
            "missing_tenant",
            "tenant header is required",
            &request_id,
            &trace_id,
            &route_id,
            &base_headers,
        );
    }

    let expected_api_key = load_secret(&bundle.root.join("data/secrets.json"), &tenant_id)?;
    if let Some(expected_api_key) = expected_api_key {
        if expected_api_key != api_key {
            state.metrics.gateway_policy_denied_total += 1;
            save_state(&state)?;
        return respond_error(
                request,
                401,
                "unauthorized",
                "missing or invalid api key",
                &request_id,
                &trace_id,
                &route_id,
                &base_headers,
            );
        }
    }

    let decision = consume_rate_limit(
        &bundle.root.join("data/rate-limit.json"),
        &paths::rate_limit_state_file(),
        &tenant_id,
        &service_id,
        &route_id,
        epoch_seconds(),
    )?;

    if !decision.allowed {
        state.metrics.gateway_rate_limit_denied_total += 1;
        save_state(&state)?;
        let mut response_headers = base_headers.clone();
        response_headers.push(("X-Gateway-Decision".to_string(), "deny".to_string()));
        response_headers.push((
            "X-Gateway-Decision-Reason".to_string(),
            "rate_limited".to_string(),
        ));
        return respond_json_with_headers(
            request,
            429,
            &json!({
                "error": {
                    "code": "rate_limited",
                    "message": "rate limit exceeded",
                    "request_id": request_id,
                    "trace_id": trace_id,
                    "route_id": route_id,
                }
            }),
            &response_headers,
        );
    }

    state.metrics.gateway_request_duration_ms += started.elapsed().as_millis() as u64;
    save_state(&state)?;

    let mut response_headers = base_headers;
    response_headers.push(("X-Gateway-Decision".to_string(), "allow".to_string()));
    response_headers.push((
        "X-Gateway-Decision-Reason".to_string(),
        "policy_passed".to_string(),
    ));
    response_headers.push(("X-Auth-Subject".to_string(), tenant_id.clone()));
    response_headers.push(("X-Auth-Method".to_string(), "x-api-key".to_string()));
    response_headers.push((
        "X-Gateway-Limit-Remaining".to_string(),
        decision.remaining.to_string(),
    ));
    respond_json_with_headers(request, 200, &json!({ "status": "ok" }), &response_headers)
}

fn respond_error(
    request: Request,
    status: u16,
    code: &str,
    message: &str,
    request_id: &str,
    trace_id: &str,
    route_id: &str,
    base_headers: &[(String, String)],
) -> Result<()> {
    let mut headers = base_headers.to_vec();
    headers.push(("X-Gateway-Decision".to_string(), "deny".to_string()));
    headers.push(("X-Gateway-Decision-Reason".to_string(), code.to_string()));
    respond_json_with_headers(
        request,
        status,
        &json!({
            "error": {
                "code": code,
                "message": message,
                "request_id": request_id,
                "trace_id": trace_id,
                "route_id": route_id
            }
        }),
        &headers,
    )
}

fn resolve_current_revision() -> Result<PathBuf> {
    let link = paths::current_link();
    if link.exists() || link.is_symlink() {
        return fs::canonicalize(&link).map_err(Into::into);
    }
    anyhow::bail!("current revision is not active");
}

fn service_for_route(bundle: &crate::models::RevisionBundle, route_id: &str) -> Option<String> {
    for router in &bundle.routers {
        if route_id.starts_with(&router.metadata.name) || route_id.starts_with('/') {
            return Some(router.spec.config.destinations[0].destination_ref.name.clone());
        }
    }
    bundle.services.keys().next().cloned()
}

fn epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_secs()
}

fn header_value(headers: &[Header], name: &str) -> Option<String> {
    headers
        .iter()
        .find(|header| format!("{}", header.field).eq_ignore_ascii_case(name))
        .map(|header| header.value.as_str().to_string())
}

fn read_json_body<T: for<'de> Deserialize<'de>>(request: &mut Request) -> Result<T> {
    let mut body = Vec::new();
    request.as_reader().read_to_end(&mut body)?;
    Ok(serde_json::from_slice(&body)?)
}

fn respond_json(request: Request, status: u16, payload: &impl serde::Serialize) -> Result<()> {
    let headers: Vec<(String, String)> = Vec::new();
    respond_json_with_headers(request, status, payload, &headers)
}

fn respond_json_with_headers(
    request: Request,
    status: u16,
    payload: &impl serde::Serialize,
    headers: &[(String, String)],
) -> Result<()> {
    let body = serde_json::to_vec(payload)?;
    let mut response = Response::from_data(body)
        .with_status_code(StatusCode(status))
        .with_header(content_type("application/json"));
    for (name, value) in headers {
        let header = Header::from_bytes(name.as_bytes(), value.as_bytes())
            .map_err(|_| anyhow::anyhow!("invalid header: {name}"))?;
        response = response.with_header(header);
    }
    request.respond(response)?;
    Ok(())
}

fn content_type(value: &str) -> Header {
    Header::from_bytes("Content-Type", value).expect("valid content type")
}

#[derive(Debug, Deserialize)]
struct RevisionBody {
    revision_path: PathBuf,
}
