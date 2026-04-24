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

## 트레이싱

- W3C Trace Context 또는 OpenTelemetry trace header 사용
- request 시작 시 trace가 없으면 생성
- upstream으로 전파

## 운영 기준

- request log만 보고 `request_id`, `trace_id`, `revision`, `plugin_chain`, 최종 decision을 확인할 수 있어야 한다.
- reload와 rollback 성공/실패는 메트릭과 로그에서 구분 가능해야 한다.
- 외부 의존성 실패와 fallback 적용 여부가 요청 로그 또는 decision log에 남아야 한다.
