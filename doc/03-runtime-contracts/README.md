# 런타임 계약

이 디렉토리는 Nginx와 Wasm plugin이 공유하는 실행 계약을 정의한다.
구현 시에는 이 문서를 리소스 스펙보다 우선 적용한다.

## 문서 목록

- [플러그인 런타임 계약](plugin-runtime-contract.md): 공통 헤더, 에러 포맷, lifecycle, failure mode, 호환성
- [플러그인 카탈로그](plugin-catalog.md): 기본 제공 플러그인 책임과 기본 실패 정책

## 구현 원칙

- 기존 헤더를 임의로 삭제하지 않는다.
- 정책 최종 결정은 단일 최종 상태로 기록한다.
- 세부 실패 사유는 구조화된 헤더 또는 로그 필드로 남긴다.
- plugin version compatibility는 revision 활성화 전 검증한다.
