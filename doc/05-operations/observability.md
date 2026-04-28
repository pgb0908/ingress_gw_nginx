# 운영 관측성

데이터 플레인은 요청 단위 추적과 revision 운영 추적이 가능해야 한다.
이 문서는 로그, 메트릭, 트레이싱의 최소 계약을 정의한다.

## 로그

모든 access log는 JSON 한 줄 형식을 쓴다.

필수 필드:

- `timestamp`
- `request_id`
- `trace_id`
- `tenant_id`
- `route_id`
- `service_id`
- `revision`
- `plugin_chain`
- `decision`
- `decision_reason`
- `status`
- `latency_ms`
- `upstream_latency_ms`

추가 권장 필드:

- `method`
- `host`
- `path`
- `plugin_chain`
- `plugin_version`
- `fallback_applied`

### 로그 출력 예시

```json
{
  "timestamp": "2026-04-28T10:30:01.234Z",
  "request_id": "req-7f3a1b2c",
  "trace_id": "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01",
  "tenant_id": "acme-corp",
  "route_id": "orders-read",
  "service_id": "orders-service",
  "revision": "prod-2026-04-28-001",
  "plugin_chain": ["tenant-filter@1.0.0", "auth-filter@1.2.0", "rate-limit-filter@1.0.0", "observe-filter@1.0.0"],
  "decision": "allow",
  "decision_reason": "all policies passed",
  "status": 200,
  "latency_ms": 48,
  "upstream_latency_ms": 32,
  "method": "GET",
  "host": "api.example.com",
  "path": "/v1/orders",
  "fallback_applied": false
}
```

차단된 요청 예시:

```json
{
  "timestamp": "2026-04-28T10:30:05.891Z",
  "request_id": "req-9a4d2e1f",
  "trace_id": "00-abc123def456-0011223344-01",
  "tenant_id": "unknown",
  "route_id": "orders-read",
  "service_id": "orders-service",
  "revision": "prod-2026-04-28-001",
  "plugin_chain": ["tenant-filter@1.0.0"],
  "decision": "deny",
  "decision_reason": "tenant not found",
  "status": 401,
  "latency_ms": 3,
  "upstream_latency_ms": 0,
  "method": "POST",
  "host": "api.example.com",
  "path": "/v1/orders",
  "fallback_applied": false
}
```

## 메트릭

Prometheus scrape 가능한 endpoint를 제공한다.

라벨 최소 항목:

- `route_id`
- `service_id`
- `tenant_id`
- `plugin`
- `revision`
- `decision`

필수 메트릭:

- `gateway_requests_total`
- `gateway_request_duration_ms`
- `gateway_plugin_executions_total`
- `gateway_plugin_failures_total`
- `gateway_policy_denied_total`
- `gateway_rate_limit_denied_total`
- `gateway_reload_total`
- `gateway_reload_failures_total`

### 메트릭 출력 예시

```
# HELP gateway_requests_total Total number of requests processed
# TYPE gateway_requests_total counter
gateway_requests_total{route_id="orders-read",service_id="orders-service",tenant_id="acme-corp",revision="prod-2026-04-28-001",decision="allow"} 1024
gateway_requests_total{route_id="orders-read",service_id="orders-service",tenant_id="acme-corp",revision="prod-2026-04-28-001",decision="deny"} 17

# HELP gateway_request_duration_ms Request duration in milliseconds
# TYPE gateway_request_duration_ms histogram
gateway_request_duration_ms_bucket{route_id="orders-read",le="10"} 120
gateway_request_duration_ms_bucket{route_id="orders-read",le="50"} 890
gateway_request_duration_ms_bucket{route_id="orders-read",le="200"} 1020
gateway_request_duration_ms_bucket{route_id="orders-read",le="+Inf"} 1024

# HELP gateway_plugin_executions_total Total plugin executions
# TYPE gateway_plugin_executions_total counter
gateway_plugin_executions_total{plugin="auth-filter",revision="prod-2026-04-28-001"} 1041
gateway_plugin_executions_total{plugin="rate-limit-filter",revision="prod-2026-04-28-001"} 1024

# HELP gateway_reload_total Total nginx reload attempts
# TYPE gateway_reload_total counter
gateway_reload_total 5

# HELP gateway_reload_failures_total Total nginx reload failures
# TYPE gateway_reload_failures_total counter
gateway_reload_failures_total 1
```

## 트레이싱

- W3C Trace Context 또는 OpenTelemetry trace header 사용
- request 시작 시 trace가 없으면 생성
- upstream으로 전파

## 운영 기준

- request log만 보고 `request_id`, `trace_id`, `revision`, `plugin_chain`, 최종 decision을 확인할 수 있어야 한다.
- reload와 rollback 성공/실패는 메트릭과 로그에서 구분 가능해야 한다.
- 외부 의존성 실패와 fallback 적용 여부가 요청 로그 또는 decision log에 남아야 한다.
