# Proxy-Wasm Hook Mapping

이 문서는 이 저장소에서 사용하는 Proxy-Wasm 훅을 개념 분류, revision 메타데이터, 실제 Rust 구현 기준으로 함께 정리한다.
문서상 분류와 코드상 메서드 이름이 다르기 때문에, 구현을 읽을 때는 이 매핑을 기준으로 본다.

## `trait`가 의미하는 것

Rust의 `trait`는 런타임과 플러그인 사이의 인터페이스다.
`proxy-wasm` 라이브러리는 특정 `trait` 메서드를 구현하면 Nginx + Wasm 런타임이 정해진 시점에 그 메서드를 호출한다.

예를 들어 `impl HttpContext for TenantFilter` 안의 `on_http_request_headers`는 요청 헤더 처리 시점에 호출된다.

## 이 저장소에서 쓰는 컨텍스트

이 저장소는 주로 두 가지 `trait`를 사용한다.

- `RootContext`
  - 플러그인 초기화와 설정 로드 담당
  - 요청별 `HttpContext` 생성 담당
- `HttpContext`
  - 요청/응답 처리와 로그 기록 담당

현재 구현에서 `RootContext`는 `on_configure`, `get_type`, `create_http_context`를 통해 동작하고, 실제 필터 로직은 대부분 `HttpContext` 훅에서 실행된다.

## 개념 분류와 실제 메서드 매핑

| 개념 분류 | 계약 문서 이름 | 실제 Rust 메서드 | 현재 사용 여부 |
|------|------|------|------|
| Infrastructure Hooks | 별도 lifecycle 항목 없음 | `on_configure`, `get_type`, `create_http_context` | 사용 |
| Request Hooks | `on_request_headers` | `on_http_request_headers` | 사용 |
| Request Hooks | `on_request_body` | `on_http_request_body` | 미사용 |
| Response & Filter Hooks | `on_response_headers` | `on_http_response_headers` | 사용 |
| Response & Filter Hooks | `on_response_body` | `on_http_response_body` | 미사용 |
| Logging Hooks | `on_log` | `on_log` | 사용 |

주의:

- 계약 문서의 lifecycle 이름은 구현 의도를 설명하는 이름이다.
- 실제 코드는 `proxy-wasm`의 `trait` 메서드 이름으로 작성된다.
- 현재 저장소에는 `Response & Filter Hooks`라는 별도 런타임 계층이 있는 것이 아니라, response 훅 안에서 필터 동작을 수행한다.

## 현재 구현된 훅

### Infrastructure Hooks

- `tenant-filter`
  - `get_type`
  - `create_http_context`
- `auth-filter`
  - `on_configure`
  - `get_type`
  - `create_http_context`
- `header-filter`
  - `on_configure`
  - `get_type`
  - `create_http_context`
- `rate-limit-filter`
  - `on_configure`
  - `get_type`
  - `create_http_context`
- `observe-filter`
  - `get_type`
  - `create_http_context`

`on_configure`는 플러그인 설정을 읽는 초기화 훅이다.
현재 `auth-filter`는 secret 설정, `header-filter`는 revision/plugin chain 메타데이터, `rate-limit-filter`는 rate limit 설정을 이 단계에서 로드한다.

### Request Hooks

현재 모든 필터가 `on_http_request_headers`를 구현한다.

- `tenant-filter`
  - `x-tenant-id` 검증
  - 누락 시 401 응답 후 `Action::Pause`
- `auth-filter`
  - `x-api-key` 검증
  - 인증 정보 헤더 주입
- `header-filter`
  - `x-request-id`, `x-trace-id` 생성
  - revision, plugin chain 헤더 주입
- `rate-limit-filter`
  - tenant/service/route 기준 rate limit 검사
  - 초과 시 429 응답 후 `Action::Pause`
- `observe-filter`
  - 시작 시각 저장

### Response & Filter Hooks

현재 `on_http_response_headers`를 구현한 필터는 두 개다.

- `header-filter`
  - 응답에 `x-gateway-decision` 기본값 주입
- `observe-filter`
  - request 단계에서 쌓인 `x-gateway-decision`, `x-gateway-decision-reason`, `x-gateway-revision`을 응답 헤더로 복사

### Logging Hooks

현재 `on_log`를 구현한 필터는 `observe-filter` 하나다.

- `observe-filter`
  - 시작 시각과 종료 시각으로 latency 계산
  - decision, reason, tenant, route, revision과 함께 구조화된 로그 메시지 기록

## 현재 미구현 훅

현재 저장소에는 아래 훅 구현이 없다.

- `on_http_request_body`
- `on_http_request_trailers`
- `on_http_response_body`
- `on_http_response_trailers`

추가적인 인프라성 훅도 구현하지 않는다.

- `on_tick`
- `on_done`
- `on_delete`

즉 현재 구조는 요청 헤더 단계에서 대부분의 판단을 끝내고, 응답 헤더와 로그 단계에서 메타데이터를 마무리하는 형태다.

## revision 메타데이터와 실제 코드의 차이

revision 번들의 `plugins[].hooks`는 플러그인이 어떤 lifecycle 단계에 참여하는지 설명하는 메타데이터다.
하지만 현재 구현에서는 이 값이 코드의 모든 `trait` 메서드를 완전하게 반영하지는 않는다.

예시:

- `observe-filter`
  - `revision.json`에는 `["on_request_headers", "on_log"]`만 선언돼 있다.
  - 실제 코드는 `on_http_response_headers`도 구현한다.
- `auth-filter`, `header-filter`, `rate-limit-filter`
  - 실제 코드는 `on_configure`를 사용한다.
  - `revision.json`의 `hooks`에는 `on_configure`가 없다.

따라서 `plugins[].hooks`는 현재 기준으로 "요청/응답 lifecycle 메타데이터"에 가깝고, 실제 Rust 구현의 전체 훅 목록은 코드에서 직접 확인해야 한다.

## 요청 1건 기준 호출 흐름

요청 한 건이 들어오면 흐름은 대략 아래와 같다.

1. Nginx가 `proxy_wasm` 체인에 등록된 모듈을 순서대로 실행한다.
2. 각 플러그인의 `RootContext`가 필요 시 설정을 유지하고, 요청별 `HttpContext`를 만든다.
3. 요청 단계에서 `on_http_request_headers`가 순서대로 호출된다.
4. 어떤 필터가 `send_http_response` 후 `Action::Pause`를 반환하면 upstream 호출 없이 종료된다.
5. upstream 응답이 있으면 구현된 필터에 대해 `on_http_response_headers`가 호출된다.
6. 요청 종료 시 `on_log`가 호출돼 최종 관측 정보를 남긴다.

현재 이 저장소의 고정 체인은 `tenant-filter -> auth-filter -> header-filter -> rate-limit-filter -> observe-filter`다.

## 관련 문서

- [플러그인 런타임 계약](plugin-runtime-contract.md)
- [플러그인 카탈로그](plugin-catalog.md)
