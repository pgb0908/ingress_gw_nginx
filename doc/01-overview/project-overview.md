# 프로젝트 개요

## 목표

이 프로젝트의 목표는 ingress-gateway의 데이터 플레인을 구현하는 것이다.
control plane은 외부에 존재한다고 가정하며, 데이터 플레인은 배포된 정적 산출물을 받아 요청 처리와 정책 집행을 담당한다.

- gateway를 단일 데모가 아니라 운영 가능한 제품형 시스템으로 만든다.
- Wasm 플러그인을 버전 단위로 배포하고 롤백할 수 있어야 한다.
- 장애 시 fallback 규칙과 운영 절차가 코드와 문서에 함께 존재해야 한다.

## 산출물

- 데이터 플레인 예제 구현
- `doc/` 운영 절차, 장애 대응, 호환성 규칙 문서
- `app/` ingress-gateway 소스 코드
- `tests/` 단위, 통합, 회귀, 장애, 롤백 테스트
- `scripts/dev/` 배포, 검증, 롤백 자동화 스크립트

## 핵심 원칙

- 이 프로젝트는 ngx_wasm_module(Proxy-Wasm ABI) 사용한다. Nginx + WASM(Rust)
- 데이터 플레인 영역만 개발한다. 즉, nginx + wsam 영역만 구현한다.
- config는 두 가지 방법이 존재한다.
  - static하게 특정 파일 위치의 config 파일을 로딩하여 사용한다.
  - Admin api를 통해 config를 받아와 nginx에 배포 받은 내용을 적용하여 사용한다.
- 모든 변경은 검증 가능하고 되돌릴 수 있어야 한다.
- 정책 결정과 실패 원인은 로그와 메트릭에 남아야 한다.


### 플러그인 런타임 계약

반드시 정의하고 구현할 항목:

- 플러그인 입력 헤더
- 플러그인 출력 헤더
- 공통 메타데이터 키
- 에러 반환 포맷
- 플러그인 lifecycle 훅
- `fail-open` / `fail-close` / `fail-static-response`
- 플러그인 버전 호환성

### 관측성

반드시 구현할 항목:

- JSON structured access log
- plugin별 decision log
- Prometheus metrics endpoint
- OpenTelemetry trace context propagation
- `request_id`, `trace_id`, `tenant_id`, `route_id`, `plugin_chain`, `revision` 노출

필수 메트릭:

- `gateway_requests_total`
- `gateway_request_duration_ms`
- `gateway_plugin_executions_total`
- `gateway_plugin_failures_total`
- `gateway_policy_denied_total`
- `gateway_rate_limit_denied_total`
- `gateway_reload_total`
- `gateway_reload_failures_total`

## 권장 저장소 구조

```text
app/
env/
  dev-env.env
  local/
  cache/
fixtures/
  revisions/
runtime/
  revisions/
  dataplane/
  process/
scripts/
  dev/
tests/
  unit/
  integration/
  e2e/
  chaos/
doc/
  01-overview/
  02-architecture/
  03-runtime-contracts/
  04-config-models/
  05-operations/
```

## 관련 문서

- [용어집](glossary.md)
- [데이터 플레인 아키텍처](../02-architecture/dataplane-architecture.md)
- [플러그인 런타임 계약](../03-runtime-contracts/plugin-runtime-contract.md)
