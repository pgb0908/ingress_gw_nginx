모든 플러그인은 아래 계약을 따라야 한다.

### 공통 헤더

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

### 응답 에러 형식

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

### 메타데이터 전달 규칙

- 기존 헤더를 삭제하지 않는다
- 새 헤더는 prefix 규칙을 따른다
- 정책 결정 결과는 `x-gateway-decision`에 누적 가능한 문자열이 아니라 최종 상태만 기록한다
- 세부 원인은 `x-gateway-decision-reason` 또는 로그 필드로 남긴다
