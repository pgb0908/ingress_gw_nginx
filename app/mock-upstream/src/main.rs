use anyhow::Result;
use serde_json::json;
use tiny_http::{Header, Response, Server, StatusCode};

fn main() -> Result<()> {
    let server = Server::http("127.0.0.1:18080")
        .map_err(|error| anyhow::anyhow!("failed to bind mock upstream: {error}"))?;
    for request in server.incoming_requests() {
        let payload = json!({
            "service": "orders-svc",
            "path": request.url(),
            "request_id": header_value(request.headers(), "X-Request-Id"),
            "trace_id": header_value(request.headers(), "X-Trace-Id"),
            "tenant_id": header_value(request.headers(), "X-Tenant-Id"),
            "route_id": header_value(request.headers(), "X-Route-Id"),
            "revision": header_value(request.headers(), "X-Gateway-Revision"),
        });
        let body = serde_json::to_vec(&payload)?;
        let response = Response::from_data(body)
            .with_status_code(StatusCode(200))
            .with_header(
                Header::from_bytes("Content-Type".as_bytes(), "application/json".as_bytes())
                    .map_err(|_| anyhow::anyhow!("invalid content-type header"))?,
            );
        request.respond(response)?;
    }
    Ok(())
}

fn header_value(headers: &[Header], name: &str) -> Option<String> {
    headers
        .iter()
        .find(|header| format!("{}", header.field).eq_ignore_ascii_case(name))
        .map(|header| header.value.as_str().to_string())
}
