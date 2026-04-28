# 플러그인 런타임 계약

이 디렉토리는 ingress-gateway 데이터 플레인과 Wasm plugin 사이의 계약을 정의한다.
plugin 구현 또는 Nginx 연동 구현을 시작하기 전에 이 문서들을 기준 계약으로 삼는다.

## 문서 목록

| 문서 | 내용 |
|------|------|
| [plugin-runtime-contract.md](plugin-runtime-contract.md) | 공통 헤더 규칙, lifecycle 훅 정의, 에러 응답 형식, 실패 정책, 버전 호환성 요건 |
| [plugin-catalog.md](plugin-catalog.md) | 기본 데이터 플레인에서 제공해야 할 plugin 목록 (책임, 입출력, failure mode) |

## 계약의 범위

- **공통 헤더**: 모든 plugin이 읽거나 쓸 수 있는 헤더 이름과 사용 규칙
- **lifecycle 훅**: `on_request_headers`, `on_request_body`, `on_response_headers`, `on_response_body`, `on_log`
- **에러 응답 형식**: 차단 응답의 JSON 구조
- **실패 정책**: `fail-open`, `fail-close`, `fail-static-response` 동작 정의
- **버전 호환성**: revision 활성화 전 plugin version 검증 요건

## 관련 문서

- [데이터 플레인 아키텍처](../02-architecture/dataplane-architecture.md)
- [리비전 배포와 롤백](../02-architecture/revision-lifecycle.md)
- [운영 관측성](../05-operations/observability.md)
