# 플러그인 카탈로그

기본 데이터 플레인 구현에서 우선 제공해야 할 plugin 목록이다.
각 plugin은 아래 책임, 입력/출력, 기본 failure mode를 가진다.

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

기본 실패 정책:

- 기본 `fail-close`

### tenant-filter

책임:

- tenant 식별
- tenant별 plugin enable/disable 적용
- tenant별 정책 프로필 선택

기본 실패 정책:

- 기본 `fail-close`

### header-filter

책임:

- 공통 헤더 주입
- request_id, trace_id, organization, route_id 표준화

기본 실패 정책:

- 기본 `fail-open`

### rate-limit-filter

책임:

- Redis 기반 rate limit 조회 및 차감
- tenant/service/route 기준 quota 적용

기본 실패 정책:

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

기본 실패 정책:

- 기본 `fail-open`

### message-filter

> **현재 미구현 (계획 중)** — 아래 명세는 계획 사양이며 revision 번들에 포함하지 않는다.

책임:

- 바디의 전문을 파싱
- config를 통해 메시지 스키마 배포받음

기본 실패 정책:

- 기본 `fail-close`

## 구현 메모

- `auth-filter`와 `tenant-filter`는 요청 초기에 실행한다.
- `header-filter`는 request/response 공통 메타데이터 정규화 책임을 가진다.
- `rate-limit-filter`는 외부 저장소 조회 실패 시 정책별 failure mode를 따라야 한다.
- `observe-filter`는 요청 처리 종료 시점의 최종 decision과 latency를 기록할 수 있어야 한다.
