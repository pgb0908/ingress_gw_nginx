### 로그

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

### 메트릭

Prometheus scrape 가능한 endpoint를 제공한다.

라벨 최소 항목:

- `route_id`
- `service_id`
- `tenant_id`
- `plugin`
- `revision`
- `decision`

### 트레이싱

- W3C Trace Context 또는 OpenTelemetry trace header 사용
- request 시작 시 trace가 없으면 생성
- upstream으로 전파