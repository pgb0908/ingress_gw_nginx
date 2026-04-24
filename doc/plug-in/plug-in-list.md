## 필수 플러그인 목록

### auth-filter

책임:

- API key 또는 JWT 검증
- secret reference 기반 키 조회 결과 사용
- 인증 실패 시 표준 JSON 응답 반환

입력:

- `Authorization`
- `x-api-key`
- `x-tenant-id`

출력:

- `x-auth-subject`
- `x-auth-method`
- `x-gateway-decision`

실패 정책:

- 기본 `fail-close`

### tenant-filter

책임:

- tenant 식별
- tenant별 plugin enable/disable 적용
- tenant별 정책 프로필 선택

실패 정책:

- 기본 `fail-close`

### header-filter

책임:

- 공통 헤더 주입
- request_id, trace_id, organization, route_id 표준화

실패 정책:

- 기본 `fail-open`

### rate-limit-filter

책임:

- Redis 기반 rate limit 조회 및 차감
- tenant/service/route 기준 quota 적용

실패 정책:

- 정책별 선택 가능
- 기본은 `fail-open` 이 아니라 `configurable`

주의:

- 금융성 또는 민감 API는 `fail-close`
- 공개성 읽기 API는 `fail-open` 허용 가능

### observe-filter

책임:

- trace metadata 보강
- plugin execution 결과 헤더화
- timing metadata 기록

실패 정책:

- 기본 `fail-open`