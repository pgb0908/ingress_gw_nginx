# 프로젝트 개요

## 목표

이 프로젝트의 목표는 ingress-gateway의 데이터 플레인을 구현하는 것이다.
control plane은 외부에 존재한다고 가정하며, 데이터 플레인은 배포된 정적 산출물을 받아 요청 처리와 정책 집행을 담당한다.

- 정책 변경을 서비스 배포와 분리한다.
- route, service, policy 단위 정책을 중앙에서 관리할 수 있어야 한다.
- gateway를 단일 데모가 아니라 운영 가능한 제품형 시스템으로 만든다.
- Wasm 플러그인을 버전 단위로 배포하고 롤백할 수 있어야 한다.
- 장애 시 fallback 규칙과 운영 절차가 코드와 문서에 함께 존재해야 한다.
- 로그, 메트릭, 트레이싱으로 요청 단위 문제 추적이 가능해야 한다.

## 구현 범위

- 데이터 플레인과 제어 플레인의 명확한 분리
- 정책 배포, 검증, 롤백 자동화
- 관측성 기본 완성
- secret, rate limit, 정책의 외부 시스템 연동
- 운영자와 서비스 팀이 함께 쓸 수 있는 제품형 구조

## 산출물

- 데이터 플레인 예제 구현
- `doc/` 운영 절차, 장애 대응, 호환성 규칙 문서
- `src/` ingress-gateway 소스 코드
- `tests/` 단위, 통합, 회귀, 장애, 롤백 테스트
- `src/scripts/` 배포, 검증, 롤백 자동화 스크립트

## 핵심 원칙

- Nginx는 네트워크와 프록시 코어에 집중한다.
- Wasm은 공통 정책 집행에 집중한다.
- 운영 설정은 control plane이 관리한다는 가정으로 개발한다.
- 요청 경로는 stateless에 가깝게 유지한다.
- 정책 배포는 서비스 배포와 분리한다.
- 모든 변경은 검증 가능하고 되돌릴 수 있어야 한다.
- 정책 결정과 실패 원인은 로그와 메트릭에 남아야 한다.

## 필수 기능

### 데이터 플레인

- Nginx 기반 ingress
- 다중 Wasm 체인 실행
- control plane이 생성한 정적 배포 산출물 로드
- policy 기반 정책 적용
- 요청 차단, 허용, 헤더 표준화, trace propagation
- rate limit 조회
- fallback 정책 수행

최소 요구사항:

- 설정 파일과 Wasm 모듈을 revision 단위 디렉터리로 로드
- reload 전 `nginx -t` 검증
- 실패 시 이전 revision 유지

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
src/
  gateway/
  plugins/
  runtime-config/
    revisions/
    current -> revisions/<revision>
  upstreams/
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
