# init_dev 정리 안내

기존 `init_dev.md`에 모여 있던 구현 지시를 주제별 문서로 분리했다.
이 파일은 하위 문서로 들어가는 호환성 진입점이다.

## 읽는 순서

1. [문서 루트](README.md)
2. [프로젝트 개요](01-overview/project-overview.md)
3. [데이터 플레인 아키텍처](02-architecture/dataplane-architecture.md)
4. [리비전 배포와 롤백](02-architecture/revision-lifecycle.md)
5. [플러그인 런타임 계약](03-runtime-contracts/plugin-runtime-contract.md)
6. [설정 리소스 모델](04-config-models/README.md)
7. [운영 관측성](05-operations/observability.md)

## 이동된 내용

- 구현 범위, 목표, 산출물, 핵심 원칙: `01-overview/`
- dataplane 책임과 revision 반영 구조: `02-architecture/`
- plugin 입력/출력, failure mode, 호환성: `03-runtime-contracts/`
- Gateway, Listener, Router, Service, Policy 리소스 스펙: `04-config-models/`
- observability, 배포 검증, rollback 절차: `05-operations/`
