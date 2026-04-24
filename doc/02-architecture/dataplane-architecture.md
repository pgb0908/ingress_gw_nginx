# 데이터 플레인 아키텍처

## 책임 경계

이 저장소는 ingress-gateway의 데이터 플레인 구현에 집중한다.
control plane은 별도 시스템으로 존재하며, 이 저장소는 control plane이 생성한 배포 산출물을 읽고 실행하는 책임만 가진다.

데이터 플레인이 담당하는 일:

- Listener, Router, Service, Policy 리소스 로드
- revision 디렉터리 선택과 현재 활성 revision 전환
- Nginx 설정 반영과 reload
- Wasm plugin chain 실행
- 요청 허용, 차단, 헤더 표준화, trace propagation
- rate limit 및 secret 같은 외부 의존 시스템 조회
- 로그, 메트릭, 트레이싱 기록
- 실패 시 fallback 및 rollback 실행

control plane이 담당한다고 가정하는 일:

- 설정 초안 생성
- 리소스 조합 검증
- revision 산출물 패키징
- 배포 승인 및 배포 시점 결정

## 요청 처리 흐름

```text
Client
  -> Listener
  -> Gateway 전역 정책
  -> Router 매칭
  -> Plugin Chain 실행
  -> Service 선택 및 upstream proxy
  -> Access Log / Metrics / Tracing 기록
```

정책 실행은 다음 원칙을 따른다.

- 보안 정책이 트래픽 정책보다 먼저 실행된다.
- 플러그인 실패 동작은 계약 문서의 failure mode를 따른다.
- 정책 결정 결과와 실패 원인은 요청 단위로 추적 가능해야 한다.

## 구현 우선순위

1. 정적 revision 로드와 Nginx 렌더링
2. reload 전 검증과 rollback
3. plugin chain 계약 구현
4. 관측성 기본 필드 완성
5. 외부 연동과 fallback 정책

## 관련 문서

- [프로젝트 개요](../01-overview/project-overview.md)
- [리비전 배포와 롤백](revision-lifecycle.md)
- [플러그인 런타임 계약](../03-runtime-contracts/plugin-runtime-contract.md)
- [설정 리소스 모델](../04-config-models/README.md)
