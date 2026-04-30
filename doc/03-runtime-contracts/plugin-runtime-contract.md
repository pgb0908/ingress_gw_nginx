# 플러그인 런타임 계약

모든 plugin은 이 계약을 따라야 한다.
이 문서는 Nginx와 Wasm plugin 사이의 공통 실행 규칙을 정의한다.

## 공통 헤더

- `x-request-id`
- `x-trace-id`
- `x-tenant-id`
- `x-route-id`
- `x-service-id`
- `x-gateway-revision`
- `x-gateway-plugin-chain`
- `x-gateway-decision`
- `x-gateway-decision-reason`
- `x-gateway-policy-profile`
- `x-gateway-plugin-version`

규칙:

- 기존 헤더를 삭제하지 않는다.
- 새 헤더는 `x-gateway-*` 또는 기능별 고정 prefix를 사용한다.
- `x-gateway-decision`에는 누적 문자열이 아니라 최종 상태만 기록한다.
- 세부 실패 원인은 `x-gateway-decision-reason` 또는 구조화된 로그 필드로 남긴다.

## lifecycle 훅

plugin은 최소한 다음 lifecycle 훅을 고려해 구현한다.

- `on_request_headers`
- `on_request_body`
- `on_response_headers`
- `on_response_body`
- `on_log`

모든 plugin이 모든 훅을 구현할 필요는 없지만, 어떤 훅에서 결정이 내려졌는지 로그에서 식별 가능해야 한다.

문서상 lifecycle 이름과 실제 Rust 구현의 `proxy-wasm` `trait` 메서드 이름은 다를 수 있다.
현재 저장소의 실제 매핑과 구현 현황은 [Proxy-Wasm Hook Mapping](proxy-wasm-hook-mapping.md) 문서를 기준으로 본다.

## 응답 에러 형식

모든 차단 응답은 아래 형식을 따른다.

```json
{
  "error": {
    "code": "unauthorized",
    "message": "missing or invalid api key",
    "request_id": "req-123",
    "trace_id": "trace-123",
    "route_id": "users-read"
  }
}
```

## 실패 정책

- `fail-open`: plugin 또는 외부 의존성 오류가 있어도 요청을 계속 진행한다.
- `fail-close`: 오류가 발생하면 표준 에러 응답으로 즉시 차단한다.
- `fail-static-response`: 정의된 고정 응답을 반환하고 upstream 호출은 수행하지 않는다.

기본 원칙:

- 인증, tenant 식별, 민감한 rate limit은 기본 `fail-close`
- 관측성 보강과 헤더 정규화는 기본 `fail-open`
- 정적 장애 안내가 필요한 경우 `fail-static-response` 허용

## 버전 호환성

- revision 활성화 전에 plugin version compatibility를 검증해야 한다.
- 호환되지 않는 plugin이 하나라도 있으면 해당 revision은 활성화하지 않는다.
- 요청 로그에는 실행된 plugin chain과 plugin 버전을 함께 남긴다.

## 관련 문서

- [Proxy-Wasm Hook Mapping](proxy-wasm-hook-mapping.md)
- [플러그인 카탈로그](plugin-catalog.md)
- [운영 관측성](../05-operations/observability.md)
